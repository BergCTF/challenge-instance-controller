use crate::{crds::ExecutableFlag, error::{Error, Result}};
use k8s_openapi::api::core::v1::{ConfigMapVolumeSource, KeyToPath, Volume, VolumeMount};

/// Build volume and mount for executable flag
pub fn build_volume_mount(config: &ExecutableFlag, _flag: &str) -> Result<(Volume, VolumeMount)> {
    let path_with_entropy = crate::flag::entropy::substitute_entropy(&config.path);
    let filename = std::path::Path::new(&path_with_entropy)
        .file_name()
        .and_then(|n| n.to_str())
        .ok_or_else(|| Error::FlagGenerationError("Invalid path".into()))?
        .to_string();

    let volume = Volume {
        name: "flag-executable".to_string(),
        config_map: Some(ConfigMapVolumeSource {
            name: "flag-executable".to_string(),
            items: Some(vec![KeyToPath {
                key: "executable".to_string(),
                path: filename.clone(),
                mode: config.mode.map(|m| m as i32),
            }]),
            default_mode: config.mode.map(|m| m as i32).or(Some(0o555)),
            ..Default::default()
        }),
        ..Default::default()
    };

    let mount = VolumeMount {
        name: "flag-executable".to_string(),
        mount_path: path_with_entropy,
        sub_path: Some(filename),
        read_only: Some(true),
        ..Default::default()
    };

    Ok((volume, mount))
}

/// Generate ELF executable embedding the flag
/// TODO: Implement in Phase 2
pub fn generate_elf_executable(_flag: &str) -> Result<Vec<u8>> {
    Err(Error::FlagGenerationError(
        "ELF executable generation not yet implemented - deferred to Phase 2".into(),
    ))
}
