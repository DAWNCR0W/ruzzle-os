#![cfg_attr(not(test), no_std)]

extern crate alloc;

use alloc::string::String;
use alloc::string::ToString;

pub mod protocol;

/// Log levels supported by the console service.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogLevel {
    Info,
    Warn,
    Error,
}

impl LogLevel {
    fn as_str(self) -> &'static str {
        match self {
            LogLevel::Info => "INFO",
            LogLevel::Warn => "WARN",
            LogLevel::Error => "ERROR",
        }
    }
}

/// Formats a log line for UART output.
///
/// The format is: "[LEVEL][pid] message".
pub fn format_log(pid: u32, level: LogLevel, message: &str) -> String {
    let mut line = String::new();
    line.push('[');
    line.push_str(level.as_str());
    line.push(']');
    line.push('[');
    line.push_str(&pid.to_string());
    line.push(']');
    line.push(' ');
    line.push_str(message);
    line
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn format_log_includes_level_and_pid() {
        let line = format_log(7, LogLevel::Info, "hello");
        assert_eq!(line, "[INFO][7] hello");
    }

    #[test]
    fn format_log_handles_error_level() {
        let line = format_log(1, LogLevel::Error, "boom");
        assert_eq!(line, "[ERROR][1] boom");
    }

    #[test]
    fn format_log_handles_warn_level() {
        let line = format_log(3, LogLevel::Warn, "heads up");
        assert_eq!(line, "[WARN][3] heads up");
    }
}
