use kernel_core::{BootInfo, FramebufferInfo, MemoryKind, MemoryRegion};
use limine::memory_map::{Entry, EntryType};
use limine::response::MemoryMapResponse;

const MAX_MEMORY_REGIONS: usize = 128;

static mut MEMORY_REGIONS: [MemoryRegion; MAX_MEMORY_REGIONS] = [MemoryRegion {
    start: 0,
    end: 0,
    kind: MemoryKind::Reserved,
}; MAX_MEMORY_REGIONS];
static mut MEMORY_REGION_COUNT: usize = 0;

/// Builds a BootInfo structure from Limine memory map and kernel metadata.
pub fn build_boot_info(
    memory_map: &MemoryMapResponse,
    kernel_start: u64,
    kernel_end: u64,
    initramfs: Option<(u64, u64)>,
    framebuffer: Option<FramebufferInfo>,
) -> BootInfo<'static> {
    let mut count = 0usize;
    for entry in memory_map.entries() {
        if count >= MAX_MEMORY_REGIONS {
            break;
        }
        let region = map_entry(entry);
        unsafe {
            MEMORY_REGIONS[count] = region;
        }
        count += 1;
    }

    unsafe {
        MEMORY_REGION_COUNT = count;
        BootInfo {
            memory_map: core::slice::from_raw_parts(
                core::ptr::addr_of!(MEMORY_REGIONS) as *const MemoryRegion,
                MEMORY_REGION_COUNT,
            ),
            kernel_start,
            kernel_end,
            initramfs,
            dtb_ptr: None,
            framebuffer,
        }
    }
}

fn map_entry(entry: &Entry) -> MemoryRegion {
    let kind = match entry.entry_type {
        EntryType::USABLE | EntryType::BOOTLOADER_RECLAIMABLE => MemoryKind::Usable,
        EntryType::FRAMEBUFFER => MemoryKind::Reserved,
        _ => MemoryKind::Reserved,
    };
    MemoryRegion {
        start: entry.base,
        end: entry.base.saturating_add(entry.length),
        kind,
    }
}
