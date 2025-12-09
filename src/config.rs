use crate::error::{Error, Result};
use std::env;

#[derive(Clone, Debug)]
pub struct ControllerConfig {
    // Challenge configuration
    pub challenge_namespace: String,
    pub challenge_domain: String,
    pub challenge_http_port: u16,
    pub challenge_tls_port: u16,

    // Gateway configuration
    pub gateway_name: String,
    pub gateway_namespace: String,
    pub challenge_http_listener_name: String,
    pub challenge_tls_listener_name: String,

    // Instance defaults
    pub default_timeout: String,
    pub default_cpu_limit: String,
    pub default_cpu_request: String,
    pub default_memory_limit: String,
    pub default_memory_request: String,
    pub default_egress_bandwidth: String,
    pub default_ingress_bandwidth: String,

    // Image configuration
    pub image_pull_policy: String,
    pub pull_secret_name: Option<String>,
    pub default_runtime_class_name: Option<String>,

    // Features
    pub additional_headless_service: bool,
}

impl ControllerConfig {
    pub fn from_env() -> Result<Self> {
        Ok(Self {
            challenge_namespace: env::var("CHALLENGE_NAMESPACE")
                .unwrap_or_else(|_| "berg".to_string()),
            challenge_domain: env::var("CHALLENGE_DOMAIN")
                .map_err(|_| Error::ConfigError("CHALLENGE_DOMAIN required".into()))?,
            challenge_http_port: env::var("CHALLENGE_HTTP_PORT")
                .unwrap_or_else(|_| "80".to_string())
                .parse()
                .map_err(|_| Error::ConfigError("Invalid CHALLENGE_HTTP_PORT".into()))?,
            challenge_tls_port: env::var("CHALLENGE_TLS_PORT")
                .unwrap_or_else(|_| "443".to_string())
                .parse()
                .map_err(|_| Error::ConfigError("Invalid CHALLENGE_TLS_PORT".into()))?,
            gateway_name: env::var("GATEWAY_NAME")
                .unwrap_or_else(|_| "berg-gateway".to_string()),
            gateway_namespace: env::var("GATEWAY_NAMESPACE")
                .unwrap_or_else(|_| "berg".to_string()),
            challenge_http_listener_name: env::var("CHALLENGE_HTTP_LISTENER_NAME")
                .unwrap_or_else(|_| "http".to_string()),
            challenge_tls_listener_name: env::var("CHALLENGE_TLS_LISTENER_NAME")
                .unwrap_or_else(|_| "tls".to_string()),
            default_timeout: env::var("CHALLENGE_INSTANCE_TIMEOUT")
                .unwrap_or_else(|_| "2h".to_string()),
            default_cpu_limit: env::var("CHALLENGE_CPU_LIMIT")
                .unwrap_or_else(|_| "1000m".to_string()),
            default_cpu_request: env::var("CHALLENGE_CPU_REQUEST")
                .unwrap_or_else(|_| "100m".to_string()),
            default_memory_limit: env::var("CHALLENGE_MEMORY_LIMIT")
                .unwrap_or_else(|_| "512Mi".to_string()),
            default_memory_request: env::var("CHALLENGE_MEMORY_REQUEST")
                .unwrap_or_else(|_| "128Mi".to_string()),
            default_egress_bandwidth: env::var("CHALLENGE_EGRESS_BANDWIDTH")
                .unwrap_or_else(|_| "10M".to_string()),
            default_ingress_bandwidth: env::var("CHALLENGE_INGRESS_BANDWIDTH")
                .unwrap_or_else(|_| "10M".to_string()),
            image_pull_policy: env::var("CHALLENGE_IMAGE_PULL_POLICY")
                .unwrap_or_else(|_| "IfNotPresent".to_string()),
            pull_secret_name: env::var("PULL_SECRET_NAME").ok(),
            default_runtime_class_name: env::var("CHALLENGE_RUNTIME_CLASS_NAME").ok(),
            additional_headless_service: env::var("CHALLENGE_ADDITIONAL_HEADLESS_SERVICE")
                .unwrap_or_else(|_| "false".to_string())
                .parse()
                .unwrap_or(false),
        })
    }
}
