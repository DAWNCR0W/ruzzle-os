use kernel_core::{load_elf, parse_elf, CapSet, Capability, ElfLoader, Process};
use hal::{Errno, PageFlags};

/// Loads the init module from an initramfs image and returns its process record.
pub fn load_init_process(initramfs: &[u8]) -> Result<Process, Errno> {
    let entries = kernel_core::parse_initramfs(initramfs)?;
    let init_entry = entries
        .iter()
        .find(|entry| entry.name == "init")
        .ok_or(Errno::NotFound)?;

    let elf = parse_elf(&init_entry.data)?;
    let mut loader = NoopLoader;
    load_elf(&init_entry.data, &elf, &mut loader)?;

    let mut process = Process::new(1, 0);
    process.ctx.pc = elf.entry as usize;
    process.caps = init_caps();
    Ok(process)
}

/// Returns the baseline capabilities for the init process.
fn init_caps() -> CapSet {
    let mut caps = CapSet::empty();
    caps.insert(Capability::ProcessSpawn);
    caps
}

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
