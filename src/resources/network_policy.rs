use crate::{
    crds::{
        cilium::{
            entities, CiliumDnsRule, CiliumEgressRule, CiliumL7Rule, CiliumPortProtocol,
            CiliumPortRule,
        },
        Challenge, ChallengeInstance, ChallengeInstanceClass, CiliumNetworkPolicy,
        CiliumNetworkPolicySpec,
    },
    error::{Error, Result},
    reconciler::Context,
};
use k8s_openapi::apimachinery::pkg::apis::meta::v1::LabelSelector;
use kube::{
    api::{Api, PostParams},
    Resource,
};
use std::collections::BTreeMap;
use tracing::info;

/// reconcile attempts to create a CiliumNetworkPolicy for the challenge instance
/// if a CiliumNetworkPolicy with that name already exists it returns Ok
/// it does not attempt to mutate existing policies
pub async fn reconcile(
    instance: &ChallengeInstance,
    challenge: &Challenge,
    namespace: &str,
    class: &ChallengeInstanceClass,
    ctx: &Context,
) -> Result<()> {
    let api: Api<CiliumNetworkPolicy> = Api::namespaced(ctx.client.clone(), namespace);

    let mut egress_rules = vec![
        // Rule 1: Allow DNS to kube-dns
        CiliumEgressRule {
            to_endpoints: Some(vec![LabelSelector {
                match_labels: Some({
                    let mut labels = BTreeMap::new();
                    labels.insert(
                        "k8s:io.kubernetes.pod.namespace".to_string(),
                        "kube-system".to_string(),
                    );
                    labels.insert("k8s:k8s-app".to_string(), "kube-dns".to_string());
                    labels
                }),
                ..Default::default()
            }]),
            to_entities: None,
            to_fqd_ns: None,
            to_ports: Some(vec![CiliumPortRule {
                ports: Some(vec![CiliumPortProtocol {
                    port: Some("53".to_string()),
                    protocol: None,
                }]),
                rules: if challenge.spec.allow_outbound_traffic {
                    None
                } else {
                    // If outbound traffic is forbidden, only accept DNS requests for internal services
                    Some(CiliumL7Rule {
                        dns: Some(vec![CiliumDnsRule {
                            match_name: None,
                            match_pattern: Some(format!("*.{}.svc.cluster.local.", namespace)),
                        }]),
                    })
                },
            }]),
        },
        // Rule 2: Allow traffic to other pods in the same namespace
        CiliumEgressRule {
            to_endpoints: Some(vec![LabelSelector::default()]), // Empty selector matches all
            to_entities: None,
            to_fqd_ns: None,
            to_ports: None,
        },
        // Rule 3: Allow outbound traffic to host (for OIDC callbacks)
        CiliumEgressRule {
            to_endpoints: None,
            to_entities: Some(vec![entities::HOST.to_string()]),
            to_fqd_ns: None,
            to_ports: Some(vec![CiliumPortRule {
                ports: Some(vec![
                    CiliumPortProtocol {
                        port: Some(class.spec.gateway.http_port.to_string()),
                        protocol: None,
                    },
                    CiliumPortProtocol {
                        port: Some(class.spec.gateway.tls_port.to_string()),
                        protocol: None,
                    },
                ]),
                rules: None,
            }]),
        },
    ];

    // Rule 4: If allow_outbound_traffic is true, allow all traffic to world
    if challenge.spec.allow_outbound_traffic {
        egress_rules.push(CiliumEgressRule {
            to_endpoints: None,
            to_entities: Some(vec![entities::WORLD.to_string()]),
            to_fqd_ns: None,
            to_ports: None,
        });
    }

    let policy = CiliumNetworkPolicy {
        metadata: kube::api::ObjectMeta {
            name: Some("challenge-network-policy".to_string()),
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
                    "network-policy".to_string(),
                );
                labels
            }),
            ..Default::default()
        },
        spec: CiliumNetworkPolicySpec {
            endpoint_selector: Some(LabelSelector::default()), // Empty selector matches all pods in namespace
            egress: Some(egress_rules),
        },
    };

    match api.create(&PostParams::default(), &policy).await {
        Ok(_) => {
            info!("Created CiliumNetworkPolicy in {}", namespace);
            Ok(())
        }
        Err(kube::Error::Api(ae)) if ae.code == 409 => {
            info!("CiliumNetworkPolicy already exists in {}", namespace);
            Ok(())
        }
        Err(e) => Err(Error::from(e)),
    }
}
