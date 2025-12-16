use crate::error::Result;
use std::env;

// TODO: use config crate here
#[derive(Clone, Debug)]
pub struct ControllerConfig {
    /// Default ChallengeInstanceClass to use if none specified
    pub default_instance_class: String,

    /// Default challenge namespace (fallback for Challenge lookups)
    pub challenge_namespace: String,

    /// Default timeout if not specified in instance or class
    pub default_timeout: String,

    /// Namespace prefix for challenge instance namespaces
    pub namespace_prefix: String,
}

impl ControllerConfig {
    pub fn from_env() -> Result<Self> {
        Ok(Self {
            default_instance_class: env::var("DEFAULT_INSTANCE_CLASS")
                .unwrap_or_else(|_| "default".to_string()),
            challenge_namespace: env::var("CHALLENGE_NAMESPACE")
                .unwrap_or_else(|_| "berg".to_string()),
            default_timeout: env::var("DEFAULT_TIMEOUT").unwrap_or_else(|_| "2h".to_string()),
            namespace_prefix: env::var("NAMESPACE_PREFIX").unwrap_or_else(|_| "ci".to_string()),
        })
    }
}
