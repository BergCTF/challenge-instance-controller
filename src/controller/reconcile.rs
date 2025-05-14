use std::{sync::Arc, time::Duration};

use k8s_openapi::api::core::v1::Namespace;
use kube::{
    api::{ObjectMeta, PatchParams},
    runtime::controller::Action,
    Resource, ResourceExt,
};
use tracing::info;

use crate::crd::ChallengeInstance;

use super::ReconcilerContext;

// main reconciliation event hook
pub async fn reconcile(
    obj: Arc<ChallengeInstance>,
    ctx: Arc<ReconcilerContext>,
) -> super::error::Result<Action> {
    let config = super::config::config();
    info!("reconcile request: {}", obj.name_any());

    // create a namespace for the challenge instance
    if !config.same_namespace {
        let namespace = generate_namespace(&obj);
        let patch_params = PatchParams::apply("challenge-instance-controller");
        let namespace_patch = serde_json::to_value(&namespace).map_err(|e| super::error::Error::SerializationError(e))?;
        let patch = kube::api::Patch::Apply(&namespace_patch);
        let _namespace = ctx
            .apis
            .namespaces
            .patch(&namespace.name_any(), &patch_params, &patch)
            .await.map_err(|e| super::error::Error::KubeError(e))?;
    }

    Ok(Action::requeue(Duration::from_secs(3600)))
}

pub fn generate_namespace(source: &ChallengeInstance) -> Namespace {
    let oref = source.controller_owner_ref(&()).unwrap();
    Namespace {
        metadata: ObjectMeta {
            name: source.metadata.name.clone(),
            owner_references: Some(vec![oref]),
            ..ObjectMeta::default()
        },
        ..Default::default()
    }
}
