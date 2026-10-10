use crate::{
    config::ControllerConfig,
    crds::{
        Challenge, ChallengeInstance, ChallengeInstanceClass, ChallengeInstanceStatus, Condition,
        ConditionStatus, Phase, TerminationReason,
    },
    date_time::DateTime,
    error::{Error, Result},
    telemetry::Metrics,
};
use kube::{
    api::{Api, DeleteParams, Patch, PatchParams},
    client::Client,
    runtime::controller::Action,
    Resource, ResourceExt,
};
use std::sync::Arc;
use std::time::Duration;
use tracing::{debug, info, instrument, warn};

pub mod finalizer;
pub mod state;
pub mod timeout;

pub const FINALIZER: &str = "challengeinstance.berg.norelect.ch/finalizer";

#[derive(Clone)]
pub struct Context {
    pub client: Client,
    pub config: Arc<ControllerConfig>,
    pub metrics: Arc<Metrics>,
}

#[instrument(skip(ctx, instance), fields(instance_name = %instance.name_any()))]
pub async fn reconcile(instance: Arc<ChallengeInstance>, ctx: Arc<Context>) -> Result<Action> {
    let name = instance.name_any();

    debug!("Reconciling ChallengeInstance {}", name);
    ctx.metrics.record_reconcile();

    // Handle deletion
    if instance.meta().deletion_timestamp.is_some() {
        return finalizer::cleanup(instance, ctx).await;
    }

    // Ensure finalizer
    if !instance
        .meta()
        .finalizers
        .as_ref()
        .map(|f| f.contains(&FINALIZER.to_string()))
        .unwrap_or(false)
    {
        return add_finalizer(instance, ctx).await;
    }

    // Get or create instance ID
    if instance
        .status
        .as_ref()
        .and_then(|s| s.instance_id.as_ref())
        .is_none()
    {
        return initialize_instance(instance, ctx).await;
    }

    // Current phase as stored in status (defaults to Pending when unset)
    let phase = instance.status.as_ref().and_then(|s| s.phase.as_ref());

    // Terminate when the instance is expired, a termination reason was set,
    // or it is already in the Terminating phase
    if timeout::should_terminate(&instance) || matches!(phase, Some(Phase::Terminating)) {
        return initiate_termination(instance, ctx).await;
    }

    // Terminal states require no further action
    if matches!(phase, Some(Phase::Terminated) | Some(Phase::Failed)) {
        return Ok(Action::await_change());
    }

    // Fetch referenced Challenge and ChallengeInstanceClass
    let challenge = match fetch_challenge(&instance, &ctx).await {
        Ok(challenge) => challenge,
        Err(Error::ChallengeNotFound { namespace, name }) => {
            record_terminal_failure(
                &instance,
                &ctx,
                "ChallengeMissing",
                &format!("Challenge {namespace}/{name} not found"),
            )
            .await?;
            return Err(Error::ChallengeNotFound { namespace, name });
        }
        Err(e) => return Err(e),
    };

    let class = match fetch_instance_class(&instance, &ctx).await {
        Ok(class) => class,
        Err(Error::InstanceClassNotFound { name }) => {
            record_terminal_failure(
                &instance,
                &ctx,
                "InstanceClassMissing",
                &format!("ChallengeInstanceClass {name} not found"),
            )
            .await?;
            return Err(Error::InstanceClassNotFound { name });
        }
        Err(e) => return Err(e),
    };

    // Reconcile based on phase
    match phase {
        None | Some(Phase::Pending) => {
            state::reconcile_pending(instance, challenge, class, ctx).await
        }
        Some(Phase::Creating) => state::reconcile_creating(instance, challenge, class, ctx).await,
        Some(Phase::Starting) => state::reconcile_starting(instance, challenge, class, ctx).await,
        Some(Phase::Running) => state::reconcile_running(instance, challenge, class, ctx).await,
        // Terminating / Terminated / Failed are handled above
        _ => Ok(Action::await_change()),
    }
}

async fn fetch_challenge(instance: &ChallengeInstance, ctx: &Context) -> Result<Challenge> {
    let challenge_ns = instance
        .spec
        .challenge_ref
        .namespace
        .as_deref()
        .unwrap_or(ctx.client.default_namespace());

    let challenges: Api<Challenge> = Api::namespaced(ctx.client.clone(), challenge_ns);

    challenges
        .get(&instance.spec.challenge_ref.name)
        .await
        .map_err(|e| match e {
            kube::Error::Api(ae) if ae.code == 404 => Error::ChallengeNotFound {
                namespace: challenge_ns.to_string(),
                name: instance.spec.challenge_ref.name.clone(),
            },
            e => Error::from(e),
        })
}

async fn fetch_instance_class(
    instance: &ChallengeInstance,
    ctx: &Context,
) -> Result<ChallengeInstanceClass> {
    let classes: Api<ChallengeInstanceClass> =
        Api::namespaced(ctx.client.clone(), ctx.client.default_namespace());

    // Use specified class or default
    let class_name = instance
        .spec
        .instance_class
        .as_deref()
        .unwrap_or(&ctx.config.default_instance_class);

    classes.get(class_name).await.map_err(|e| match e {
        kube::Error::Api(ae) if ae.code == 404 => Error::InstanceClassNotFound {
            name: class_name.to_string(),
        },
        e => Error::from(e),
    })
}

async fn add_finalizer(instance: Arc<ChallengeInstance>, ctx: Arc<Context>) -> Result<Action> {
    let api: Api<ChallengeInstance> =
        Api::namespaced(ctx.client.clone(), ctx.client.default_namespace());

    let mut finalizers = instance.meta().finalizers.clone().unwrap_or_default();
    finalizers.push(FINALIZER.to_string());

    let patch = serde_json::json!({
        "metadata": {
            "finalizers": finalizers
        }
    });

    api.patch(
        &instance.name_any(),
        &PatchParams::default(),
        &Patch::Merge(&patch),
    )
    .await?;

    Ok(Action::requeue(Duration::from_secs(1)))
}

async fn initialize_instance(
    instance: Arc<ChallengeInstance>,
    ctx: Arc<Context>,
) -> Result<Action> {
    let instance_id = uuid::Uuid::new_v4().to_string();
    let expires_at = match timeout::calculate_expiry(
        instance
            .spec
            .timeout
            .as_ref()
            .unwrap_or(&ctx.config.default_timeout),
    ) {
        Ok(expires_at) => expires_at,
        Err(e) => {
            record_terminal_failure(
                &instance,
                &ctx,
                "InvalidTimeout",
                &format!("Could not parse timeout: {e}"),
            )
            .await?;
            return Err(e);
        }
    };

    update_status(&instance, &ctx, |status| {
        status.instance_id = Some(instance_id);
        status.phase = Some(Phase::Pending);
        status.started_at = Some(DateTime::now());
        status.expires_at = Some(DateTime::from(expires_at));
    })
    .await?;

    ctx.metrics.incr_active_instances();
    Ok(Action::requeue(Duration::from_secs(1)))
}

/// Transition the instance to the `Terminating` phase and delete it so the
/// finalizer performs cleanup. Idempotent: when the instance is already in a
/// terminating/terminal phase the status update is skipped and only the
/// deletion completion is ensured.
pub async fn initiate_termination(
    instance: Arc<ChallengeInstance>,
    ctx: Arc<Context>,
) -> Result<Action> {
    let name = instance.name_any();
    let reason = termination_reason(&instance);
    info!(
        "Initiating termination for instance {} ({:?})",
        name, reason
    );

    let current_phase = instance
        .status
        .as_ref()
        .and_then(|s| s.phase.as_ref())
        .cloned();

    if !matches!(
        current_phase,
        Some(Phase::Terminating) | Some(Phase::Terminated) | Some(Phase::Failed)
    ) {
        let message = match reason {
            TerminationReason::Timeout => "Instance has expired".to_string(),
            TerminationReason::UserRequest => "Termination requested by user".to_string(),
            TerminationReason::AdminTermination => {
                "Termination requested by administrator".to_string()
            }
        };
        if reason == TerminationReason::Timeout {
            ctx.metrics.record_timeout();
        }
        update_status(&instance, &ctx, |status| {
            status.phase = Some(Phase::Terminating);
            status.conditions.push(Condition {
                r#type: "Terminating".to_string(),
                status: ConditionStatus::True,
                last_transition_time: Some(DateTime::now()),
                reason: Some(format!("{reason:?}")),
                message: Some(message),
            });
        })
        .await?;
    }

    // Ensure the instance is deleted so the finalizer runs
    if instance.meta().deletion_timestamp.is_none() {
        let api: Api<ChallengeInstance> =
            Api::namespaced(ctx.client.clone(), ctx.client.default_namespace());
        api.delete(&name, &DeleteParams::default()).await?;
        info!("Deleted instance {}", name);
    }

    Ok(Action::await_change())
}

/// Derive the reason an instance is being terminated.
fn termination_reason(instance: &ChallengeInstance) -> TerminationReason {
    if timeout::is_expired(instance) {
        TerminationReason::Timeout
    } else if let Some(reason) = &instance.spec.termination_reason {
        reason.clone()
    } else {
        TerminationReason::UserRequest
    }
}

/// Record a terminal failure on the instance: set phase to `Failed` and add a
/// condition. No-op when the instance is already in a terminal phase.
pub async fn record_terminal_failure(
    instance: &ChallengeInstance,
    ctx: &Context,
    reason: &str,
    message: &str,
) -> Result<()> {
    let current_phase = instance
        .status
        .as_ref()
        .and_then(|s| s.phase.as_ref())
        .cloned();

    if matches!(
        current_phase,
        Some(Phase::Terminating) | Some(Phase::Terminated) | Some(Phase::Failed)
    ) {
        return Ok(());
    }

    update_status(instance, ctx, |status| {
        status.phase = Some(Phase::Failed);
        status.conditions.push(Condition {
            r#type: "Failed".to_string(),
            status: ConditionStatus::False,
            last_transition_time: Some(DateTime::now()),
            reason: Some(reason.to_string()),
            message: Some(message.to_string()),
        });
    })
    .await
}

/// Helper to update status
pub async fn update_status<F>(instance: &ChallengeInstance, ctx: &Context, mutate: F) -> Result<()>
where
    F: FnOnce(&mut ChallengeInstanceStatus),
{
    let api: Api<ChallengeInstance> =
        Api::namespaced(ctx.client.clone(), ctx.client.default_namespace());

    let mut status = instance.status.clone().unwrap_or_default();
    mutate(&mut status);
    status.observed_generation = instance.meta().generation;

    let patch = serde_json::json!({
        "status": status
    });

    api.patch_status(
        &instance.name_any(),
        &PatchParams::default(),
        &Patch::Merge(&patch),
    )
    .await?;

    Ok(())
}

/// Error handling for reconciliation
pub fn error_policy(_instance: Arc<ChallengeInstance>, error: &Error, ctx: Arc<Context>) -> Action {
    warn!("[*] Reconciliation error: {:?}", error);
    ctx.metrics.record_error();

    if error.is_retryable() {
        Action::requeue(Duration::from_secs(10))
    } else {
        Action::requeue(Duration::from_secs(300))
    }
}
