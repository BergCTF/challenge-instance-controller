use crate::{
    crds::{ChallengeInstance, ContainerSpec},
    error::Result,
    reconciler::Context,
};
use k8s_openapi::{
    api::policy::v1::{PodDisruptionBudget, PodDisruptionBudgetSpec},
    apimachinery::pkg::{apis::meta::v1::LabelSelector, util::intstr::IntOrString},
};
use kube::{
    api::{Api, PostParams},
    Resource,
};
use std::collections::BTreeMap;
use tracing::info;

/// reconcile attempts to create pdbs for the given workload
/// if the pdb already exists it returns Ok without attempting to mutate the object
pub async fn reconcile(
    instance: &ChallengeInstance,
    container: &ContainerSpec,
    namespace: &str,
    ctx: &Context,
) -> Result<()> {
    let api: Api<PodDisruptionBudget> = Api::namespaced(ctx.client.clone(), namespace);

    let pdb_name = format!("{}-pdb", container.hostname);

    let pdb = PodDisruptionBudget {
        metadata: kube::api::ObjectMeta {
            name: Some(pdb_name.clone()),
            owner_references: Some(vec![instance.controller_owner_ref(&()).unwrap()]),
            namespace: Some(namespace.to_string()),
            ..Default::default()
        },
        spec: Some(PodDisruptionBudgetSpec {
            max_unavailable: Some(IntOrString::Int(0)),
            selector: Some(LabelSelector {
                match_labels: Some({
                    let mut labels = BTreeMap::new();
                    labels.insert(
                        "berg.norelect.ch/container".to_string(),
                        container.hostname.clone(),
                    );
                    labels
                }),
                ..Default::default()
            }),
            ..Default::default()
        }),
        ..Default::default()
    };

    match api.create(&PostParams::default(), &pdb).await {
        Ok(_) => {
            info!("Created PodDisruptionBudget {} in {}", pdb_name, namespace);
            Ok(())
        }
        Err(kube::Error::Api(ae)) if ae.code == 409 => {
            info!("PodDisruptionBudget {} already exists", pdb_name);
            Ok(())
        }
        Err(e) => Err(e.into()),
    }
}
