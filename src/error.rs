use thiserror::Error;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Error, Debug)]
pub enum Error {
    #[error("Kubernetes API error: {0}")]
    KubeError(#[from] kube::Error),

    #[error("Serialization error: {0}")]
    SerializationError(#[from] serde_json::Error),

    #[error("Challenge not found: {namespace}/{name}")]
    ChallengeNotFound { namespace: String, name: String },

    #[error("ChallengeInstanceClass not found: {name}")]
    InstanceClassNotFound { name: String },

    #[error("Flag validation failed: {0}")]
    FlagValidationError(String),

    #[error("Resource creation failed: {resource_type} - {reason}")]
    ResourceCreationError {
        resource_type: String,
        reason: String,
    },

    #[error("Timeout parsing error: {0}")]
    TimeoutParseError(String),

    #[error("Flag generation error: {0}")]
    FlagGenerationError(String),

    #[error("Invalid configuration: {0}")]
    ConfigError(String),

    #[error("Finalizer error: {0}")]
    FinalizerError(String),
}

impl Error {
    /// Determine if this error is retryable
    pub fn is_retryable(&self) -> bool {
        matches!(
            self,
            Error::KubeError(_) | Error::ResourceCreationError { .. }
        )
    }
}
