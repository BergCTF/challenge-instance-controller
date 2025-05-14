use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum ChallengePortType {
    InternalPort,
    PublicPort,
    PublicHttpRoute,
    PublicTlsRoute,
}