use kube::CustomResource;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(CustomResource, Debug, Serialize, Deserialize, Clone, JsonSchema)]
#[kube(
    group = "berg.norelect.ch",
    version = "v1",
    kind = "Page",
    plural = "pages",
    namespaced
)]
#[serde(rename_all = "camelCase")]
pub struct BergPage {
    pub spec: PageSpec,
}

#[derive(Debug, Serialize, Deserialize, Clone, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct PageSpec {
    #[serde(default = "default_string")]
    pub title: String,
    #[serde(default = "default_string")]
    pub path: String,
    #[serde(default = "default_index")]
    pub index: i32,
    #[serde(default = "default_string")]
    pub content: String,
}

fn default_string() -> String {
    String::new()
}

fn default_index() -> i32 {
    0
}