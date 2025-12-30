use hal::PhysAddr;

/// Boot information passed from platform-specific code.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BootInfo<'a> {
    pub memory_map: &'a [MemoryRegion],
    pub kernel_start: PhysAddr,
    pub kernel_end: PhysAddr,
    pub initramfs: Option<(PhysAddr, PhysAddr)>,
    pub dtb_ptr: Option<PhysAddr>,
    pub framebuffer: Option<FramebufferInfo>,
}

/// Describes a contiguous physical memory region.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MemoryRegion {
    pub start: PhysAddr,
    pub end: PhysAddr,
    pub kind: MemoryKind,
}

/// Enumerates the physical memory region types.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemoryKind {
    Usable,
    Reserved,
    Mmio,
}

/// Describes a linear framebuffer provided by the bootloader.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FramebufferInfo {
    pub addr: PhysAddr,
    pub width: u32,
    pub height: u32,
    pub pitch: u32,
    pub bpp: u16,
    pub red_mask_size: u8,
    pub red_mask_shift: u8,
    pub green_mask_size: u8,
    pub green_mask_shift: u8,
    pub blue_mask_size: u8,
    pub blue_mask_shift: u8,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn boot_info_holds_memory_map() {
        let regions = [MemoryRegion {
            start: 0x1000,
            end: 0x2000,
            kind: MemoryKind::Usable,
        }];
        let info = BootInfo {
            memory_map: &regions,
            kernel_start: 0x0,
            kernel_end: 0x1000,
            initramfs: None,
            dtb_ptr: None,
            framebuffer: None,
        };
        assert_eq!(info.memory_map.len(), 1);
        assert_eq!(info.memory_map[0].kind, MemoryKind::Usable);
    }
}
