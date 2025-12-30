#![no_std]

use arch_x86_64 as arch;
use kernel_core::{BootInfo, MemoryRegion};

/// Returns a placeholder BootInfo for the QEMU x86_64 platform.
pub fn boot_info() -> BootInfo<'static> {
    static REGIONS: [MemoryRegion; 0] = [];
    BootInfo {
        memory_map: &REGIONS,
        kernel_start: 0,
        kernel_end: 0,
        initramfs: None,
        dtb_ptr: None,
        framebuffer: None,
    }
}

/// Placeholder UART write for early boot logging.
pub fn uart_write(_byte: u8) {
    arch::serial_write_byte(_byte);
}

/// Placeholder timer tick handler for QEMU x86_64.
pub fn timer_tick() {
    // Timer tick handling is driven by the x86_64 PIT + PIC.
}

/// Placeholder IRQ acknowledgement routine.
pub fn acknowledge_irq(_irq: u32) {
    arch::acknowledge_irq(_irq as u8);
}

/// Initializes platform devices such as the serial port.
pub fn init() {
    arch::init_serial();
}
