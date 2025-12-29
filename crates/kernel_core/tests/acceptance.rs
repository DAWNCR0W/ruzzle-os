use kernel_core::{
    build_initramfs, cap_transfer, endpoint_create, ipc_recv, ipc_send, parse_elf,
    parse_initramfs, validate_user_buffer, Capability, CapSet, ElfLoader, Errno, PageFlags,
    Process, Syscall, SyscallResult,
};

struct NoopLoader;

impl ElfLoader for NoopLoader {
    fn map(&mut self, _vaddr: u64, _mem_size: u64, _flags: PageFlags) -> Result<(), Errno> {
        Ok(())
    }

    fn copy(&mut self, _vaddr: u64, _data: &[u8]) -> Result<(), Errno> {
        Ok(())
    }

    fn zero(&mut self, _vaddr: u64, _size: u64) -> Result<(), Errno> {
        Ok(())
    }
}

fn build_test_elf() -> Vec<u8> {
    let mut image = vec![0u8; 0x200];
    image[..4].copy_from_slice(&[0x7F, b'E', b'L', b'F']);
    image[4] = 2;
    image[5] = 1;
    image[6] = 1;
    image[16..18].copy_from_slice(&2u16.to_le_bytes());
    image[18..20].copy_from_slice(&0x3Eu16.to_le_bytes());
    image[20..24].copy_from_slice(&1u32.to_le_bytes());
    image[24..32].copy_from_slice(&0x400000u64.to_le_bytes());
    image[32..40].copy_from_slice(&64u64.to_le_bytes());
    image[52..54].copy_from_slice(&64u16.to_le_bytes());
    image[54..56].copy_from_slice(&56u16.to_le_bytes());
    image[56..58].copy_from_slice(&1u16.to_le_bytes());

    let ph = 64;
    image[ph..ph + 4].copy_from_slice(&1u32.to_le_bytes());
    image[ph + 4..ph + 8].copy_from_slice(&0x5u32.to_le_bytes());
    image[ph + 8..ph + 16].copy_from_slice(&0x100u64.to_le_bytes());
    image[ph + 16..ph + 24].copy_from_slice(&0x400000u64.to_le_bytes());
    image[ph + 32..ph + 40].copy_from_slice(&4u64.to_le_bytes());
    image[ph + 40..ph + 48].copy_from_slice(&8u64.to_le_bytes());

    image[0x100..0x104].copy_from_slice(&[1, 2, 3, 4]);
    image
}

#[test]
fn acceptance_init_pipeline() {
    let init_elf = build_test_elf();
    let entries = vec![
        kernel_core::InitramfsEntry {
            name: "init".to_string(),
            data: init_elf.clone(),
        },
        kernel_core::InitramfsEntry {
            name: "console-service".to_string(),
            data: vec![0xCA, 0xFE],
        },
    ];

    let image = build_initramfs(&entries);
    let parsed = parse_initramfs(&image).expect("initramfs parse should succeed");
    let init_entry = parsed
        .iter()
        .find(|entry| entry.name == "init")
        .expect("init module should exist");

    let elf = parse_elf(&init_entry.data).expect("ELF parse should succeed");
    let mut loader = NoopLoader;
    kernel_core::load_elf(&init_entry.data, &elf, &mut loader)
        .expect("ELF load should succeed");
}

#[test]
fn acceptance_capability_enforcement() {
    let caps = CapSet::empty();
    let result = kernel_core::syscall::dispatch(Syscall::DebugLog, caps, None);
    assert_eq!(result, Err(Errno::NoPerm));

    let mut caps = CapSet::empty();
    caps.insert(Capability::ConsoleWrite);
    let result = kernel_core::syscall::dispatch(Syscall::DebugLog, caps, None);
    assert_eq!(result, Ok(SyscallResult::Unit));
}

#[test]
fn acceptance_user_memory_protection() {
    assert_eq!(validate_user_buffer(0x1000, 4), Ok(()));
    assert_eq!(validate_user_buffer(0xFFFF_8000_0000_0000, 4), Err(Errno::NoPerm));
}

#[test]
fn acceptance_ipc_cap_transfer() {
    let mut process = Process::new(1, 0);
    process.caps.insert(Capability::EndpointCreate);
    process.caps.insert(Capability::ConsoleWrite);

    let handle = endpoint_create(&mut process).expect("endpoint create should succeed");
    cap_transfer(&mut process, Capability::ConsoleWrite).expect("cap transfer should succeed");
    ipc_send(&mut process, handle, b"ok").expect("send should succeed");

    let mut buffer = [0u8; 8];
    let result = ipc_recv(&mut process, handle, &mut buffer).expect("recv should succeed");
    assert_eq!(result.cap, Some(Capability::ConsoleWrite));
}
