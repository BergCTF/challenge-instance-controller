use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use serde_json::Value;
use crate::crd::v1::dynamic_flag::DynamicFlag;
use crate::crd::v1::challenge_port::ChallengePort;

#[derive(Debug, Serialize, Deserialize, Clone, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ChallengeContainer {
    pub hostname: String,
    pub image: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dynamic_flag: Option<DynamicFlag>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub environment: Option<HashMap<String, serde_json::Value>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resource_limits: Option<HashMap<String, String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub readiness_probe: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub runtime_class_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub egress_bandwidth: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ports: Option<Vec<ChallengePort>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub additional_capabilities: Option<Vec<String>>,
}