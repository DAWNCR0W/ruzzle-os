#![cfg(feature = "aarch64")]

use core::arch::global_asm;

use kernel::kprintln;

#[cfg(feature = "qemu_virt")]
use platform_qemu_aarch64_virt as platform;

const BOOT_STACK_SIZE: usize = 4096 * 4;

global_asm!(
    r#"
    .section .text._start, "ax"
    .global _start
_start:
    mrs x1, CurrentEL
    lsr x1, x1, #2
    cmp x1, #2
    b.ne 1f

    adrp x2, boot_stack_end
    add x2, x2, :lo12:boot_stack_end
    msr sp_el1, x2

    mov x2, #(1 << 31)
    msr hcr_el2, x2

    mov x2, #0x3C5
    msr spsr_el2, x2

    adr x2, el1_entry
    msr elr_el2, x2
    eret

1:
el1_entry:
    adrp x1, boot_stack_end
    add x1, x1, :lo12:boot_stack_end
    mov sp, x1
    bl aarch64_entry
    b .

    .section .bss.stack, "aw", %nobits
    .align 16
boot_stack:
    .skip {stack_size}
boot_stack_end:
"#,
    stack_size = const BOOT_STACK_SIZE,
);

extern "C" {
    static __kernel_start: u8;
    static __kernel_end: u8;
    static mut __bss_start: u8;
    static mut __bss_end: u8;
}

#[no_mangle]
pub extern "C" fn aarch64_entry(dtb_ptr: u64) -> ! {
    unsafe {
        let mut cursor = &mut __bss_start as *mut u8;
        let end = &mut __bss_end as *mut u8;
        while cursor < end {
            core::ptr::write_volatile(cursor, 0);
            cursor = cursor.add(1);
        }
    }
    kernel::console::init();
    kprintln!("Ruzzle OS: aarch64 entry");

    let kernel_start = unsafe { &__kernel_start as *const u8 as usize };
    let kernel_end = unsafe { &__kernel_end as *const u8 as usize };

    #[cfg(feature = "qemu_virt")]
    {
        let boot_info = platform::boot_info_from_dtb(dtb_ptr as usize, kernel_start, kernel_end);
        return kernel::entry(boot_info);
    }

    let boot_info = kernel_core::BootInfo {
        memory_map: &[],
        kernel_start,
        kernel_end,
        initramfs: None,
        dtb_ptr: Some(dtb_ptr as usize),
    };
    kernel::entry(boot_info)
}
