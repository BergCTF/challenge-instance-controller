use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct DynamicExecutableFlag {
    pub path: String,
    #[serde(default = "default_mode")]
    pub mode: i32,
}

fn default_mode() -> i32 {
    73 // octal 111, --x--x--x
}