use super::{update_status, Context, FINALIZER};
use crate::{
    crds::{ChallengeInstance, Condition, ConditionStatus, Phase},
    date_time::DateTime,
    error::Result,
    utils,
};
use k8s_openapi::api::{
    apps::v1::Deployment,
    core::v1::{Namespace, Pod},
};
use kube::{
    api::{Api, DeleteParams, ListParams, Patch, PatchParams},
    runtime::controller::Action,
    ResourceExt,
};
use std::{sync::Arc, time::Duration};
use tracing::{debug, info};

pub async fn cleanup(instance: Arc<ChallengeInstance>, ctx: Arc<Context>) -> Result<Action> {
    debug!("Cleaning up ChallengeInstance {}", instance.name_any());

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

    // Clean up workloads before cleaning up NetworkPolicies
    let deployments_api: Api<Deployment> = Api::namespaced(ctx.client.clone(), &namespace_name);
    let pods_api: Api<Pod> = Api::namespaced(ctx.client.clone(), &namespace_name);
    let mut deleting = false;
    for deploy in deployments_api.list(&ListParams::default()).await? {
        deleting = true;
        let selectors = deploy.spec.unwrap().selector;
        if pods_api
            .list(&ListParams::default().labels_from(&selectors.try_into().unwrap()))
            .await?
            .iter()
            .any(|p| p.status.to_owned().unwrap().phase.unwrap() != "Terminating")
        {
            deployments_api
                .delete(&deploy.metadata.name.unwrap(), &DeleteParams::background())
                .await?;
        }
    }

    if deleting {
        // wait for resources to be deleted before continuing
        return Ok(Action::requeue(Duration::from_secs(2)));
    }

    // Delete namespace (cascades to all resources)
    let namespaces: Api<Namespace> = Api::all(ctx.client.clone());

    match namespaces.get(&namespace_name).await {
        Ok(namespace) => {
            if !namespace
                .status
                .and_then(|s| s.phase)
                .map(|phase| phase == "Terminating")
                .unwrap_or_default()
            {
                namespaces
                    .delete(&namespace_name, &DeleteParams::default())
                    .await?;
                info!("Deleted namespace {}", namespace_name);
                return Ok(Action::requeue(Duration::from_secs(2)));
            } else {
                debug!("Namespace {} already terminating", namespace_name);
                return Ok(Action::requeue(Duration::from_secs(2)));
            }
        }
        Err(kube::Error::Api(ae)) if ae.code == 404 => {
            debug!("Namespace {} finished deleting", namespace_name);
        }
        Err(e) => return Err(e.into()),
    }

    // Update status to Terminated
    let now = DateTime::now();
    update_status(&instance, &ctx, |status| {
        status.phase = Some(Phase::Terminated);
        status.terminated_at = Some(now.clone());
        status.conditions.push(Condition {
            r#type: "NamespaceDeleted".to_string(),
            status: ConditionStatus::True,
            last_transition_time: Some(now),
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
