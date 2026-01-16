use crate::{
    crds::{
        Challenge, ChallengeInstance, ChallengeInstanceClass, ContainerSpec, PortSpec, PortType,
        ServiceEndpoint,
    },
    error::Result,
    reconciler::Context,
};
use k8s_openapi::{
    api::core::v1::{Service, ServicePort, ServiceSpec},
    apimachinery::pkg::apis::meta::v1::OwnerReference,
};
use kube::{
    api::{Api, PostParams},
    Resource,
};
use std::collections::BTreeMap;
use tracing::{debug, info};

/// reconcile attempts to create ClusterIP and NodePort services for a deployment as required
/// if the service already exists it returns Ok without attempting to mutate the object
pub async fn reconcile(
    class: &ChallengeInstanceClass,
    instance: &ChallengeInstance,
    _challenge: &Challenge,
    container: &ContainerSpec,
    namespace: &str,
    ctx: &Context,
) -> Result<Vec<ServiceEndpoint>> {
    let api: Api<Service> = Api::namespaced(ctx.client.clone(), namespace);

    let mut endpoints = vec![];

    // create ClusterIP service
    if !container.ports.is_empty() {
        let service_name = container.hostname.to_string();
        let svc = make_svc(
            &service_name,
            "ClusterIP",
            namespace,
            &container.hostname,
            &container.ports,
            instance.controller_owner_ref(&()).unwrap(),
        );

        match api.create(&PostParams::default(), &svc).await {
            Ok(_) => info!("Created service {} in {}", service_name, namespace),
            Err(kube::Error::Api(ae)) if ae.code == 409 => {
                debug!("Service {} already exists", service_name)
            }
            Err(e) => return Err(e.into()),
        };
    }

    // if any nodeport ports exist, create a node port service
    let node_ports = container
        .ports
        .iter()
        .filter(|p| p.r#type == PortType::PublicPort)
        .cloned()
        .collect::<Vec<_>>();
    if !node_ports.is_empty() {
        let service_name = format!("{}-node-port", container.hostname);
        let svc = make_svc(
            &service_name,
            "NodePort",
            namespace,
            &container.hostname,
            &node_ports,
            instance.controller_owner_ref(&()).unwrap(),
        );
        let svc = api.create(&PostParams::default(), &svc).await;

        if let Ok(svc) = svc {
            for port in &node_ports {
                endpoints.push(ServiceEndpoint {
                    name: (port.name.to_owned())
                        .unwrap_or(format!("{}:{}", container.hostname, port.port))
                        .to_owned(),
                    hostname: class.spec.gateway.domain.clone(),
                    port: svc
                        .spec
                        .to_owned()
                        .unwrap()
                        .ports
                        .unwrap()
                        .iter()
                        .find(|p| p.port as u16 == port.port)
                        .unwrap()
                        .node_port
                        .unwrap_or_default() as u16,
                    protocol: "TCP".to_string(),
                    app_protocol: port.app_protocol.clone(),
                    tls: Some(false),
                });
            }
        } else if let Err(kube::Error::Api(ae)) = svc {
            if ae.code == 409 {
                debug!("Service {} already exists", service_name)
            } else {
                return Err(kube::Error::Api(ae).into());
            }
        } else if let Err(e) = svc {
            return Err(e.into());
        }
    }

    Ok(endpoints)
}

fn make_svc(
    name: &str,
    service_type: &str,
    namespace: &str,
    hostname: &str,
    ports: &[PortSpec],
    oref: OwnerReference,
) -> Service {
    Service {
        metadata: kube::api::ObjectMeta {
            name: Some(name.to_owned()),
            namespace: Some(namespace.to_string()),
            owner_references: Some(vec![oref]),
            labels: Some({
                let mut labels = BTreeMap::new();
                labels.insert(
                    "app.kubernetes.io/managed-by".to_string(),
                    "berg".to_string(),
                );
                labels
            }),
            ..Default::default()
        },
        spec: Some(ServiceSpec {
            type_: Some(service_type.to_string()),
            selector: Some({
                let mut selector = BTreeMap::new();
                selector.insert(
                    "berg.norelect.ch/container".to_string(),
                    hostname.to_owned(),
                );
                selector
            }),
            ports: Some(
                ports
                    .iter()
                    .map(|p| ServicePort {
                        name: Some(
                            p.name
                                .to_owned()
                                .unwrap_or(format!("{}-{}", hostname, p.port))
                                .to_owned(),
                        ),
                        port: p.port as i32,
                        protocol: Some(p.protocol.to_uppercase()),
                        ..Default::default()
                    })
                    .collect(),
            ),
            ..Default::default()
        }),
        ..Default::default()
    }
}
