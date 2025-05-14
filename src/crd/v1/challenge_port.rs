use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use crate::crd::v1::challenge_port_type::ChallengePortType;

#[derive(Debug, Serialize, Deserialize, Clone, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ChallengePort {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(default = "default_port")]
    pub port: i32,
    #[serde(default = "default_protocol")]
    pub protocol: String,
    pub app_protocol: String,
    #[serde(default = "default_port_type")]
    pub r#type: ChallengePortType,
}

fn default_port() -> i32 {
    80
}

fn default_protocol() -> String {
    "tcp".to_string()
}

fn default_port_type() -> ChallengePortType {
    ChallengePortType::InternalPort
}