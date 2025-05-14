use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct DynamicContentFlag {
    pub path: String,
    #[serde(default = "default_mode")]
    pub mode: i32,
}

fn default_mode() -> i32 {
    292 // octal 444, r--r--r--
}