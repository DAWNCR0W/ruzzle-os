use core::fmt::{self, Write};

#[cfg(feature = "x86_64")]
use arch_x86_64 as arch;
#[cfg(feature = "aarch64")]
use platform_qemu_aarch64_virt as platform;

/// Initializes the early serial console.
pub fn init() {
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

pub fn print(args: fmt::Arguments) {
    let mut writer = ConsoleWriter;
    let _ = writer.write_fmt(args);
}

/// Returns true if a byte is available on any console input.
pub fn has_input() -> bool {
    #[cfg(feature = "x86_64")]
    {
        return arch::keyboard_has_data() || arch::serial_has_data();
    }
    #[cfg(feature = "aarch64")]
    {
        return platform::uart_has_data();
    }
    #[cfg(not(feature = "x86_64"))]
    {
        false
    }
}

/// Reads a byte from the active console input. Callers should check `has_input` first.
pub fn read_byte() -> u8 {
    #[cfg(feature = "x86_64")]
    {
        while arch::keyboard_has_data() {
            if let Some(byte) = arch::keyboard_read_byte() {
                return byte;
            }
        }
        if arch::serial_has_data() {
            return arch::serial_read_byte();
        }
        return 0;
    }
    #[cfg(feature = "aarch64")]
    {
        if platform::uart_has_data() {
            return platform::uart_read_byte();
        }
        return 0;
    }
    #[cfg(not(feature = "x86_64"))]
    {
        0
    }
}

struct ConsoleWriter;

impl Write for ConsoleWriter {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        #[cfg(feature = "x86_64")]
        {
            arch::serial_write_str(s);
            arch::vga_write_str(s);
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
