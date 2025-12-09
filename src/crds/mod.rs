pub mod challenge;
pub mod challenge_instance;
pub mod cilium;

// Re-export types
pub use challenge::{Challenge, ChallengeSpec, ContainerSpec, DynamicFlag, DynamicFlagMode, EnvFlag, ContentFlag, ExecutableFlag, PortSpec, PortType, ResourceSpec};
pub use challenge_instance::{ChallengeInstance, ChallengeInstanceSpec, ChallengeInstanceStatus, ChallengeRef, Condition, ConditionStatus, Phase, ServiceEndpoint, TerminationReason};
pub use cilium::{CiliumNetworkPolicy, CiliumNetworkPolicySpec, CiliumEgressRule, CiliumPortRule, CiliumPortProtocol, CiliumL7Rule, CiliumDnsRule, CiliumFQDNRule};
