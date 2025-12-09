use crate::{
    crds::{Challenge, ChallengeInstance, ChallengeInstanceClass, ContainerSpec, PortType, ServiceEndpoint},
    error::Result,
    reconciler::Context,
};
use k8s_openapi::api::core::v1::{Service, ServicePort, ServiceSpec};
use kube::api::{Api, PostParams};
use std::collections::BTreeMap;
use tracing::info;

pub async fn create(
    _instance: &ChallengeInstance,
    _challenge: &Challenge,
    container: &ContainerSpec,
    namespace: &str,
    ctx: &Context,
) -> Result<()> {
    let api: Api<Service> = Api::namespaced(ctx.client.clone(), namespace);

    // Create ClusterIP service for all containers
    for port in &container.ports {
        let service_name = format!("{}-{}", container.hostname, port.name);

        let service_type = match port.r#type {
            PortType::PublicPort => "NodePort",
            _ => "ClusterIP",
        };

        let svc = Service {
            metadata: kube::api::ObjectMeta {
                name: Some(service_name.clone()),
                namespace: Some(namespace.to_string()),
                labels: Some({
                    let mut labels = BTreeMap::new();
                    labels.insert("app.kubernetes.io/managed-by".to_string(), "berg".to_string());
                    labels
                }),
                ..Default::default()
            },
            spec: Some(ServiceSpec {
                type_: Some(service_type.to_string()),
                selector: Some({
                    let mut selector = BTreeMap::new();
                    selector.insert("berg.norelect.ch/container".to_string(), container.hostname.clone());
                    selector
                }),
                ports: Some(vec![ServicePort {
                    name: Some(port.name.clone()),
                    port: port.port as i32,
                    protocol: Some(port.protocol.to_uppercase()),
                    ..Default::default()
                }]),
                ..Default::default()
            }),
            ..Default::default()
        };

        match api.create(&PostParams::default(), &svc).await {
            Ok(_) => info!("Created service {} in {}", service_name, namespace),
            Err(kube::Error::Api(ae)) if ae.code == 409 => {
                info!("Service {} already exists", service_name)
            }
            Err(e) => return Err(e.into()),
        }
    }

    Ok(())
}

pub async fn discover_endpoints(
    instance: &ChallengeInstance,
    challenge: &Challenge,
    _namespace: &str,
    class: &ChallengeInstanceClass,
    _ctx: &Context,
) -> Result<Vec<ServiceEndpoint>> {
    let mut endpoints = Vec::new();

    for container in &challenge.spec.containers {
        for port in &container.ports {
            match port.r#type {
                PortType::InternalPort => continue,
                PortType::PublicPort => {
                    // For NodePort, return the domain with the node port
                    // In a real implementation, we'd query the service to get the actual NodePort
                    endpoints.push(ServiceEndpoint {
                        name: port.name.clone(),
                        hostname: class.spec.gateway.domain.clone(),
                        port: port.port, // Simplified - should query actual NodePort
                        protocol: "TCP".to_string(),
                        app_protocol: port.app_protocol.clone(),
                        tls: Some(false),
                    });
                }
                PortType::PublicHttpRoute | PortType::PublicTlsRoute => {
                    // For HTTP/TLS routes, construct hostname from instance ID
                    let hostname = if let Some(ref status) = instance.status {
                        if let Some(ref instance_id) = status.instance_id {
                            format!(
                                "{}.{}.{}",
                                instance_id,
                                instance.spec.challenge_ref.name,
                                class.spec.gateway.domain
                            )
                        } else {
                            class.spec.gateway.domain.clone()
                        }
                    } else {
                        class.spec.gateway.domain.clone()
                    };

                    let is_tls = port.r#type == PortType::PublicTlsRoute;

                    endpoints.push(ServiceEndpoint {
                        name: port.name.clone(),
                        hostname,
                        port: if is_tls {
                            class.spec.gateway.tls_port
                        } else {
                            class.spec.gateway.http_port
                        },
                        protocol: "TCP".to_string(),
                        app_protocol: Some(if is_tls { "HTTPS".to_string() } else { "HTTP".to_string() }),
                        tls: Some(is_tls),
                    });
                }
            }
        }
    }

    Ok(endpoints)
}
