pub mod challenge;
pub mod challenge_instance;
pub mod challenge_instance_class;
pub mod cilium;
pub mod gateway;

// Re-export types
pub use challenge::{Challenge, ChallengeSpec, ContainerSpec, DynamicFlag, DynamicFlagMode, EnvFlag, ContentFlag, ExecutableFlag, PortSpec, PortType, ResourceSpec};
pub use challenge_instance::{ChallengeInstance, ChallengeInstanceSpec, ChallengeInstanceStatus, ChallengeRef, Condition, ConditionStatus, Phase, ServiceEndpoint, TerminationReason};
pub use challenge_instance_class::{ChallengeInstanceClass, ChallengeInstanceClassSpec, GatewayConfig, ResourceDefaults, NetworkConfig, ImagePullConfig, SecurityConfig};
pub use cilium::{CiliumNetworkPolicy, CiliumNetworkPolicySpec, CiliumEgressRule, CiliumPortRule, CiliumPortProtocol, CiliumL7Rule, CiliumDnsRule, CiliumFQDNRule};
pub use gateway::{HTTPRoute, HTTPRouteSpec, HTTPRouteRule, HTTPBackendRef, TLSRoute, TLSRouteSpec, TLSRouteRule, BackendRef, ParentReference};
