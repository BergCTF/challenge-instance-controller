use berg_operator::{
    config::ControllerConfig,
    crds::{ChallengeInstance, CiliumNetworkPolicy, HTTPRoute, TLSRoute},
    reconciler::{self, Context},
    telemetry::{self, Metrics},
};
use futures::StreamExt;
use k8s_openapi::api::{
    apps::v1::Deployment,
    core::v1::{ConfigMap, Namespace, Service},
    policy::v1::PodDisruptionBudget,
};
use kube::{
    runtime::{controller::Controller, watcher::Config as WatcherConfig},
    Api, Client,
};
use std::sync::Arc;
use tracing::info;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    telemetry::init();
    let metrics = Arc::new(Metrics::default());

    info!("Starting Berg Challenge Instance Controller");
    let config = Arc::new(ControllerConfig::from_env()?);
    info!("Configuration loaded");
    let client = Client::try_default().await?;
    info!("Connected to Kubernetes cluster");

    let ctx = Arc::new(Context {
        client: client.clone(),
        config,
        metrics,
    });

    let instances = kube::Api::<ChallengeInstance>::all(client.clone());

    // instances owned by the controller. this is used to trigger reconciliations of parent
    // resources if child resources change
    let namespaces = Api::<Namespace>::all(client.to_owned()); // used for challenge namespaces
    let config_maps = Api::<ConfigMap>::all(client.to_owned()); // used for dynamic flags
    let deployments = Api::<Deployment>::all(client.to_owned());
    let np = Api::<CiliumNetworkPolicy>::all(client.to_owned());
    let pdb = Api::<PodDisruptionBudget>::all(client.to_owned());
    let services = Api::<Service>::all(client.to_owned());
    let http_routes = Api::<HTTPRoute>::all(client.to_owned());
    let tls_routes = Api::<TLSRoute>::all(client.to_owned());

    info!("Starting controller loop");
    Controller::new(instances, WatcherConfig::default())
        .owns(namespaces, WatcherConfig::default())
        .owns(config_maps, WatcherConfig::default())
        .owns(deployments, WatcherConfig::default())
        .owns(np, WatcherConfig::default())
        .owns(pdb, WatcherConfig::default())
        .owns(services, WatcherConfig::default())
        .owns(http_routes, WatcherConfig::default())
        .owns(tls_routes, WatcherConfig::default())
        .shutdown_on_signal()
        .run(reconciler::reconcile, reconciler::error_policy, ctx)
        .for_each(|res| async move {
            match res {
                Ok(o) => info!("Reconciled: {:?}", o),
                // if the object cannot be found it was likely deleted. we can ignore this.
                Err(kube::runtime::controller::Error::ObjectNotFound(_)) => {}
                Err(e) => tracing::error!("[!] Reconciliation error: {:?}", e),
            }
        })
        .await;

    Ok(())
}
