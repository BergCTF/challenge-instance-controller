use berg_operator::{
    config::ControllerConfig,
    crds::ChallengeInstance,
    reconciler::{self, Context},
    telemetry::{self, Metrics},
};
use kube::{
    runtime::{controller::Controller, watcher::Config as WatcherConfig},
    Client,
};
use std::sync::Arc;
use tracing::info;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize tracing
    telemetry::init();

    info!("Starting Berg Challenge Instance Controller");

    // Load configuration
    let config = Arc::new(ControllerConfig::from_env()?);
    info!("Configuration loaded");

    // Create Kubernetes client
    let client = Client::try_default().await?;
    info!("Connected to Kubernetes cluster");

    // Initialize metrics
    let metrics = Arc::new(Metrics::default());

    // Create context
    let ctx = Arc::new(Context {
        client: client.clone(),
        config,
        metrics,
    });

    // Create controller
    let instances = kube::Api::<ChallengeInstance>::all(client.clone());

    info!("Starting controller loop");

    use futures::StreamExt;

    Controller::new(instances, WatcherConfig::default())
        .shutdown_on_signal()
        .run(
            reconciler::reconcile,
            reconciler::error_policy,
            ctx,
        )
        .for_each(|res| async move {
            match res {
                Ok(o) => info!("Reconciled: {:?}", o),
                Err(e) => tracing::error!("Reconciliation error: {:?}", e),
            }
        })
        .await;

    Ok(())
}
