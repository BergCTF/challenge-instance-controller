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
use tracing::info;

pub async fn create(
    instance: &ChallengeInstance,
    _challenge: &Challenge,
    container: &ContainerSpec,
    namespace: &str,
    ctx: &Context,
) -> Result<()> {
    let api: Api<Service> = Api::namespaced(ctx.client.clone(), namespace);

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
                info!("Service {} already exists", service_name)
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
        match api.create(&PostParams::default(), &svc).await {
            Ok(_) => info!("Created service {} in {}", service_name, namespace),
            Err(kube::Error::Api(ae)) if ae.code == 409 => {
                info!("Service {} already exists", service_name)
            }
            Err(e) => return Err(e.into()),
        };
    }

    Ok(())
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
                        name: p.name.to_owned(),
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

pub async fn discover_endpoints(
    instance: &ChallengeInstance,
    challenge: &Challenge,
    namespace: &str,
    class: &ChallengeInstanceClass,
    ctx: &Context,
) -> Result<Vec<ServiceEndpoint>> {
    let mut endpoints = Vec::new();
    let api: Api<Service> = Api::namespaced(ctx.client.clone(), namespace);

    for container in &challenge.spec.containers {
        for port in &container.ports {
            match port.r#type {
                PortType::InternalPort => continue,
                PortType::PublicPort => {
                    // Query the service to get the actual NodePort assigned by Kubernetes
                    let service_name = format!("{}-node-port", container.hostname);
                    let actual_port = match api.get(&service_name).await {
                        Ok(svc) => {
                            // Extract the NodePort from the service
                            svc.spec
                                .and_then(|spec| spec.ports)
                                .and_then(|ports| {
                                    ports.into_iter().find(|p| p.port as u16 == port.port)
                                })
                                .and_then(|p| p.node_port)
                                .unwrap_or(0) as u16
                        }
                        Err(_) => 0,
                    };

                    endpoints.push(ServiceEndpoint {
                        name: (port.name.to_owned())
                            .unwrap_or(port.port.to_string())
                            .to_owned(),
                        hostname: class.spec.gateway.domain.clone(),
                        port: actual_port,
                        protocol: "TCP".to_string(),
                        app_protocol: port.app_protocol.clone(),
                        tls: Some(false),
                    });
                }
                PortType::PublicHttpRoute | PortType::PublicTlsRoute => {
                    // For HTTP/TLS routes, construct hostname from instance ID
                    let hostname = if let Some(ref status) = instance.status {
                        if let Some(ref instance_id) = status.instance_id {
                            format!("{}.{}", instance_id, class.spec.gateway.domain)
                        } else {
                            class.spec.gateway.domain.clone()
                        }
                    } else {
                        class.spec.gateway.domain.clone()
                    };

                    let is_tls = port.r#type == PortType::PublicTlsRoute;

                    endpoints.push(ServiceEndpoint {
                        name: (port.name.to_owned())
                            .unwrap_or(port.port.to_string())
                            .clone(),
                        hostname,
                        port: if is_tls {
                            class.spec.gateway.tls_port
                        } else {
                            class.spec.gateway.http_port
                        },
                        protocol: "TCP".to_string(),
                        app_protocol: Some(if is_tls {
                            "HTTPS".to_string()
                        } else {
                            "HTTP".to_string()
                        }),
                        tls: Some(is_tls),
                    });
                }
            }
        }
    }

    Ok(endpoints)
}
