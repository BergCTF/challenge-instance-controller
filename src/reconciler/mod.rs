use crate::{
    config::ControllerConfig,
    crds::{Challenge, ChallengeInstance, ChallengeInstanceClass, ChallengeInstanceStatus, Phase},
    date_time::DateTime,
    error::{Error, Result},
    telemetry::Metrics,
};
use kube::{
    api::{Api, Patch, PatchParams},
    client::Client,
    runtime::controller::Action,
    Resource, ResourceExt,
};
use std::sync::Arc;
use std::time::Duration;
use tracing::{error, info, instrument};

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

    info!("Reconciling ChallengeInstance {}", name);
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

    // Check timeout expiration
    if timeout::is_expired(&instance) {
        return timeout::terminate_expired(instance, ctx).await;
    }

    // Fetch referenced Challenge and ChallengeInstanceClass
    let challenge = fetch_challenge(&instance, &ctx).await?;
    let class = fetch_instance_class(&instance, &ctx).await?;

    // Reconcile based on phase
    let phase = instance
        .status
        .as_ref()
        .and_then(|s| s.phase.as_ref())
        .unwrap_or(&Phase::Pending);

    match phase {
        Phase::Pending => state::reconcile_pending(instance, challenge, class, ctx).await,
        Phase::Creating => state::reconcile_creating(instance, challenge, class, ctx).await,
        Phase::Starting => state::reconcile_starting(instance, challenge, class, ctx).await,
        Phase::Running => state::reconcile_running(instance, challenge, class, ctx).await,
        Phase::Terminating => state::reconcile_terminating(instance, ctx).await,
        Phase::Terminated | Phase::Failed => {
            // No action needed
            Ok(Action::await_change())
        }
    }
}

async fn fetch_challenge(instance: &ChallengeInstance, ctx: &Context) -> Result<Challenge> {
    let challenge_ns = instance
        .spec
        .challenge_ref
        .namespace
        .as_deref()
        .unwrap_or(&ctx.config.challenge_namespace);

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
    let classes: Api<ChallengeInstanceClass> = Api::all(ctx.client.clone());

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
    let api: Api<ChallengeInstance> = Api::all(ctx.client.clone());

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
    let expires_at = timeout::calculate_expiry(
        instance
            .spec
            .timeout
            .as_ref()
            .unwrap_or(&ctx.config.default_timeout),
    )?;

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

/// Helper to update status
pub async fn update_status<F>(instance: &ChallengeInstance, ctx: &Context, mutate: F) -> Result<()>
where
    F: FnOnce(&mut ChallengeInstanceStatus),
{
    let api: Api<ChallengeInstance> = Api::all(ctx.client.clone());

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
    error!("[*] Reconciliation error: {:?}", error);
    ctx.metrics.record_error();

    if error.is_retryable() {
        Action::requeue(Duration::from_secs(10))
    } else {
        Action::requeue(Duration::from_secs(300))
    }
}
