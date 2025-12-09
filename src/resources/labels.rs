use crate::{crds::{Challenge, ChallengeInstance, ContainerSpec}, reconciler::Context};
use std::collections::BTreeMap;

/// Generate standard labels for all resources
pub fn common_labels(instance: &ChallengeInstance, _challenge: &Challenge) -> BTreeMap<String, String> {
    let mut labels = BTreeMap::new();
    labels.insert("app.kubernetes.io/managed-by".to_string(), "berg".to_string());
    labels.insert("app.kubernetes.io/component".to_string(), "challenge".to_string());
    labels.insert("berg.norelect.ch/challenge".to_string(), instance.spec.challenge_ref.name.clone());
    labels.insert("berg.norelect.ch/owner-id".to_string(), instance.spec.owner_id.clone());
    if let Some(ref status) = instance.status {
        if let Some(ref instance_id) = status.instance_id {
            labels.insert("berg.norelect.ch/instance-id".to_string(), instance_id.clone());
        }
    }
    labels
}

/// Generate labels for namespace
pub fn namespace_labels(instance: &ChallengeInstance, ctx: &Context) -> BTreeMap<String, String> {
    let mut labels = BTreeMap::new();
    labels.insert("app.kubernetes.io/managed-by".to_string(), "berg".to_string());
    labels.insert("app.kubernetes.io/component".to_string(), "challenge".to_string());
    labels.insert("berg.norelect.ch/challenge".to_string(), instance.spec.challenge_ref.name.clone());
    labels.insert("berg.norelect.ch/challenge-namespace".to_string(),
        instance.spec.challenge_ref.namespace.clone()
            .unwrap_or_else(|| ctx.config.challenge_namespace.clone()));
    labels.insert("berg.norelect.ch/owner-id".to_string(), instance.spec.owner_id.clone());
    if let Some(ref status) = instance.status {
        if let Some(ref instance_id) = status.instance_id {
            labels.insert("berg.norelect.ch/instance-id".to_string(), instance_id.clone());
        }
    }
    labels
}

/// Generate labels for pods
pub fn pod_labels(instance: &ChallengeInstance, challenge: &Challenge, container: &ContainerSpec) -> BTreeMap<String, String> {
    let mut labels = common_labels(instance, challenge);
    labels.insert("berg.norelect.ch/container".to_string(), container.hostname.clone());
    labels
}

/// Generate selector labels for pods
pub fn pod_selector_labels(container: &ContainerSpec) -> BTreeMap<String, String> {
    let mut labels = BTreeMap::new();
    labels.insert("app.kubernetes.io/managed-by".to_string(), "berg".to_string());
    labels.insert("app.kubernetes.io/component".to_string(), "challenge-pod".to_string());
    labels.insert("berg.norelect.ch/container".to_string(), container.hostname.clone());
    labels
}

/// Generate resource labels
pub fn resource_labels(instance: &ChallengeInstance, challenge: &Challenge) -> BTreeMap<String, String> {
    common_labels(instance, challenge)
}
