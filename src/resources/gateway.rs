use crate::{
    crds::{
        ChallengeInstance, ChallengeInstanceClass, ContainerSpec, HTTPBackendRef, HTTPRoute,
        HTTPRouteRule, HTTPRouteSpec, ParentReference, PortType, TLSRoute, TLSRouteRule,
        TLSRouteSpec, BackendRef,
    },
    error::{Error, Result},
    reconciler::Context,
};
use kube::api::{Api, PostParams};
use std::collections::BTreeMap;
use tracing::info;
use uuid::Uuid;

/// Create HTTPRoute for publicHttpRoute ports
pub async fn create_http_routes(
    instance: &ChallengeInstance,
    container: &ContainerSpec,
    namespace: &str,
    class: &ChallengeInstanceClass,
    ctx: &Context,
) -> Result<Vec<String>> {
    let api: Api<HTTPRoute> = Api::namespaced(ctx.client.clone(), namespace);
    let mut hostnames = Vec::new();

    for port in &container.ports {
        if port.r#type == PortType::PublicHttpRoute {
            let service_guid = Uuid::new_v4();
            let hostname = format!("{}.{}", service_guid, class.spec.gateway.domain);

            let route_name = format!("{}-{}", container.hostname, port.port);

            let route = HTTPRoute {
                metadata: kube::api::ObjectMeta {
                    name: Some(route_name.clone()),
                    namespace: Some(namespace.to_string()),
                    labels: Some({
                        let mut labels = BTreeMap::new();
                        labels.insert("app.kubernetes.io/managed-by".to_string(), "berg".to_string());
                        labels.insert("app.kubernetes.io/component".to_string(), "http-route".to_string());
                        labels.insert("berg.norelect.ch/hostname".to_string(), service_guid.to_string());
                        if let Some(ref status) = instance.status {
                            if let Some(ref instance_id) = status.instance_id {
                                labels.insert("berg.norelect.ch/instance-id".to_string(), instance_id.clone());
                            }
                        }
                        labels
                    }),
                    ..Default::default()
                },
                spec: HTTPRouteSpec {
                    hostnames: Some(vec![hostname.clone()]),
                    parent_refs: Some(vec![ParentReference {
                        group: None,
                        kind: Some("Gateway".to_string()),
                        namespace: Some(class.spec.gateway.namespace.clone()),
                        name: class.spec.gateway.name.clone(),
                        section_name: Some(class.spec.gateway.http_listener_name.clone()),
                        port: None,
                    }]),
                    rules: Some(vec![HTTPRouteRule {
                        name: None,
                        backend_refs: Some(vec![HTTPBackendRef {
                            group: None,
                            kind: None,
                            namespace: Some(namespace.to_string()),
                            name: container.hostname.clone(),
                            port: Some(port.port as i32),
                            weight: None,
                        }]),
                        filters: None,
                    }]),
                },
            };

            match api.create(&PostParams::default(), &route).await {
                Ok(_) => {
                    info!("Created HTTPRoute {} in {}", route_name, namespace);
                    hostnames.push(format!("{}:{}", hostname, class.spec.gateway.http_port));
                }
                Err(kube::Error::Api(ae)) if ae.code == 409 => {
                    info!("HTTPRoute {} already exists", route_name);
                    hostnames.push(format!("{}:{}", hostname, class.spec.gateway.http_port));
                }
                Err(e) => return Err(Error::from(e)),
            }
        }
    }

    Ok(hostnames)
}

/// Create TLSRoute for publicTlsRoute ports
pub async fn create_tls_routes(
    instance: &ChallengeInstance,
    container: &ContainerSpec,
    namespace: &str,
    class: &ChallengeInstanceClass,
    ctx: &Context,
) -> Result<Vec<String>> {
    let api: Api<TLSRoute> = Api::namespaced(ctx.client.clone(), namespace);
    let mut hostnames = Vec::new();

    for port in &container.ports {
        if port.r#type == PortType::PublicTlsRoute {
            let service_guid = Uuid::new_v4();
            let hostname = format!("{}.{}", service_guid, class.spec.gateway.domain);

            let route_name = format!("{}-{}", container.hostname, port.port);

            let route = TLSRoute {
                metadata: kube::api::ObjectMeta {
                    name: Some(route_name.clone()),
                    namespace: Some(namespace.to_string()),
                    labels: Some({
                        let mut labels = BTreeMap::new();
                        labels.insert("app.kubernetes.io/managed-by".to_string(), "berg".to_string());
                        labels.insert("app.kubernetes.io/component".to_string(), "tls-route".to_string());
                        labels.insert("berg.norelect.ch/hostname".to_string(), service_guid.to_string());
                        if let Some(ref status) = instance.status {
                            if let Some(ref instance_id) = status.instance_id {
                                labels.insert("berg.norelect.ch/instance-id".to_string(), instance_id.clone());
                            }
                        }
                        labels
                    }),
                    ..Default::default()
                },
                spec: TLSRouteSpec {
                    hostnames: Some(vec![hostname.clone()]),
                    parent_refs: Some(vec![ParentReference {
                        group: None,
                        kind: Some("Gateway".to_string()),
                        namespace: Some(class.spec.gateway.namespace.clone()),
                        name: class.spec.gateway.name.clone(),
                        section_name: Some(class.spec.gateway.tls_listener_name.clone()),
                        port: None,
                    }]),
                    rules: Some(vec![TLSRouteRule {
                        name: route_name.clone(),
                        backend_refs: Some(vec![BackendRef {
                            group: None,
                            kind: None,
                            namespace: Some(namespace.to_string()),
                            name: container.hostname.clone(),
                            port: Some(port.port as i32),
                            weight: None,
                        }]),
                    }]),
                },
            };

            match api.create(&PostParams::default(), &route).await {
                Ok(_) => {
                    info!("Created TLSRoute {} in {}", route_name, namespace);
                    hostnames.push(format!("{}:{}", hostname, class.spec.gateway.tls_port));
                }
                Err(kube::Error::Api(ae)) if ae.code == 409 => {
                    info!("TLSRoute {} already exists", route_name);
                    hostnames.push(format!("{}:{}", hostname, class.spec.gateway.tls_port));
                }
                Err(e) => return Err(Error::from(e)),
            }
        }
    }

    Ok(hostnames)
}
