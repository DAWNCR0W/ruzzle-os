use core::fmt::{self, Write};

#[cfg(feature = "x86_64")]
use arch_x86_64 as arch;
#[cfg(feature = "aarch64")]
use platform_qemu_aarch64_virt as platform;
#[cfg(feature = "x86_64")]
use spin::Mutex;

use kernel_core::FramebufferInfo;

#[cfg(feature = "x86_64")]
use crate::framebuffer::FramebufferConsole;

#[cfg(feature = "x86_64")]
static FRAMEBUFFER: Mutex<Option<FramebufferConsole>> = Mutex::new(None);

/// Initializes the early serial console.
pub fn init_early() {
    #[cfg(feature = "x86_64")]
    {
        arch::init_serial();
        arch::vga_init();
        arch::keyboard_init();
    }
    #[cfg(feature = "aarch64")]
    {
        platform::init();
    }
}

/// Attaches a framebuffer console if available.
#[cfg(feature = "x86_64")]
pub fn init_framebuffer(framebuffer: Option<FramebufferInfo>) {
    let Some(info) = framebuffer else {
        return;
    };
    if let Some(mut console) = FramebufferConsole::new(info) {
        console.clear();
        let mut fb = FRAMEBUFFER.lock();
        *fb = Some(console);
    }
}

/// Attaches a framebuffer console (no-op on non-x86_64 targets).
#[cfg(not(feature = "x86_64"))]
pub fn init_framebuffer(_framebuffer: Option<FramebufferInfo>) {}

pub fn print(args: fmt::Arguments) {
    let mut writer = ConsoleWriter;
    let _ = writer.write_fmt(args);
}

/// Returns true if a byte is available on any console input.
#[cfg(feature = "x86_64")]
pub fn has_input() -> bool {
    arch::keyboard_has_data()
        || arch::usb_input_has_data()
        || arch::virtio_input_has_data()
        || arch::serial_has_data()
}

/// Returns true if a byte is available on any console input.
#[cfg(all(not(feature = "x86_64"), feature = "aarch64"))]
pub fn has_input() -> bool {
    platform::uart_has_data()
}

/// Returns true if a byte is available on any console input.
#[cfg(not(any(feature = "x86_64", feature = "aarch64")))]
pub fn has_input() -> bool {
    false
}

/// Reads a byte from the active console input. Callers should check `has_input` first.
#[cfg(feature = "x86_64")]
pub fn read_byte() -> u8 {
    while arch::keyboard_has_data() {
        if let Some(byte) = arch::keyboard_read_byte() {
            return byte;
        }
    }
    if arch::usb_input_has_data() {
        if let Some(byte) = arch::usb_input_read_byte() {
            return byte;
        }
    }
    if arch::virtio_input_has_data() {
        if let Some(byte) = arch::virtio_input_read_byte() {
            return byte;
        }
    }
    if arch::serial_has_data() {
        return arch::serial_read_byte();
    }
    0
}

/// Reads a byte from the active console input. Callers should check `has_input` first.
#[cfg(all(not(feature = "x86_64"), feature = "aarch64"))]
pub fn read_byte() -> u8 {
    if platform::uart_has_data() {
        return platform::uart_read_byte();
    }
    0
}

/// Reads a byte from the active console input. Callers should check `has_input` first.
#[cfg(not(any(feature = "x86_64", feature = "aarch64")))]
pub fn read_byte() -> u8 {
    0
}

struct ConsoleWriter;

impl Write for ConsoleWriter {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        #[cfg(feature = "x86_64")]
        {
            arch::serial_write_str(s);
            let mut fb = FRAMEBUFFER.lock();
            if let Some(console) = fb.as_mut() {
                console.write_str(s);
            } else {
                arch::vga_write_str(s);
            }
        }
        #[cfg(feature = "aarch64")]
        {
            for byte in s.bytes() {
                platform::uart_write(byte);
            }
        }
        Ok(())
    }
}

#[macro_export]
macro_rules! kprint {
    ($($arg:tt)*) => {{
        $crate::console::print(format_args!($($arg)*));
    }};
}

#[macro_export]
macro_rules! kprintln {
    () => {{
        $crate::kprint!("\n");
    }};
    ($($arg:tt)*) => {{
        $crate::kprint!("{}\n", format_args!($($arg)*));
    }};
}
