use crate::{
    crds::ChallengeInstance,
    error::{Error, Result},
    reconciler::Context,
    resources::labels,
};
use k8s_openapi::api::core::v1::{Namespace, Secret};
use kube::{
    api::{Api, PostParams},
    Client, Resource,
};
use tracing::info;

/// reconcile attempts to create a Namespace
/// if the Namespace already exists it returns OK
pub async fn reconcile(
    instance: &ChallengeInstance,
    namespace_name: &str,
    ctx: &Context,
) -> Result<()> {
    let namespaces: Api<Namespace> = Api::all(ctx.client.clone());

    let ns = Namespace {
        metadata: kube::api::ObjectMeta {
            name: Some(namespace_name.to_string()),
            labels: Some(labels::namespace_labels(instance, ctx)),
            owner_references: Some(vec![instance.controller_owner_ref(&()).unwrap()]),
            ..Default::default()
        },
        ..Default::default()
    };

    match namespaces.create(&PostParams::default(), &ns).await {
        Ok(_) => {
            info!("Created namespace {}", namespace_name);
            Ok(())
        }
        Err(kube::Error::Api(ae)) if ae.code == 409 => {
            info!("Namespace {} already exists", namespace_name);
            Ok(())
        }
        Err(e) => Err(Error::from(e)),
    }
}

pub async fn copy_pull_secret(
    client: &Client,
    secret_name: &str,
    target_namespace: &str,
) -> Result<()> {
    // Read secret from controller namespace (assuming it's in the same namespace as the controller)
    let source_secrets: Api<Secret> = Api::default_namespaced(client.clone());

    match source_secrets.get(secret_name).await {
        Ok(mut secret) => {
            // Copy to target namespace
            secret.metadata.namespace = Some(target_namespace.to_string());
            secret.metadata.uid = None;
            secret.metadata.resource_version = None;

            let target_secrets: Api<Secret> = Api::namespaced(client.clone(), target_namespace);

            match target_secrets.create(&PostParams::default(), &secret).await {
                Ok(_) => {
                    info!(
                        "Copied pull secret {} to namespace {}",
                        secret_name, target_namespace
                    );
                    Ok(())
                }
                Err(kube::Error::Api(ae)) if ae.code == 409 => {
                    info!("Pull secret already exists in {}", target_namespace);
                    Ok(())
                }
                Err(e) => Err(Error::from(e)),
            }
        }
        Err(kube::Error::Api(ae)) if ae.code == 404 => {
            info!("Pull secret {} not found, skipping", secret_name);
            Ok(())
        }
        Err(e) => Err(Error::from(e)),
    }
}
