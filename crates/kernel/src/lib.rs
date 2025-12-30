#![no_std]
#![feature(alloc_error_handler)]

extern crate alloc;

#[cfg(feature = "x86_64")]
use arch_x86_64 as arch;
#[cfg(feature = "aarch64")]
use arch_aarch64 as arch;

#[cfg(feature = "qemu_x86_64")]
use platform_qemu_x86_64 as platform;
#[cfg(feature = "qemu_virt")]
use platform_qemu_aarch64_virt as platform;

pub mod boot;
pub mod console;
#[cfg(feature = "x86_64")]
mod framebuffer;
#[cfg(feature = "x86_64")]
mod font;
pub mod smp;
pub mod allocator;
pub mod init;
pub mod shell;

use kernel_core::BootInfo;

/// Kernel entrypoint invoked by the bootloader.
pub fn entry(boot_info: BootInfo) -> ! {
    console::init_framebuffer(boot_info.framebuffer);
    allocator::init_heap();
    kprintln!("Ruzzle OS: kernel entry");
    #[cfg(feature = "x86_64")]
    arch::set_memory_offsets(
        boot_info.hhdm_offset.unwrap_or(0),
        boot_info.kernel_virtual_base,
        boot_info.kernel_start,
    );
    #[cfg(feature = "x86_64")]
    arch::init();
    #[cfg(feature = "x86_64")]
    arch::virtio_input_init();
    #[cfg(feature = "x86_64")]
    arch::usb_input_init();
    #[cfg(feature = "aarch64")]
    arch::init();
    #[cfg(feature = "x86_64")]
    arch::enable_interrupts();

    #[cfg(feature = "qemu_x86_64")]
    platform::init();
    #[cfg(feature = "qemu_virt")]
    platform::init();

    kprintln!(
        "boot: regions={}, kernel=[{:#x}-{:#x}]",
        boot_info.memory_map.len(),
        boot_info.kernel_start,
        boot_info.kernel_end
    );

    let initramfs_slice = boot_info.initramfs.map(|(start, end)| {
        let size = end.saturating_sub(start) as usize;
        unsafe { core::slice::from_raw_parts(start as *const u8, size) }
    });

    if let Some(initramfs) = initramfs_slice {
        match init::load_init_process(initramfs) {
            Ok(process) => {
                kprintln!("init: loaded pid={} entry={:#x}", process.pid, process.ctx.pc);
            }
            Err(err) => {
                kprintln!("init: failed to load ({:?})", err);
            }
        }
    } else {
        kprintln!("init: no initramfs provided");
    }

    shell::run(initramfs_slice);
}
