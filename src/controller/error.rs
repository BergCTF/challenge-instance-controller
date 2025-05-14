#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("Kube API error: {0}")]
    KubeError(#[from] kube::Error),
    
    #[error("Serialization error: {0}")]
    SerializationError(#[from] serde_json::Error),
}

pub type Result<T, E = Error> = std::result::Result<T, E>;
