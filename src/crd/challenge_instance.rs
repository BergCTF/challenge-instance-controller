use kube::CustomResource;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(CustomResource, Debug, Serialize, Deserialize, Default, Clone, PartialEq, JsonSchema)]
#[kube(
    group = "berg.norelect.ch",
    version = "v1",
    kind = "ChallengeInstance",
    doc = "A Berg Challenge Instance",
    status = "ChallengeInstanceStatus",
    singular = "challengeinstance",
    plural = "challengeinstances",
    namespaced
)]
#[serde(rename_all = "camelCase")]
pub struct ChallengeInstanceSpec {
    challenge: String,
    dynamic_flag: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ChallengeInstanceStatus {
    namespace: String,
}
