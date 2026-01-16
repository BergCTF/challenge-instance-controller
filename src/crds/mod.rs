pub mod challenge;
pub mod challenge_instance;
pub mod challenge_instance_class;
pub mod cilium;
pub mod gateway;

// Re-export types
pub use challenge::{
    Challenge, ChallengeSpec, ContainerSpec, ContentFlag, DynamicFlag, DynamicFlagMode, EnvFlag,
    ExecutableFlag, PortSpec, PortType, ResourceSpec,
};
pub use challenge_instance::{
    ChallengeInstance, ChallengeInstanceSpec, ChallengeInstanceStatus, ChallengeRef, Condition,
    ConditionStatus, Phase, ServiceEndpoint, TerminationReason,
};
pub use challenge_instance_class::{
    ChallengeInstanceClass, ChallengeInstanceClassSpec, GatewayConfig, ImagePullConfig,
    NetworkConfig, ResourceDefaults, SecurityConfig,
};
pub use cilium::{
    CiliumDnsRule, CiliumEgressRule, CiliumFQDNRule, CiliumL7Rule, CiliumNetworkPolicy,
    CiliumNetworkPolicySpec, CiliumPortProtocol, CiliumPortRule,
};
pub use gateway::{
    BackendRef, HTTPBackendRef, HTTPRoute, HTTPRouteRule, HTTPRouteSpec, ParentReference, TLSRoute,
    TLSRouteRule, TLSRouteSpec,
};
