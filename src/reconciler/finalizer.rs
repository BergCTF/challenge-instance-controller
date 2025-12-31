use super::{update_status, Context, FINALIZER};
use crate::{
    crds::{ChallengeInstance, Condition, ConditionStatus, DateTime, Phase},
    error::Result,
    utils,
};
use k8s_openapi::api::core::v1::Namespace;
use kube::{
    api::{Api, DeleteParams, Patch, PatchParams},
    runtime::controller::Action,
    ResourceExt,
};
use std::sync::Arc;
use tracing::info;

pub async fn cleanup(instance: Arc<ChallengeInstance>, ctx: Arc<Context>) -> Result<Action> {
    info!("Cleaning up ChallengeInstance {}", instance.name_any());

    let namespace_name = if let Some(ref status) = instance.status {
        if let Some(ref ns) = status.namespace {
            ns.clone()
        } else {
            utils::generate_namespace_name(
                &ctx.config.namespace_prefix,
                &instance.spec.challenge_ref.name,
                &instance.spec.owner_id,
            )
        }
    } else {
        utils::generate_namespace_name(
            &ctx.config.namespace_prefix,
            &instance.spec.challenge_ref.name,
            &instance.spec.owner_id,
        )
    };

    // Delete namespace (cascades to all resources)
    let namespaces: Api<Namespace> = Api::all(ctx.client.clone());

    match namespaces.get(&namespace_name).await {
        Ok(_) => {
            info!("Deleting namespace {}", namespace_name);
            namespaces
                .delete(&namespace_name, &DeleteParams::default())
                .await?;
        }
        Err(kube::Error::Api(ae)) if ae.code == 404 => {
            info!("Namespace {} already deleted", namespace_name);
        }
        Err(e) => return Err(e.into()),
    }

    // Update status to Terminated
    let now = chrono::Utc::now().to_rfc3339();
    update_status(&instance, &ctx, |status| {
        status.phase = Some(Phase::Terminated);
        status.terminated_at = Some(DateTime(now.clone()));
        status.conditions.push(Condition {
            r#type: "NamespaceDeleted".to_string(),
            status: ConditionStatus::True,
            last_transition_time: Some(DateTime(now)),
            reason: Some("Deleted".to_string()),
            message: Some("Namespace deleted".to_string()),
        });
    })
    .await?;

    // Remove finalizer
    remove_finalizer(&instance, &ctx).await?;

    ctx.metrics.decr_active_instances();
    Ok(Action::await_change())
}

async fn remove_finalizer(instance: &ChallengeInstance, ctx: &Context) -> Result<()> {
    let api: Api<ChallengeInstance> = Api::all(ctx.client.clone());

    let mut finalizers = instance.metadata.finalizers.clone().unwrap_or_default();
    finalizers.retain(|f| f != FINALIZER);

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

    Ok(())
}
