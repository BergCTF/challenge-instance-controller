use kube::CustomResource;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::date_time::DateTime;

/// ChallengeInstance is the primary resource managed by this controller
/// It is cluster scoped since it manages namespaces
/// In the future, it may be beneficial to expose a namespace scoped challenge instance to allow
/// individual challenge authors to instance their challenges
#[derive(CustomResource, Serialize, Deserialize, Clone, Debug, JsonSchema)]
#[kube(
    group = "berg.norelect.ch",
    version = "v1",
    kind = "ChallengeInstance",
    plural = "challengeinstances",
    singular = "challengeinstance",
    shortname = "ci",
    shortname = "instance",
    namespaced = false,
    status = "ChallengeInstanceStatus",
    printcolumn = r#"{"name":"Challenge", "type":"string", "jsonPath":".spec.challengeRef.name"}"#,
    printcolumn = r#"{"name":"Owner", "type":"string", "jsonPath":".spec.ownerId"}"#,
    printcolumn = r#"{"name":"Phase", "type":"string", "jsonPath":".status.phase"}"#,
    printcolumn = r#"{"name":"Namespace", "type":"string", "jsonPath":".status.namespace"}"#,
    printcolumn = r#"{"name":"Age", "type":"date", "jsonPath":".metadata.creationTimestamp"}"#,
    printcolumn = r#"{"name":"Expires", "type":"date", "jsonPath":".status.expiresAt"}"#
)]
#[serde(rename_all = "camelCase")]
pub struct ChallengeInstanceSpec {
    /// Reference to the Challenge resource
    pub challenge_ref: ChallengeRef,

    /// UUID of the owner (player/team)
    #[schemars(regex(
        pattern = r"^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$"
    ))]
    pub owner_id: String,

    /// Pre-generated flag for this instance
    #[schemars(length(max = 1024))]
    pub flag: String,

    /// ChallengeInstanceClass to use for this instance
    /// If not specified, the default class will be used
    #[serde(skip_serializing_if = "Option::is_none")]
    pub instance_class: Option<String>,

    /// Duration after which instance auto-terminates (e.g., "2h", "30m")
    #[serde(default = "default_timeout")]
    #[schemars(regex(pattern = r"^([0-9]+h)?([0-9]+m)?([0-9]+s)?$"))]
    pub timeout: Option<String>,

    /// Reason for termination
    pub termination_reason: Option<TerminationReason>,
}

#[derive(Serialize, Deserialize, Clone, Debug, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ChallengeRef {
    pub name: String,
    pub namespace: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug, JsonSchema, PartialEq)]
pub enum TerminationReason {
    UserRequest,
    Timeout,
    AdminTermination,
}

#[derive(Serialize, Deserialize, Clone, Debug, JsonSchema, Default)]
#[serde(rename_all = "camelCase")]
pub struct ChallengeInstanceStatus {
    /// Generated UUID for this instance
    pub instance_id: Option<String>,

    /// Current lifecycle phase
    pub phase: Option<Phase>,

    /// Namespace containing instance resources
    pub namespace: Option<String>,

    /// Service endpoints
    #[serde(default)]
    pub services: Vec<ServiceEndpoint>,

    /// Timestamps (RFC3339 format)
    pub started_at: Option<DateTime>,
    pub ready_at: Option<DateTime>,
    pub terminated_at: Option<DateTime>,
    pub expires_at: Option<DateTime>,

    /// Status conditions
    #[serde(default)]
    pub conditions: Vec<Condition>,

    /// Last observed generation
    pub observed_generation: Option<i64>,
}

#[derive(Serialize, Deserialize, Clone, Debug, JsonSchema, PartialEq)]
pub enum Phase {
    Pending,
    Creating,
    Starting,
    Running,
    Terminating,
    Terminated,
    Failed,
}

#[derive(Serialize, Deserialize, Clone, Debug, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ServiceEndpoint {
    pub name: String,
    pub hostname: String,
    pub port: u16,
    pub protocol: String,
    pub app_protocol: Option<String>,
    pub tls: Option<bool>,
}

#[derive(Serialize, Deserialize, Clone, Debug, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct Condition {
    pub r#type: String,
    pub status: ConditionStatus,
    pub last_transition_time: Option<DateTime>,
    pub reason: Option<String>,
    pub message: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug, JsonSchema, PartialEq)]
pub enum ConditionStatus {
    True,
    False,
    Unknown,
}

fn default_timeout() -> Option<String> {
    Some("2h".to_string())
}
