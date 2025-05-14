use std::{sync::Arc, time::Duration};

use futures::StreamExt;
use k8s_openapi::api::core::v1::{ConfigMap, Namespace, Pod, Secret, Service};
use kube::{
    runtime::{controller::Action, watcher, Controller},
    Api, Client,
};

use crate::crd::{
    external::{CiliumNetworkPolicy, HTTPRoute, TLSRoute},
    ChallengeInstance,
};

mod config;
mod error;
mod reconcile;

#[derive(Clone)]
pub struct ReconcilerContext {
    pub apis: ReconcilerApis,
    pub client: kube::Client,
    pub config: config::Config,
}

#[derive(Clone)]
pub struct ReconcilerApis {
    challenge_instances: Api<ChallengeInstance>,
    namespaces: Api<Namespace>,
    pods: Api<Pod>,
    secrets: Api<Secret>,
    cilium_network_policies: Api<CiliumNetworkPolicy>,
    services: Api<Service>,
    http_routes: Api<HTTPRoute>,
    tls_routes: Api<TLSRoute>,
    config_maps: Api<ConfigMap>,
}

// instrument the controller cycle
pub async fn instrument() -> Result<(), kube::Error> {
    // load once at startup to make sure it's valid and cached
    let _ = config::config();
    let client = Client::try_default().await?;

    let challenge_instances = Api::<ChallengeInstance>::all(client.clone());
    // we create a namespace per instance
    let namespaces = Api::<Namespace>::all(client.clone());
    // we create pods for the instance
    let pods = Api::<Pod>::all(client.clone());
    // the berg pull secret
    let secrets = Api::<Secret>::all(client.clone());
    // cilium network policies
    let cilium_network_policies = Api::<CiliumNetworkPolicy>::all(client.clone());
    // services
    let services = Api::<Service>::all(client.clone());
    // http_routes
    let http_routes = Api::<HTTPRoute>::all(client.clone());
    // tls_routes
    let tls_routes = Api::<TLSRoute>::all(client.clone());
    // config_maps for dynamic flags
    let config_maps = Api::<ConfigMap>::all(client.clone());

    let context = ReconcilerContext {
        apis: ReconcilerApis {
            challenge_instances: challenge_instances.clone(),
            namespaces: namespaces.clone(),
            pods: pods.clone(),
            secrets: secrets.clone(),
            cilium_network_policies: cilium_network_policies.clone(),
            services: services.clone(),
            http_routes: http_routes.clone(),
            tls_routes: tls_routes.clone(),
            config_maps: config_maps.clone(),
        },
        client: client.clone(),
        config: config::config().clone(),
    };

    Controller::new(challenge_instances.clone(), Default::default())
        .owns(namespaces, watcher::Config::default())
        .owns(pods, watcher::Config::default())
        .owns(secrets, watcher::Config::default())
        .owns(cilium_network_policies, watcher::Config::default())
        .owns(services, watcher::Config::default())
        .owns(http_routes, watcher::Config::default())
        .owns(tls_routes, watcher::Config::default())
        .owns(config_maps, watcher::Config::default())
        .run(reconcile::reconcile, error_policy, Arc::new(context))
        .for_each(|_| futures::future::ready(()))
        .await;

    Ok(())
}

// error policy to determine the action when the reconciliation fails
fn error_policy(_object: Arc<ChallengeInstance>, _err: &error::Error, _ctx: Arc<ReconcilerContext>) -> Action {
    Action::requeue(Duration::from_secs(5))
}
