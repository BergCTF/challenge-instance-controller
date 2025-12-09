use kube::CustomResource;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// ChallengeInstanceClass defines configuration for ChallengeInstances
/// Similar to StorageClass in Kubernetes, this allows different "tiers" of instances
#[derive(CustomResource, Serialize, Deserialize, Clone, Debug, JsonSchema)]
#[kube(
    group = "berg.norelect.ch",
    version = "v1",
    kind = "ChallengeInstanceClass",
    namespaced = false,
    printcolumn = r#"{"name": "Gateway", "type": "string", "jsonPath": ".spec.gateway.name"}"#,
    printcolumn = r#"{"name": "Default", "type": "boolean", "jsonPath": ".spec.default"}"#,
    printcolumn = r#"{"name": "Age", "type": "date", "jsonPath": ".metadata.creationTimestamp"}"#
)]
#[serde(rename_all = "camelCase")]
pub struct ChallengeInstanceClassSpec {
    /// Gateway configuration for routing challenge traffic
    pub gateway: GatewayConfig,

    /// Default resource requests for challenge containers
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default_resources: Option<ResourceDefaults>,

    /// Network configuration
    #[serde(skip_serializing_if = "Option::is_none")]
    pub network: Option<NetworkConfig>,

    /// Image pull configuration
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image_pull: Option<ImagePullConfig>,

    /// Security and runtime configuration
    #[serde(skip_serializing_if = "Option::is_none")]
    pub security: Option<SecurityConfig>,

    /// Whether this is the default class
    #[serde(default)]
    pub default: bool,

    /// Default timeout for instances using this class
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default_timeout: Option<String>,

    /// Challenge namespace where instances will be created
    pub challenge_namespace: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct GatewayConfig {
    /// Gateway name
    pub name: String,

    /// Gateway namespace
    pub namespace: String,

    /// HTTP listener name in the Gateway
    pub http_listener_name: String,

    /// TLS listener name in the Gateway
    pub tls_listener_name: String,

    /// Challenge domain for routing (e.g., challenges.example.com)
    pub domain: String,

    /// HTTP port exposed by the Gateway
    #[serde(default = "default_http_port")]
    pub http_port: u16,

    /// TLS port exposed by the Gateway
    #[serde(default = "default_tls_port")]
    pub tls_port: u16,
}

#[derive(Serialize, Deserialize, Clone, Debug, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ResourceDefaults {
    /// Default CPU request (e.g., "100m")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cpu_request: Option<String>,

    /// Default CPU limit (e.g., "500m")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cpu_limit: Option<String>,

    /// Default memory request (e.g., "128Mi")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub memory_request: Option<String>,

    /// Default memory limit (e.g., "512Mi")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub memory_limit: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct NetworkConfig {
    /// Default egress bandwidth limit (e.g., "10M")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub egress_bandwidth: Option<String>,

    /// Default ingress bandwidth limit (e.g., "10M")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ingress_bandwidth: Option<String>,

    /// Enable additional headless service for containers
    #[serde(default)]
    pub additional_headless_service: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ImagePullConfig {
    /// Image pull policy (Always, IfNotPresent, Never)
    #[serde(default = "default_image_pull_policy")]
    pub policy: String,

    /// Name of secret containing registry credentials
    #[serde(skip_serializing_if = "Option::is_none")]
    pub secret_name: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct SecurityConfig {
    /// RuntimeClass to use for pods
    #[serde(skip_serializing_if = "Option::is_none")]
    pub runtime_class_name: Option<String>,

    /// Pod security context settings
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pod_security_context: Option<PodSecurityContextConfig>,
}

#[derive(Serialize, Deserialize, Clone, Debug, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct PodSecurityContextConfig {
    /// Run as non-root user
    #[serde(skip_serializing_if = "Option::is_none")]
    pub run_as_non_root: Option<bool>,

    /// FS group for volumes
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fs_group: Option<i64>,

    /// Supplemental groups
    #[serde(skip_serializing_if = "Option::is_none")]
    pub supplemental_groups: Option<Vec<i64>>,
}

fn default_http_port() -> u16 {
    80
}

fn default_tls_port() -> u16 {
    443
}

fn default_image_pull_policy() -> String {
    "IfNotPresent".to_string()
}
