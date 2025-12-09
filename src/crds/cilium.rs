use k8s_openapi::apimachinery::pkg::apis::meta::v1::LabelSelector;
use kube::CustomResource;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// CiliumNetworkPolicy CRD
#[derive(CustomResource, Serialize, Deserialize, Clone, Debug, JsonSchema)]
#[kube(
    group = "cilium.io",
    version = "v2",
    kind = "CiliumNetworkPolicy",
    namespaced
)]
#[serde(rename_all = "camelCase")]
pub struct CiliumNetworkPolicySpec {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub endpoint_selector: Option<LabelSelector>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub egress: Option<Vec<CiliumEgressRule>>,
}

#[derive(Serialize, Deserialize, Clone, Debug, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct CiliumEgressRule {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub to_endpoints: Option<Vec<LabelSelector>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub to_entities: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub to_fqd_ns: Option<Vec<CiliumFQDNRule>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub to_ports: Option<Vec<CiliumPortRule>>,
}

#[derive(Serialize, Deserialize, Clone, Debug, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct CiliumFQDNRule {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub match_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub match_pattern: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct CiliumPortRule {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ports: Option<Vec<CiliumPortProtocol>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rules: Option<CiliumL7Rule>,
}

#[derive(Serialize, Deserialize, Clone, Debug, JsonSchema)]
pub struct CiliumPortProtocol {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub port: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub protocol: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug, JsonSchema)]
pub struct CiliumL7Rule {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dns: Option<Vec<CiliumDnsRule>>,
}

#[derive(Serialize, Deserialize, Clone, Debug, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct CiliumDnsRule {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub match_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub match_pattern: Option<String>,
}

/// Cilium entity constants
pub mod entities {
    pub const HOST: &str = "host";
    pub const REMOTE_NODE: &str = "remote-node";
    pub const KUBE_API_SERVER: &str = "kube-apiserver";
    pub const INGRESS: &str = "ingress";
    pub const CLUSTER: &str = "cluster";
    pub const INIT: &str = "init";
    pub const HEALTH: &str = "health";
    pub const UNMANAGED: &str = "unmanaged";
    pub const WORLD: &str = "world";
    pub const ALL: &str = "all";
}

/// Protocol constants
pub mod protocols {
    pub const TCP: &str = "TCP";
    pub const UDP: &str = "UDP";
}
