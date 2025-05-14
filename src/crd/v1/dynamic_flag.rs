use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use crate::crd::v1::dynamic_content_flag::DynamicContentFlag;
use crate::crd::v1::dynamic_env_flag::DynamicEnvFlag;
use crate::crd::v1::dynamic_executable_flag::DynamicExecutableFlag;

#[derive(Debug, Serialize, Deserialize, Clone, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct DynamicFlag {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub env: Option<DynamicEnvFlag>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<DynamicContentFlag>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub executable: Option<DynamicExecutableFlag>,
}