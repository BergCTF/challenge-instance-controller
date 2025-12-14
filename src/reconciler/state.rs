use super::{update_status, Context};
use crate::{
    crds::{Challenge, ChallengeInstance, ChallengeInstanceClass, Condition, ConditionStatus, Phase},
    error::Result,
    resources, utils,
};
use kube::{runtime::controller::Action, ResourceExt};
use std::sync::Arc;
use std::time::Duration;
use tracing::info;

/// Pending → Creating transition
pub async fn reconcile_pending(
    instance: Arc<ChallengeInstance>,
    challenge: Challenge,
    _class: ChallengeInstanceClass,
    ctx: Arc<Context>,
) -> Result<Action> {
    info!("Validating flag for instance {}", instance.name_any());

    // Validate flag if required
    let requires_flag = challenge
        .spec
        .containers
        .iter()
        .any(|c| c.dynamic_flag.is_some());

    if requires_flag && instance.spec.flag.is_empty() {
        update_status(&instance, &ctx, |status| {
            status.phase = Some(Phase::Failed);
            status.conditions.push(Condition {
                r#type: "FlagValidation".to_string(),
                status: ConditionStatus::False,
                last_transition_time: Some(chrono::Utc::now().to_rfc3339()),
                reason: Some("FlagMissing".to_string()),
                message: Some("Flag required but not provided".to_string()),
            });
        })
        .await?;

        return Ok(Action::await_change());
    }

    // Transition to Creating
    update_status(&instance, &ctx, |status| {
        status.phase = Some(Phase::Creating);
        status.conditions.push(Condition {
            r#type: "FlagValidation".to_string(),
            status: ConditionStatus::True,
            last_transition_time: Some(chrono::Utc::now().to_rfc3339()),
            reason: Some("FlagValid".to_string()),
            message: Some("Flag validation passed".to_string()),
        });
    })
    .await?;

    Ok(Action::requeue(Duration::from_secs(1)))
}

/// Creating phase - create all resources
pub async fn reconcile_creating(
    instance: Arc<ChallengeInstance>,
    challenge: Challenge,
    class: ChallengeInstanceClass,
    ctx: Arc<Context>,
) -> Result<Action> {
    info!("Creating resources for instance {}", instance.name_any());

    // Generate unique namespace per owner
    let namespace_name = utils::generate_namespace_name(&instance.spec.owner_id);

    // 1. Create namespace
    resources::namespace::create(&instance, &namespace_name, &ctx).await?;

    // 2. Copy image pull secret if configured
    if let Some(ref image_pull) = class.spec.image_pull {
        if let Some(ref secret_name) = image_pull.secret_name {
            resources::namespace::copy_pull_secret(&ctx.client, secret_name, &namespace_name).await?;
        }
    }

    // 3. Create network policy
    resources::network_policy::create(&instance, &challenge, &namespace_name, &class, &ctx).await?;

    // 4. For each container, create resources
    for container in &challenge.spec.containers {
        // Services
        resources::service::create(&instance, &challenge, container, &namespace_name, &ctx)
            .await?;

        // Gateway API routes
        resources::gateway::create_http_routes(&instance, container, &namespace_name, &class, &ctx).await?;
        resources::gateway::create_tls_routes(&instance, container, &namespace_name, &class, &ctx).await?;

        // ConfigMaps for flags
        if let Some(ref dynamic_flag) = container.dynamic_flag {
            resources::configmap::create_flag_configmap(
                &instance,
                container,
                dynamic_flag,
                &namespace_name,
                &ctx,
            )
            .await?;
        }

        // PodDisruptionBudget
        resources::pdb::create(&instance, container, &namespace_name, &ctx).await?;

        // Deployment
        resources::deployment::create(&instance, &challenge, container, &namespace_name, &class, &ctx)
            .await?;
    }

    // Update status
    let now = chrono::Utc::now().to_rfc3339();
    update_status(&instance, &ctx, |status| {
        status.namespace = Some(namespace_name.clone());
        status.phase = Some(Phase::Starting);
        status.conditions.extend([
            Condition {
                r#type: "NamespaceCreated".to_string(),
                status: ConditionStatus::True,
                last_transition_time: Some(now.clone()),
                reason: Some("Created".to_string()),
                message: Some(format!("Namespace {} created", namespace_name)),
            },
            Condition {
                r#type: "ResourcesCreated".to_string(),
                status: ConditionStatus::True,
                last_transition_time: Some(now),
                reason: Some("Created".to_string()),
                message: Some("All resources created".to_string()),
            },
        ]);
    })
    .await?;

    Ok(Action::requeue(Duration::from_secs(2)))
}

/// Starting phase - wait for pods to be ready
pub async fn reconcile_starting(
    instance: Arc<ChallengeInstance>,
    challenge: Challenge,
    class: ChallengeInstanceClass,
    ctx: Arc<Context>,
) -> Result<Action> {
    let namespace = instance
        .status
        .as_ref()
        .and_then(|s| s.namespace.as_ref())
        .expect("Namespace should be set in Starting phase");

    // Check pod readiness
    let all_ready = resources::deployment::check_pods_ready(&ctx.client, namespace).await?;

    if all_ready {
        info!(
            "All pods ready for instance {}, transitioning to Running",
            instance.name_any()
        );

        // Discover service endpoints
        let endpoints =
            resources::service::discover_endpoints(&instance, &challenge, namespace, &class, &ctx).await?;

        let now = chrono::Utc::now().to_rfc3339();
        update_status(&instance, &ctx, |status| {
            status.phase = Some(Phase::Running);
            status.ready_at = Some(now.clone());
            status.services = endpoints;
            status.conditions.push(Condition {
                r#type: "PodsReady".to_string(),
                status: ConditionStatus::True,
                last_transition_time: Some(now),
                reason: Some("AllReady".to_string()),
                message: Some("All pods are ready".to_string()),
            });
        })
        .await?;

        // Requeue at expiration time
        let expires_at_str = instance
            .status
            .as_ref()
            .and_then(|s| s.expires_at.as_ref())
            .expect("expiresAt should be set");
        let duration = if let Ok(expires_at) = chrono::DateTime::parse_from_rfc3339(expires_at_str) {
            (expires_at.with_timezone(&chrono::Utc) - chrono::Utc::now())
                .to_std()
                .unwrap_or(Duration::from_secs(3600))
        } else {
            Duration::from_secs(3600)
        };

        Ok(Action::requeue(duration))
    } else {
        info!(
            "Waiting for pods to become ready for instance {}",
            instance.name_any()
        );

        let now = chrono::Utc::now().to_rfc3339();
        update_status(&instance, &ctx, |status| {
            // Update or add PodsReady condition as Unknown
            if let Some(cond) = status
                .conditions
                .iter_mut()
                .find(|c| c.r#type == "PodsReady")
            {
                cond.status = ConditionStatus::Unknown;
                cond.last_transition_time = Some(now.clone());
            } else {
                status.conditions.push(Condition {
                    r#type: "PodsReady".to_string(),
                    status: ConditionStatus::Unknown,
                    last_transition_time: Some(now),
                    reason: Some("WaitingForPods".to_string()),
                    message: Some("Waiting for pods to be ready".to_string()),
                });
            }
        })
        .await?;

        Ok(Action::requeue(Duration::from_secs(5)))
    }
}

/// Running phase - monitor health
pub async fn reconcile_running(
    instance: Arc<ChallengeInstance>,
    _challenge: Challenge,
    _class: ChallengeInstanceClass,
    ctx: Arc<Context>,
) -> Result<Action> {
    // Check if expired (should be caught earlier, but double-check)
    if super::timeout::is_expired(&instance) {
        return super::timeout::terminate_expired(instance, ctx).await;
    }

    // Requeue at expiration
    let expires_at_str = instance
        .status
        .as_ref()
        .and_then(|s| s.expires_at.as_ref())
        .expect("expiresAt should be set");
    let duration = if let Ok(expires_at) = chrono::DateTime::parse_from_rfc3339(expires_at_str) {
        (expires_at.with_timezone(&chrono::Utc) - chrono::Utc::now())
            .to_std()
            .unwrap_or(Duration::from_secs(60))
    } else {
        Duration::from_secs(60)
    };

    Ok(Action::requeue(duration))
}

/// Terminating phase
pub async fn reconcile_terminating(
    instance: Arc<ChallengeInstance>,
    ctx: Arc<Context>,
) -> Result<Action> {
    // This is handled by the finalizer
    super::finalizer::cleanup(instance, ctx).await
}
