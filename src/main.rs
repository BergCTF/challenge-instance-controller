use berg_operator::{
    config::ControllerConfig,
    crds::ChallengeInstance,
    reconciler::{self, Context},
    telemetry::{self, Metrics},
};
use futures::StreamExt;
use kube::{
    runtime::{controller::Controller, watcher::Config as WatcherConfig},
    Client,
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

    info!("Starting controller loop");
    Controller::new(instances, WatcherConfig::default())
        .shutdown_on_signal()
        .run(reconciler::reconcile, reconciler::error_policy, ctx)
        .for_each(|res| async move {
            match res {
                Ok(o) => info!("Reconciled: {:?}", o),
                Err(e) => tracing::error!("Reconciliation error: {:?}", e),
            }
        })
        .await;

    Ok(())
}
