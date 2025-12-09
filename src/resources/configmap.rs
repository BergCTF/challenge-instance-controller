use crate::{
    crds::{ChallengeInstance, ContainerSpec, DynamicFlag},
    error::{Error, Result},
    reconciler::Context,
};
use k8s_openapi::api::core::v1::ConfigMap;
use kube::api::{Api, PostParams};
use std::collections::BTreeMap;
use tracing::info;

pub async fn create_flag_configmap(
    instance: &ChallengeInstance,
    _container: &ContainerSpec,
    dynamic_flag: &DynamicFlag,
    namespace: &str,
    ctx: &Context,
) -> Result<()> {
    let api: Api<ConfigMap> = Api::namespaced(ctx.client.clone(), namespace);

    // Create ConfigMap for content flag
    if let Some(ref _content) = dynamic_flag.content {
        let flag_content = format!("{}\n", instance.spec.flag);

        let mut data = BTreeMap::new();
        data.insert("content".to_string(), flag_content);

        let cm = ConfigMap {
            metadata: kube::api::ObjectMeta {
                name: Some("flag-content".to_string()),
                namespace: Some(namespace.to_string()),
                labels: Some({
                    let mut labels = BTreeMap::new();
                    labels.insert("app.kubernetes.io/managed-by".to_string(), "berg".to_string());
                    labels.insert("app.kubernetes.io/component".to_string(), "flag-content".to_string());
                    labels
                }),
                ..Default::default()
            },
            data: Some(data),
            ..Default::default()
        };

        match api.create(&PostParams::default(), &cm).await {
            Ok(_) => info!("Created flag content ConfigMap in {}", namespace),
            Err(kube::Error::Api(ae)) if ae.code == 409 => {
                info!("Flag content ConfigMap already exists")
            }
            Err(e) => return Err(Error::from(e)),
        }
    }

    // Create ConfigMap for executable flag
    if let Some(ref _executable) = dynamic_flag.executable {
        // Generate minimal ELF executable that outputs the flag
        let elf_binary = crate::flag::executable::generate_elf_executable(&instance.spec.flag)?;

        let mut binary_data = BTreeMap::new();
        binary_data.insert(
            "executable".to_string(),
            k8s_openapi::ByteString(elf_binary),
        );

        let cm = ConfigMap {
            metadata: kube::api::ObjectMeta {
                name: Some("flag-executable".to_string()),
                namespace: Some(namespace.to_string()),
                labels: Some({
                    let mut labels = BTreeMap::new();
                    labels.insert("app.kubernetes.io/managed-by".to_string(), "berg".to_string());
                    labels.insert("app.kubernetes.io/component".to_string(), "flag-executable".to_string());
                    labels
                }),
                ..Default::default()
            },
            binary_data: Some(binary_data),
            ..Default::default()
        };

        match api.create(&PostParams::default(), &cm).await {
            Ok(_) => info!("Created flag executable ConfigMap in {}", namespace),
            Err(kube::Error::Api(ae)) if ae.code == 409 => {
                info!("Flag executable ConfigMap already exists")
            }
            Err(e) => return Err(Error::from(e)),
        }
    }

    Ok(())
}
