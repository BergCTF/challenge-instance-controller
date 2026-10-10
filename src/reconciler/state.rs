use super::{update_status, Context};
use crate::{
    crds::{
        Challenge, ChallengeInstance, ChallengeInstanceClass, Condition, ConditionStatus, Phase,
    },
    date_time::DateTime,
    error::{Error, Result},
    resources, utils,
};
use kube::{runtime::controller::Action, ResourceExt};
use std::sync::Arc;
use std::time::Duration;
use tracing::{debug, info};

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
                last_transition_time: Some(DateTime::now()),
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
            last_transition_time: Some(DateTime::now()),
            reason: Some("FlagValid".to_string()),
            message: Some("Flag validation passed".to_string()),
        });
    })
    .await?;

    Ok(Action::requeue(Duration::from_secs(1)))
}

pub async fn reconcile_creating(
    instance: Arc<ChallengeInstance>,
    challenge: Challenge,
    class: ChallengeInstanceClass,
    ctx: Arc<Context>,
) -> Result<Action> {
    info!("Creating resources for instance {}", instance.name_any());

    let namespace_name = utils::generate_namespace_name(
        &ctx.config.namespace_prefix,
        challenge.metadata.name.as_ref().unwrap(),
        &instance.spec.owner_id,
    );

    resources::namespace::reconcile(&instance, &namespace_name, &ctx).await?;
    resources::network_policy::reconcile(&challenge, &namespace_name, &class, &ctx).await?;

    if let Some(ref image_pull) = class.spec.image_pull {
        for secret in &image_pull.secret_names {
            resources::namespace::copy_pull_secret(&ctx.client, secret, &namespace_name).await?;
        }
    }

    let mut endpoints = Vec::new();

    for container in &challenge.spec.containers {
        // Services
        endpoints.extend(
            resources::service::reconcile(&class, &challenge, container, &namespace_name, &ctx)
                .await?,
        );

        // Gateway API routes
        endpoints.extend(
            resources::gateway::create_http_routes(
                &instance,
                container,
                &namespace_name,
                &class,
                &ctx,
            )
            .await?,
        );
        endpoints.extend(
            resources::gateway::create_tls_routes(
                &instance,
                container,
                &namespace_name,
                &class,
                &ctx,
            )
            .await?,
        );

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
        resources::pdb::reconcile(container, &namespace_name, &ctx).await?;
    }

    // first create all services, then create deployments
    // we inject service metadata information into deployments
    // so we'll wanna wait until they're ready (eg nodeport services)
    for container in &challenge.spec.containers {
        match resources::deployment::reconcile(
            &instance,
            &challenge,
            container,
            &namespace_name,
            &class,
            &endpoints,
            &ctx,
        )
        .await
        {
            Ok(_) => debug!("reconciled deployment"),
            Err(Error::ProgressingWait) => {
                debug!("deferring deployment while we wait for dependencies to become available");
                return Ok(Action::requeue(Duration::from_secs(2)));
            }
            Err(err) => return Err(err),
        };
    }

    debug!("Collected {} endpoints", endpoints.len());
    // Update status
    let now = chrono::Utc::now();
    update_status(&instance, &ctx, |status| {
        status.namespace = Some(namespace_name.clone());
        // move on to next phase
        status.phase = Some(Phase::Starting);
        status.conditions.extend([
            Condition {
                r#type: "NamespaceCreated".to_string(),
                status: ConditionStatus::True,
                last_transition_time: Some(DateTime::from(now)),
                reason: Some("Created".to_string()),
                message: Some(format!("Namespace {} created", namespace_name)),
            },
            Condition {
                r#type: "ResourcesCreated".to_string(),
                status: ConditionStatus::True,
                last_transition_time: Some(DateTime::from(now)),
                reason: Some("Created".to_string()),
                message: Some("All resources created".to_string()),
            },
        ]);
        status.services = endpoints;
    })
    .await?;

    Ok(Action::requeue(Duration::from_secs(2)))
}

/// Starting phase - wait for pods to be ready
pub async fn reconcile_starting(
    instance: Arc<ChallengeInstance>,
    _challenge: Challenge,
    _class: ChallengeInstanceClass,
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

        let now = chrono::Utc::now();
        update_status(&instance, &ctx, |status| {
            status.phase = Some(Phase::Running);
            status.ready_at = Some(DateTime(now));
            status.conditions.push(Condition {
                r#type: "PodsReady".to_string(),
                status: ConditionStatus::True,
                last_transition_time: Some(DateTime(now)),
                reason: Some("AllReady".to_string()),
                message: Some("All pods are ready".to_string()),
            });
        })
        .await?;

        // Requeue at expiration time
        let expires_at_dt = instance
            .status
            .as_ref()
            .and_then(|s| s.expires_at.as_ref())
            .expect("expiresAt should be set");
        let duration = (expires_at_dt.0 - chrono::Utc::now())
            .to_std()
            .unwrap_or(Duration::from_secs(3600));

        Ok(Action::requeue(duration))
    } else {
        debug!(
            "Waiting for pods to become ready for instance {}",
            instance.name_any()
        );

        let now = chrono::Utc::now();
        if !instance
            .status
            .as_ref()
            .map(|status| {
                status
                    .conditions
                    .iter()
                    .any(|c| c.r#type == "PodsReady" && c.status == ConditionStatus::Unknown)
            })
            .is_some_and(|f| f)
        {
            update_status(&instance, &ctx, |status| {
                // Update or add PodsReady condition as Unknown
                if let Some(cond) = status
                    .conditions
                    .iter_mut()
                    .find(|c| c.r#type == "PodsReady")
                {
                    cond.status = ConditionStatus::Unknown;
                    cond.last_transition_time = Some(DateTime(now));
                } else {
                    status.conditions.push(Condition {
                        r#type: "PodsReady".to_string(),
                        status: ConditionStatus::Unknown,
                        last_transition_time: Some(DateTime(now)),
                        reason: Some("WaitingForPods".to_string()),
                        message: Some("Waiting for pods to be ready".to_string()),
                    });
                }
            })
            .await?;
        }

        Ok(Action::requeue(Duration::from_secs(5)))
    }
}

/// Running phase - monitor workload health until the instance expires.
pub async fn reconcile_running(
    instance: Arc<ChallengeInstance>,
    _challenge: Challenge,
    _class: ChallengeInstanceClass,
    ctx: Arc<Context>,
) -> Result<Action> {
    let namespace = instance
        .status
        .as_ref()
        .and_then(|s| s.namespace.clone())
        .unwrap_or_default();

    if !namespace.is_empty() {
        let pods = resources::deployment::list_managed_pods(&ctx.client, &namespace).await?;
        let all_ready = !pods.is_empty() && pods.iter().all(resources::deployment::is_pod_ready);

        if all_ready {
            debug!(
                "Workload healthy for instance {} in {}",
                instance.name_any(),
                namespace
            );
        } else if pods.is_empty() {
            // The workload (or its namespace) is no longer present
            super::record_terminal_failure(
                &instance,
                &ctx,
                "WorkloadLost",
                &format!("Workload no longer present in namespace {namespace}"),
            )
            .await?;
            return Ok(Action::await_change());
        } else {
            // Pods exist but are not ready: surface degraded state and retry soon
            let now = chrono::Utc::now();
            update_status(&instance, &ctx, |status| {
                if let Some(cond) = status
                    .conditions
                    .iter_mut()
                    .find(|c| c.r#type == "PodsReady")
                {
                    cond.status = ConditionStatus::False;
                    cond.last_transition_time = Some(DateTime::from(now));
                    cond.reason = Some("NotReady".to_string());
                    cond.message = Some("Pods not ready while running".to_string());
                } else {
                    status.conditions.push(Condition {
                        r#type: "PodsReady".to_string(),
                        status: ConditionStatus::False,
                        last_transition_time: Some(DateTime::from(now)),
                        reason: Some("NotReady".to_string()),
                        message: Some("Pods not ready while running".to_string()),
                    });
                }
            })
            .await?;
            return Ok(Action::requeue(Duration::from_secs(10)));
        }
    }

    // Healthy (or nothing to monitor): schedule the next wakeup at expiration
    let expires_at_dt = instance
        .status
        .as_ref()
        .and_then(|s| s.expires_at.as_ref())
        .expect("expiresAt should be set");
    let duration = (expires_at_dt.0 - chrono::Utc::now())
        .to_std()
        .unwrap_or(Duration::from_secs(600))
        // requeue every 10 minutes just in case
        .min(Duration::from_secs(600));

    Ok(Action::requeue(duration))
}
