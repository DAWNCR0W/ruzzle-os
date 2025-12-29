#![no_std]

use hal::{Errno, PageFlags, PagingOps, PagingRoot, PhysAddr, VirtAddr};

/// Initializes AArch64 CPU state (minimal stub).
pub fn init() {}

/// Enables interrupts (stub; real mask setup is TBD).
pub fn enable_interrupts() {}

/// Busy-loop using `wfe`.
pub fn halt_loop() -> ! {
    loop {
        unsafe {
            core::arch::asm!("wfe");
        }
    }
}

/// AArch64 paging operations (stub implementation).
pub struct AArch64Paging;

impl PagingOps for AArch64Paging {
    fn map(
        &mut self,
        _root: PagingRoot,
        _va: VirtAddr,
        _pa: PhysAddr,
        _flags: PageFlags,
    ) -> Result<(), Errno> {
        Err(Errno::Unimplemented)
    }

    fn unmap(&mut self, _root: PagingRoot, _va: VirtAddr) -> Result<(), Errno> {
        Err(Errno::Unimplemented)
    }

    fn switch_as(&self, _root: PagingRoot) {}
}
