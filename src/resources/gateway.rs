use crate::{
    crds::{
        BackendRef, ChallengeInstance, ChallengeInstanceClass, ContainerSpec, HTTPBackendRef,
        HTTPRoute, HTTPRouteRule, HTTPRouteSpec, ParentReference, PortType, ServiceEndpoint,
        TLSRoute, TLSRouteRule, TLSRouteSpec,
    },
    error::{Error, Result},
    reconciler::Context,
};
use kube::{
    api::{Api, PostParams},
    Resource,
};
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
) -> Result<Vec<ServiceEndpoint>> {
    let api: Api<HTTPRoute> = Api::namespaced(ctx.client.clone(), namespace);

    let mut endpoints = Vec::new();

    for port in &container.ports {
        if port.r#type == PortType::PublicHttpRoute {
            let service_guid = Uuid::new_v4();
            let mut hostname = format!("{}.{}", service_guid, class.spec.gateway.domain);

            let route_name = format!("{}-{}", container.hostname, port.port);

            let route = HTTPRoute {
                metadata: kube::api::ObjectMeta {
                    name: Some(route_name.clone()),
                    namespace: Some(namespace.to_string()),
                    owner_references: Some(vec![instance.controller_owner_ref(&()).unwrap()]),
                    labels: Some({
                        let mut labels = BTreeMap::new();
                        labels.insert(
                            "app.kubernetes.io/managed-by".to_string(),
                            "berg".to_string(),
                        );
                        labels.insert(
                            "app.kubernetes.io/component".to_string(),
                            "http-route".to_string(),
                        );
                        labels.insert(
                            "berg.norelect.ch/hostname".to_string(),
                            service_guid.to_string(),
                        );
                        if let Some(ref status) = instance.status {
                            if let Some(ref instance_id) = status.instance_id {
                                labels.insert(
                                    "berg.norelect.ch/instance-id".to_string(),
                                    instance_id.clone(),
                                );
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
                }
                Err(kube::Error::Api(ae)) if ae.code == 409 => {
                    let route = api.get(&route.metadata.name.unwrap()).await?;
                    hostname = route.spec.hostnames.unwrap().first().unwrap().to_owned();
                    info!("HTTPRoute {} already exists", route_name);
                }
                Err(e) => return Err(Error::from(e)),
            }
            endpoints.push(ServiceEndpoint {
                name: (port.name.to_owned())
                    .unwrap_or(port.port.to_string())
                    .to_owned(),
                hostname,
                port: class.spec.gateway.http_port,
                protocol: "TCP".to_string(),
                app_protocol: Some("HTTP".to_string()),
                tls: Some(true),
            });
        }
    }

    Ok(endpoints)
}

/// Create TLSRoute for publicTlsRoute ports
pub async fn create_tls_routes(
    instance: &ChallengeInstance,
    container: &ContainerSpec,
    namespace: &str,
    class: &ChallengeInstanceClass,
    ctx: &Context,
) -> Result<Vec<ServiceEndpoint>> {
    let api: Api<TLSRoute> = Api::namespaced(ctx.client.clone(), namespace);
    let mut endpoints = Vec::new();

    for port in &container.ports {
        if port.r#type == PortType::PublicTlsRoute {
            let service_guid = Uuid::new_v4();
            let mut hostname = format!("{}.{}", service_guid, class.spec.gateway.domain);

            let route_name = format!("{}-{}", container.hostname, port.port);

            let route = TLSRoute {
                metadata: kube::api::ObjectMeta {
                    name: Some(route_name.clone()),
                    namespace: Some(namespace.to_string()),
                    owner_references: Some(vec![instance.controller_owner_ref(&()).unwrap()]),
                    labels: Some({
                        let mut labels = BTreeMap::new();
                        labels.insert(
                            "app.kubernetes.io/managed-by".to_string(),
                            "berg".to_string(),
                        );
                        labels.insert(
                            "app.kubernetes.io/component".to_string(),
                            "tls-route".to_string(),
                        );
                        labels.insert(
                            "berg.norelect.ch/hostname".to_string(),
                            service_guid.to_string(),
                        );
                        if let Some(ref status) = instance.status {
                            if let Some(ref instance_id) = status.instance_id {
                                labels.insert(
                                    "berg.norelect.ch/instance-id".to_string(),
                                    instance_id.clone(),
                                );
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
                }
                Err(kube::Error::Api(ae)) if ae.code == 409 => {
                    info!("TLSRoute {} already exists", route_name);
                    let route = api.get(&route.metadata.name.unwrap()).await?;
                    hostname = route.spec.hostnames.unwrap().first().unwrap().to_owned();
                }
                Err(e) => return Err(Error::from(e)),
            }
            endpoints.push(ServiceEndpoint {
                name: (port.name.to_owned())
                    .unwrap_or(port.port.to_string())
                    .to_owned(),
                hostname,
                port: class.spec.gateway.tls_port,
                protocol: "TCP".to_string(),
                app_protocol: Some("TCP".to_string()),
                tls: Some(true),
            });
        }
    }

    Ok(endpoints)
}
