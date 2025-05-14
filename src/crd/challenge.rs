use kube::CustomResource;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(CustomResource, Debug, Serialize, Deserialize, Default, Clone, JsonSchema)]
#[kube(
    group = "berg.norelect.ch",
    version = "v1",
    kind = "Challenge",
    namespaced
)]
#[serde(rename_all = "camelCase")]
pub struct ChallengeSpec {
    pub templates: Vec<Value>,
}
