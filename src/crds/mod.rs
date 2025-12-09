pub mod challenge;
pub mod challenge_instance;

// Re-export types
pub use challenge::{Challenge, ChallengeSpec, ContainerSpec, DynamicFlag, DynamicFlagMode, EnvFlag, ContentFlag, ExecutableFlag, PortSpec, PortType, ResourceSpec};
pub use challenge_instance::{ChallengeInstance, ChallengeInstanceSpec, ChallengeInstanceStatus, ChallengeRef, Condition, ConditionStatus, Phase, ServiceEndpoint, TerminationReason};
