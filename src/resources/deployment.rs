use crate::{
    crds::{Challenge, ChallengeInstance, ChallengeInstanceClass, ContainerSpec},
    error::Result,
    flag,
    reconciler::Context,
    resources::labels,
};
use k8s_openapi::{
    api::{
        apps::v1::{Deployment, DeploymentSpec},
        core::v1::{
            Capabilities, Container, EnvVar, Pod, PodSpec, PodTemplateSpec, ResourceRequirements,
            SecurityContext,
        },
    },
    apimachinery::pkg::{api::resource::Quantity, apis::meta::v1::LabelSelector},
};
use kube::{
    api::{Api, ListParams, PostParams},
    Client,
};
use std::collections::BTreeMap;
use tracing::info;

pub async fn create(
    instance: &ChallengeInstance,
    challenge: &Challenge,
    container_spec: &ContainerSpec,
    namespace: &str,
    class: &ChallengeInstanceClass,
    ctx: &Context,
) -> Result<()> {
    let api: Api<Deployment> = Api::namespaced(ctx.client.clone(), namespace);

    let deployment = build_deployment(instance, challenge, container_spec, namespace, class, ctx)?;

    match api.create(&PostParams::default(), &deployment).await {
        Ok(_) => {
            info!(
                "Created deployment {} in {}",
                container_spec.hostname, namespace
            );
            Ok(())
        }
        Err(kube::Error::Api(ae)) if ae.code == 409 => {
            info!("Deployment {} already exists", container_spec.hostname);
            Ok(())
        }
        Err(e) => Err(e.into()),
    }
}

fn build_deployment(
    instance: &ChallengeInstance,
    challenge: &Challenge,
    container_spec: &ContainerSpec,
    namespace: &str,
    class: &ChallengeInstanceClass,
    _ctx: &Context,
) -> Result<Deployment> {
    let container_name = &container_spec.hostname;

    // Build environment variables
    let mut env_vars = vec![];

    // From container.environment
    for (key, value) in &container_spec.environment {
        env_vars.push(EnvVar {
            name: key.clone(),
            value: Some(value.clone()),
            ..Default::default()
        });
    }

    // Add instance metadata
    env_vars.push(EnvVar {
        name: "CHALLENGE_NAMESPACE".to_string(),
        value: Some(namespace.to_string()),
        ..Default::default()
    });

    // TODO: verify we push service endpoints here

    // Add flag if env mode
    if let Some(ref dynamic_flag) = container_spec.dynamic_flag {
        if let Some(ref env_flag) = dynamic_flag.env {
            env_vars.push(EnvVar {
                name: env_flag.name.clone(),
                value: Some(instance.spec.flag.clone()),
                ..Default::default()
            });
        }
    }

    // Build volumes and mounts for content/executable flags
    let mut volumes = vec![];
    let mut volume_mounts = vec![];

    if let Some(ref dynamic_flag) = container_spec.dynamic_flag {
        if let Some(ref content) = dynamic_flag.content {
            let (volume, mount) = flag::content::build_volume_mount(content, &instance.spec.flag)?;
            volumes.push(volume);
            volume_mounts.push(mount);
        }

        if let Some(ref executable) = dynamic_flag.executable {
            let (volume, mount) =
                flag::executable::build_volume_mount(executable, &instance.spec.flag)?;
            volumes.push(volume);
            volume_mounts.push(mount);
        }
    }

    // Build resource requirements
    let resources = build_resources(container_spec, class);

    // Build security context
    let security_context = build_security_context(container_spec);

    // Build container
    let container = Container {
        name: container_name.clone(),
        image: Some(container_spec.image.clone()),
        image_pull_policy: class.spec.image_pull.as_ref().map(|ip| ip.policy.clone()),
        env: if env_vars.is_empty() {
            None
        } else {
            Some(env_vars)
        },
        // TODO: where are my ports
        volume_mounts: if volume_mounts.is_empty() {
            None
        } else {
            Some(volume_mounts)
        },
        resources: Some(resources),
        readiness_probe: container_spec
            .readiness_probe
            .as_ref()
            .and_then(|v| serde_json::from_value(v.clone()).ok()),
        liveness_probe: container_spec
            .liveness_probe
            .as_ref()
            .and_then(|v| serde_json::from_value(v.clone()).ok()),
        security_context: Some(security_context),
        ..Default::default()
    };

    // Build pod annotations
    let mut pod_annotations = BTreeMap::new();
    if let Some(ref egress) = container_spec.egress_bandwidth {
        pod_annotations.insert("kubernetes.io/egress-bandwidth".to_string(), egress.clone());
    }
    if let Some(ref ingress) = container_spec.ingress_bandwidth {
        pod_annotations.insert(
            "kubernetes.io/ingress-bandwidth".to_string(),
            ingress.clone(),
        );
    }
    pod_annotations.insert(
        "cluster-autoscaler.kubernetes.io/safe-to-evict".to_string(),
        "false".to_string(),
    );

    // Build pod template
    let pod_template = PodTemplateSpec {
        metadata: Some(kube::api::ObjectMeta {
            labels: Some(labels::pod_labels(instance, challenge, container_spec)),
            annotations: if pod_annotations.is_empty() {
                None
            } else {
                Some(pod_annotations)
            },
            ..Default::default()
        }),
        spec: Some(PodSpec {
            hostname: Some(container_name.clone()),
            containers: vec![container],
            volumes: if volumes.is_empty() {
                None
            } else {
                Some(volumes)
            },
            // TODO: verify image pull secrets
            runtime_class_name: container_spec.runtime_class_name.clone().or_else(|| {
                class
                    .spec
                    .security
                    .as_ref()
                    .and_then(|s| s.runtime_class_name.clone())
            }),
            enable_service_links: Some(false),
            automount_service_account_token: Some(false),
            termination_grace_period_seconds: Some(0),
            ..Default::default()
        }),
    };

    Ok(Deployment {
        metadata: kube::api::ObjectMeta {
            name: Some(container_name.clone()),
            namespace: Some(namespace.to_string()),
            labels: Some(labels::resource_labels(instance, challenge)),
            ..Default::default()
        },
        spec: Some(DeploymentSpec {
            replicas: Some(1),
            selector: LabelSelector {
                match_labels: Some(labels::pod_selector_labels(container_spec)),
                ..Default::default()
            },
            template: pod_template,
            ..Default::default()
        }),
        ..Default::default()
    })
}

fn build_resources(
    container_spec: &ContainerSpec,
    class: &ChallengeInstanceClass,
) -> ResourceRequirements {
    let mut limits = BTreeMap::new();
    let mut requests = BTreeMap::new();

    // Get defaults from class
    let default_cpu_limit = class
        .spec
        .default_resources
        .as_ref()
        .and_then(|r| r.cpu_limit.clone())
        .unwrap_or_else(|| "1000m".to_string());
    let default_cpu_request = class
        .spec
        .default_resources
        .as_ref()
        .and_then(|r| r.cpu_request.clone())
        .unwrap_or_else(|| "100m".to_string());
    let default_memory_limit = class
        .spec
        .default_resources
        .as_ref()
        .and_then(|r| r.memory_limit.clone())
        .unwrap_or_else(|| "512Mi".to_string());
    let default_memory_request = class
        .spec
        .default_resources
        .as_ref()
        .and_then(|r| r.memory_request.clone())
        .unwrap_or_else(|| "128Mi".to_string());

    // CPU
    let cpu_limit = container_spec
        .resource_limits
        .as_ref()
        .and_then(|r| r.cpu.clone())
        .unwrap_or(default_cpu_limit);
    let cpu_request = container_spec
        .resource_requests
        .as_ref()
        .and_then(|r| r.cpu.clone())
        .unwrap_or(default_cpu_request);

    limits.insert("cpu".to_string(), Quantity(cpu_limit));
    requests.insert("cpu".to_string(), Quantity(cpu_request));

    // Memory
    let memory_limit = container_spec
        .resource_limits
        .as_ref()
        .and_then(|r| r.memory.clone())
        .unwrap_or(default_memory_limit);
    let memory_request = container_spec
        .resource_requests
        .as_ref()
        .and_then(|r| r.memory.clone())
        .unwrap_or(default_memory_request);

    limits.insert("memory".to_string(), Quantity(memory_limit));
    requests.insert("memory".to_string(), Quantity(memory_request));

    ResourceRequirements {
        limits: Some(limits),
        requests: Some(requests),
        ..Default::default()
    }
}

fn build_security_context(container_spec: &ContainerSpec) -> SecurityContext {
    let capabilities_to_add = container_spec.additional_capabilities.clone();
    let mut capabilities_to_drop = vec![];

    // Drop DAC_OVERRIDE if executable flag mode
    if let Some(ref dynamic_flag) = container_spec.dynamic_flag {
        if dynamic_flag.executable.is_some() {
            capabilities_to_drop.push("DAC_OVERRIDE".to_string());
        }
    }

    SecurityContext {
        privileged: Some(false),
        allow_privilege_escalation: Some(true),
        capabilities: Some(Capabilities {
            add: if capabilities_to_add.is_empty() {
                None
            } else {
                Some(capabilities_to_add)
            },
            drop: if capabilities_to_drop.is_empty() {
                None
            } else {
                Some(capabilities_to_drop)
            },
        }),
        ..Default::default()
    }
}

pub async fn check_pods_ready(client: &Client, namespace: &str) -> Result<bool> {
    let pods: Api<Pod> = Api::namespaced(client.clone(), namespace);

    let lp = ListParams::default().labels("app.kubernetes.io/managed-by=berg");

    let pod_list = pods.list(&lp).await?;

    if pod_list.items.is_empty() {
        return Ok(false);
    }

    for pod in pod_list.items {
        if let Some(status) = pod.status {
            // Check phase
            if status.phase.as_deref() != Some("Running") {
                return Ok(false);
            }

            // Check conditions
            if let Some(conditions) = status.conditions {
                let ready = conditions
                    .iter()
                    .find(|c| c.type_ == "Ready")
                    .map(|c| c.status == "True")
                    .unwrap_or(false);

                if !ready {
                    return Ok(false);
                }
            } else {
                return Ok(false);
            }
        } else {
            return Ok(false);
        }
    }

    Ok(true)
}

pub async fn check_pods_healthy(client: &Client, namespace: &str) -> Result<bool> {
    // For now, same as check_pods_ready
    // In production, this would check liveness probes and other health indicators
    check_pods_ready(client, namespace).await
}
