use kube::CustomResource;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::crd::v1::challenge_attachment::ChallengeAttachment;
use crate::crd::v1::challenge_container::ChallengeContainer;

#[derive(Debug, Serialize, Deserialize, Clone, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ChallengeSpec {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub display_name: Option<String>,
    #[serde(default = "default_string")]
    pub author: String,
    #[serde(default = "default_string")]
    pub description: String,
    #[serde(default = "default_string")]
    pub flag: String,
    #[serde(default = "default_flag_format")]
    pub flag_format: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hide_until: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub static_value: Option<i32>,
    #[serde(default = "default_string")]
    pub difficulty: String,
    #[serde(default = "default_bool")]
    pub allow_outbound_traffic: bool,
    #[serde(default = "default_vec_string")]
    pub categories: Vec<String>,
    #[serde(default = "default_vec_string")]
    pub tags: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub event: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub containers: Option<Vec<ChallengeContainer>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub attachments: Option<Vec<ChallengeAttachment>>,
}

impl ChallengeSpec {
    pub fn supports_dynamic_flags(&self) -> bool {
        self.containers.as_ref().map_or(false, |containers| {
            containers.iter().any(|c| c.dynamic_flag.is_some())
        })
    }
}

#[derive(CustomResource, Debug, Serialize, Deserialize, Clone, JsonSchema)]
#[kube(
    group = "berg.norelect.ch",
    version = "v1",
    kind = "Challenge",
    plural = "challenges",
    namespaced
)]
#[serde(rename_all = "camelCase")]
pub struct BergChallenge {
    pub spec: ChallengeSpec,
}

fn default_string() -> String {
    String::new()
}

fn default_flag_format() -> String {
    "flag{...}".to_string()
}

fn default_bool() -> bool {
    false
}

fn default_vec_string() -> Vec<String> {
    Vec::new()
}