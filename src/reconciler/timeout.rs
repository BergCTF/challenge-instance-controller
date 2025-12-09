use crate::{
    crds::{ChallengeInstance, TerminationReason},
    error::{Error, Result},
};
use chrono::{DateTime, Duration, Utc};
use kube::{api::{Api, DeleteParams}, runtime::controller::Action, ResourceExt};
use std::sync::Arc;
use tracing::info;

use super::Context;

/// Check if an instance has expired
pub fn is_expired(instance: &ChallengeInstance) -> bool {
    if let Some(status) = &instance.status {
        if let Some(ref expires_at_str) = status.expires_at {
            if let Ok(expires_at) = DateTime::parse_from_rfc3339(expires_at_str) {
                return Utc::now() > expires_at.with_timezone(&Utc);
            }
        }
    }
    false
}

/// Calculate expiry time from a timeout string like "2h", "30m", "1h30m"
pub fn calculate_expiry(timeout_str: &str) -> Result<String> {
    let duration = parse_timeout(timeout_str)?;
    let expiry = Utc::now() + duration;
    Ok(expiry.to_rfc3339())
}

/// Parse a timeout string into a Duration
fn parse_timeout(timeout_str: &str) -> Result<Duration> {
    let mut total_seconds = 0i64;
    let mut current_num = String::new();

    for ch in timeout_str.chars() {
        if ch.is_ascii_digit() {
            current_num.push(ch);
        } else if !current_num.is_empty() {
            let num: i64 = current_num
                .parse()
                .map_err(|_| Error::TimeoutParseError(format!("Invalid number: {}", current_num)))?;

            match ch {
                'h' => total_seconds += num * 3600,
                'm' => total_seconds += num * 60,
                's' => total_seconds += num,
                _ => {
                    return Err(Error::TimeoutParseError(format!(
                        "Invalid time unit: {}",
                        ch
                    )))
                }
            }

            current_num.clear();
        }
    }

    if !current_num.is_empty() {
        return Err(Error::TimeoutParseError(
            "Timeout string must end with a unit (h/m/s)".to_string(),
        ));
    }

    Duration::try_seconds(total_seconds).ok_or_else(|| {
        Error::TimeoutParseError(format!("Invalid duration: {} seconds", total_seconds))
    })
}

/// Terminate an expired instance
pub async fn terminate_expired(
    instance: Arc<ChallengeInstance>,
    ctx: Arc<Context>,
) -> Result<Action> {
    info!("Instance {} has expired, terminating", instance.name_any());
    ctx.metrics.record_timeout();

    let ns = instance.namespace().unwrap();
    let api: Api<ChallengeInstance> = Api::namespaced(ctx.client.clone(), &ns);

    // Set termination reason and delete
    let patch = serde_json::json!({
        "spec": {
            "terminationReason": TerminationReason::Timeout
        }
    });

    api.patch(
        &instance.name_any(),
        &kube::api::PatchParams::default(),
        &kube::api::Patch::Merge(&patch),
    )
    .await?;

    // Delete the instance (will trigger finalizer)
    api.delete(&instance.name_any(), &DeleteParams::default())
        .await?;

    Ok(Action::await_change())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_timeout() {
        assert_eq!(parse_timeout("2h").unwrap(), Duration::try_hours(2).unwrap());
        assert_eq!(
            parse_timeout("30m").unwrap(),
            Duration::try_minutes(30).unwrap()
        );
        assert_eq!(
            parse_timeout("1h30m").unwrap(),
            Duration::try_minutes(90).unwrap()
        );
        assert_eq!(
            parse_timeout("1h30m15s").unwrap(),
            Duration::try_seconds(5415).unwrap()
        );
    }

    #[test]
    fn test_parse_timeout_invalid() {
        assert!(parse_timeout("invalid").is_err());
        assert!(parse_timeout("2x").is_err());
        assert!(parse_timeout("2").is_err());
    }
}
