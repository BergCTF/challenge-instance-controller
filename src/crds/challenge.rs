use kube::CustomResource;
use schemars::{JsonSchema, Schema, SchemaGenerator};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Schema for Kubernetes probe objects (liveness/readiness)
fn probe_schema(_gen: &mut SchemaGenerator) -> Schema {
    serde_json::from_value(serde_json::json!({
        "type": "object",
        "description": "Kubernetes probe configuration (exec, httpGet, tcpSocket, or grpc)",
        "nullable": true,
        "x-kubernetes-preserve-unknown-fields": true
    }))
    .unwrap()
}

/// Schema for date-time strings
fn datetime_schema(_gen: &mut SchemaGenerator) -> Schema {
    serde_json::from_value(serde_json::json!({
        "type": "string",
        "format": "date-time",
        "nullable": true
    }))
    .unwrap()
}

/// Challenge resource (read-only from controller perspective)
#[derive(CustomResource, Serialize, Deserialize, Clone, Debug, JsonSchema)]
#[kube(
    group = "berg.norelect.ch",
    version = "v1",
    kind = "Challenge",
    namespaced
)]
#[serde(rename_all = "camelCase")]
pub struct ChallengeSpec {
    pub display_name: Option<String>,
    pub author: String,
    pub description: String,
    pub flag: String,
    pub flag_format: String,
    pub dynamic_flag_mode: Option<DynamicFlagMode>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(schema_with = "datetime_schema")]
    pub hide_until: Option<String>,
    pub difficulty: String,
    pub static_value: Option<f64>,
    pub categories: Vec<String>,
    #[serde(default)]
    pub tags: Vec<String>,
    pub event: Option<String>,
    #[serde(default)]
    pub allow_outbound_traffic: bool,
    #[serde(default)]
    pub containers: Vec<ContainerSpec>,
    #[serde(default)]
    pub attachments: Vec<AttachmentSpec>,
}

#[derive(Serialize, Deserialize, Clone, Debug, JsonSchema, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum DynamicFlagMode {
    Suffix,
    Leetify,
}

#[derive(Serialize, Deserialize, Clone, Debug, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ContainerSpec {
    pub hostname: String,
    pub image: String,
    #[serde(default)]
    pub environment: HashMap<String, String>,
    pub ports: Vec<PortSpec>,
    pub dynamic_flag: Option<DynamicFlag>,
    pub resource_requests: Option<ResourceSpec>,
    pub resource_limits: Option<ResourceSpec>,
    #[serde(default)]
    pub additional_capabilities: Vec<String>,
    pub runtime_class_name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(schema_with = "probe_schema")]
    pub readiness_probe: Option<serde_json::Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(schema_with = "probe_schema")]
    pub liveness_probe: Option<serde_json::Value>,
    pub egress_bandwidth: Option<String>,
    pub ingress_bandwidth: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct PortSpec {
    pub name: Option<String>,
    pub port: u16,
    pub protocol: String,
    pub app_protocol: Option<String>,
    pub r#type: PortType,
}

#[derive(Serialize, Deserialize, Clone, Debug, JsonSchema, PartialEq)]
#[serde(rename_all = "camelCase")]
pub enum PortType {
    InternalPort,
    PublicPort,
    PublicHttpRoute,
    PublicTlsRoute,
}

#[derive(Serialize, Deserialize, Clone, Debug, JsonSchema)]
pub struct DynamicFlag {
    pub env: Option<EnvFlag>,
    pub content: Option<ContentFlag>,
    pub executable: Option<ExecutableFlag>,
}

#[derive(Serialize, Deserialize, Clone, Debug, JsonSchema)]
pub struct EnvFlag {
    pub name: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, JsonSchema)]
pub struct ContentFlag {
    pub path: String,
    pub mode: Option<u32>,
}

#[derive(Serialize, Deserialize, Clone, Debug, JsonSchema)]
pub struct ExecutableFlag {
    pub path: String,
    pub mode: Option<u32>,
}

#[derive(Serialize, Deserialize, Clone, Debug, JsonSchema)]
pub struct ResourceSpec {
    pub cpu: Option<String>,
    pub memory: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct AttachmentSpec {
    pub file_name: String,
    pub download_url: Option<String>,
    pub download_image: Option<String>,
    pub download_image_pull_secret: Option<String>,
    pub download_image_insecure: Option<bool>,
}
