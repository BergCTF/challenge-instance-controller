use crate::{
    crds::ExecutableFlag,
    error::{Error, Result},
};
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

/// Generate a minimal x86_64 ELF executable that outputs the flag to stdout
///
/// This creates a statically-linked ELF binary that:
/// 1. Writes the flag to stdout using the write syscall (syscall 1)
/// 2. Exits with code 0 using the exit syscall (syscall 60)
///
/// The ELF format:
/// - ELF Header (64 bytes)
/// - Program Header for loadable segment (56 bytes)
/// - Code section with syscall instructions
/// - Data section with the flag string
pub fn generate_elf_executable(flag: &str) -> Result<Vec<u8>> {
    let flag_bytes = flag.as_bytes();
    let flag_len = flag_bytes.len();

    // Memory layout:
    // 0x400000: ELF header + program header
    // 0x400078: code section (_start)
    // 0x4000XX: data section (flag string)

    const BASE_ADDR: u64 = 0x400000;
    const CODE_OFFSET: u64 = 0x78; // After ELF header (64) + program header (56) = 120 = 0x78

    // Calculate offsets
    let code_addr = BASE_ADDR + CODE_OFFSET;
    let code_len = 45;
    let data_offset = CODE_OFFSET + code_len;
    let data_addr = BASE_ADDR + data_offset;

    let mut elf = Vec::new();

    // ELF Header (64 bytes)
    elf.extend_from_slice(&[
        // e_ident
        0x7f, 0x45, 0x4c, 0x46, // ELF magic number
        0x02, // 64-bit
        0x01, // Little endian
        0x01, // ELF version 1
        0x00, // System V ABI
        0x00, 0x00, 0x00, 0x00, // ABI version + padding
        0x00, 0x00, 0x00, 0x00, // padding
    ]);
    elf.extend_from_slice(&[
        0x02, 0x00, // e_type: ET_EXEC (executable)
        0x3e, 0x00, // e_machine: x86-64
        0x01, 0x00, 0x00, 0x00, // e_version: 1
    ]);
    elf.extend_from_slice(&code_addr.to_le_bytes()); // e_entry: entry point
    elf.extend_from_slice(&[0x40, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00]); // e_phoff: program header offset (64)
    elf.extend_from_slice(&[0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00]); // e_shoff: section header offset (none)
    elf.extend_from_slice(&[0x00, 0x00, 0x00, 0x00]); // e_flags
    elf.extend_from_slice(&[0x40, 0x00]); // e_ehsize: ELF header size (64)
    elf.extend_from_slice(&[0x38, 0x00]); // e_phentsize: program header entry size (56)
    elf.extend_from_slice(&[0x01, 0x00]); // e_phnum: 1 program header
    elf.extend_from_slice(&[0x00, 0x00]); // e_shentsize: section header entry size (0)
    elf.extend_from_slice(&[0x00, 0x00]); // e_shnum: 0 section headers
    elf.extend_from_slice(&[0x00, 0x00]); // e_shstrndx: 0

    // Program Header (56 bytes) - PT_LOAD segment
    let file_size = data_offset + flag_len as u64;
    let mem_size = file_size;

    elf.extend_from_slice(&[0x01, 0x00, 0x00, 0x00]); // p_type: PT_LOAD
    elf.extend_from_slice(&[0x05, 0x00, 0x00, 0x00]); // p_flags: PF_R | PF_X (readable + executable)
    elf.extend_from_slice(&[0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00]); // p_offset: 0
    elf.extend_from_slice(&BASE_ADDR.to_le_bytes()); // p_vaddr
    elf.extend_from_slice(&BASE_ADDR.to_le_bytes()); // p_paddr
    elf.extend_from_slice(&file_size.to_le_bytes()); // p_filesz
    elf.extend_from_slice(&mem_size.to_le_bytes()); // p_memsz
    elf.extend_from_slice(&[0x00, 0x10, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00]); // p_align: 0x1000 (4KB)

    // Code section - x86_64 assembly
    // _start:
    //   mov rax, 1          ; syscall: write
    //   mov rdi, 1          ; fd: stdout
    //   mov rsi, <data_addr> ; buf: flag address
    //   mov rdx, <flag_len> ; count: flag length
    //   syscall
    //   mov rax, 60         ; syscall: exit
    //   xor rdi, rdi        ; status: 0
    //   syscall

    let header_len = elf.len();

    elf.extend_from_slice(&[
        0x48, 0xc7, 0xc0, 0x01, 0x00, 0x00, 0x00, // mov rax, 1
        0x48, 0xc7, 0xc7, 0x01, 0x00, 0x00, 0x00, // mov rdi, 1
    ]);

    // mov rsi, <data_addr> - movabs rsi, <imm64>
    elf.extend_from_slice(&[0x48, 0xbe]);
    elf.extend_from_slice(&data_addr.to_le_bytes());

    // mov rdx, <flag_len>
    elf.extend_from_slice(&[0x48, 0xc7, 0xc2]);
    elf.extend_from_slice(&(flag_len as u32).to_le_bytes());

    elf.extend_from_slice(&[
        0x0f, 0x05, // syscall
        0x48, 0xc7, 0xc0, 0x3c, 0x00, 0x00, 0x00, // mov rax, 60 (exit)
        0x48, 0x31, 0xff, // xor rdi, rdi
        0x0f, 0x05, // syscall
    ]);

    assert_eq!(header_len + code_len as usize, elf.len());

    // Data section - the flag string
    elf.extend_from_slice(flag_bytes);

    Ok(elf)
}

#[cfg(test)]
mod tests {
    use std::{
        fs::{File, Permissions},
        io::Write,
        os::{self, unix::fs::PermissionsExt},
        path::Path,
        process::Command,
    };

    use super::*;

    #[test]
    fn test_generate_elf_executable() {
        let flag = "flag{test_flag}";
        let elf = generate_elf_executable(flag).unwrap();

        // Verify ELF magic
        assert_eq!(&elf[0..4], &[0x7f, 0x45, 0x4c, 0x46]);

        // Verify it's 64-bit
        assert_eq!(elf[4], 0x02);

        // Verify it's little endian
        assert_eq!(elf[5], 0x01);

        // Verify executable type
        assert_eq!(&elf[16..18], &[0x02, 0x00]);

        // Verify x86-64 machine type
        assert_eq!(&elf[18..20], &[0x3e, 0x00]);
    }

    #[test]
    fn test_generate_elf_various_lengths() {
        // Test with different flag lengths
        let flags = vec![
            "flag{a}",
            "flag{short}",
            "flag{this_is_a_longer_flag_for_testing}",
            "flag{🚩}", // Unicode
        ];

        for flag in flags {
            let elf = generate_elf_executable(flag).unwrap();
            assert!(elf.len() > 120); // Should have at least header + program header + code
            assert!(elf.len() >= 120 + flag.len()); // Should contain the flag
        }
    }

    #[test]
    fn test_run_elf_executable() {
        let flag = "flag{uwu_awa_owo}";
        let elf = generate_elf_executable(flag).unwrap();

        let path = Path::new("/dev/shm/elf");
        {
            let mut file = File::create(&path).unwrap();
            file.set_permissions(Permissions::from_mode(0o777)).unwrap();
            file.write_all(&elf).unwrap();
            file.flush().unwrap();
        }

        let result = Command::new(&path).output().unwrap();
        // std::fs::remove_file(&path).unwrap();
        assert!(flag == String::from_utf8(result.stdout).unwrap());
    }
}
