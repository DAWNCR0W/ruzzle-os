#![cfg_attr(not(test), no_std)]

extern crate alloc;

use alloc::string::{String, ToString};
use user_puzzle_board::PuzzleBoard;
use user_session_service::SessionManager;
use user_settings_service::SystemSettings;

/// High-level system info snapshot.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SystemInfo {
    pub hostname: String,
    pub locale: String,
    pub timezone: String,
    pub keyboard: String,
    pub active_user: Option<String>,
    pub slots_filled: usize,
    pub slots_total: usize,
    pub cpu_total: usize,
    pub cpu_online: usize,
    pub gpu_devices: usize,
}

/// Runtime metrics supplied by the kernel.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SystemMetrics {
    pub cpu_total: usize,
    pub cpu_online: usize,
    pub gpu_devices: usize,
}

impl Default for SystemMetrics {
    fn default() -> Self {
        Self {
            cpu_total: 1,
            cpu_online: 1,
            gpu_devices: 0,
        }
    }
}

/// Builds a system info snapshot from active services.
pub fn build_system_info(
    settings: &SystemSettings,
    session: &SessionManager,
    board: &PuzzleBoard,
    metrics: SystemMetrics,
) -> SystemInfo {
    let slots = board.list();
    let filled = slots.iter().filter(|slot| slot.provider.is_some()).count();
    SystemInfo {
        hostname: settings.hostname().to_string(),
        locale: settings.locale().to_string(),
        timezone: settings.timezone().to_string(),
        keyboard: settings.keyboard().to_string(),
        active_user: session.active_user().map(|name| name.to_string()),
        slots_filled: filled,
        slots_total: slots.len(),
        cpu_total: metrics.cpu_total,
        cpu_online: metrics.cpu_online,
        gpu_devices: metrics.gpu_devices,
    }
}

/// Formats system info into a CLI-friendly text block.
pub fn format_system_info(info: &SystemInfo) -> String {
    let mut out = String::new();
    out.push_str("system:\n");
    out.push_str("  hostname: ");
    out.push_str(&info.hostname);
    out.push('\n');
    out.push_str("  locale: ");
    out.push_str(&info.locale);
    out.push('\n');
    out.push_str("  timezone: ");
    out.push_str(&info.timezone);
    out.push('\n');
    out.push_str("  keyboard: ");
    out.push_str(&info.keyboard);
    out.push('\n');
    out.push_str("  user: ");
    if let Some(user) = &info.active_user {
        out.push_str(user);
    } else {
        out.push_str("<none>");
    }
    out.push('\n');
    out.push_str("  slots: ");
    out.push_str(&info.slots_filled.to_string());
    out.push('/');
    out.push_str(&info.slots_total.to_string());
    out.push('\n');
    out.push_str("  cpu: ");
    out.push_str(&info.cpu_online.to_string());
    out.push('/');
    out.push_str(&info.cpu_total.to_string());
    out.push('\n');
    out.push_str("  gpu: ");
    out.push_str(&info.gpu_devices.to_string());
    out.push('\n');
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use user_puzzle_board::{PuzzleBoard, PuzzleSlot};
    use user_session_service::SessionManager;
    use user_settings_service::SystemSettings;
    use user_user_service::UserManager;

    fn board() -> PuzzleBoard {
        PuzzleBoard::new(vec![
            PuzzleSlot::new("ruzzle.slot.console@1", true),
            PuzzleSlot::new("ruzzle.slot.shell@1", true),
        ])
    }

    #[test]
    fn build_info_reflects_state() {
        let settings = SystemSettings::new_defaults();
        let mut session = SessionManager::new();
        let users = {
            let mut users = UserManager::new();
            users.add_user("root", true).unwrap();
            users
        };
        session.login(&users, "root").unwrap();

        let mut board = board();
        board.mark_running(
            "console-service",
            &["ruzzle.slot.console@1".to_string()],
        );
        let info = build_system_info(
            &settings,
            &session,
            &board,
            SystemMetrics {
                cpu_total: 4,
                cpu_online: 2,
                gpu_devices: 1,
            },
        );
        assert_eq!(info.hostname, "ruzzle");
        assert_eq!(info.active_user, Some("root".to_string()));
        assert_eq!(info.slots_filled, 1);
        assert_eq!(info.slots_total, 2);
        assert_eq!(info.cpu_total, 4);
        assert_eq!(info.cpu_online, 2);
        assert_eq!(info.gpu_devices, 1);
    }

    #[test]
    fn format_includes_defaults_when_missing() {
        let settings = SystemSettings::new_defaults();
        let session = SessionManager::new();
        let board = board();
        let info = build_system_info(&settings, &session, &board, SystemMetrics::default());
        let text = format_system_info(&info);
        assert!(text.contains("hostname: ruzzle"));
        assert!(text.contains("user: <none>"));
        assert!(text.contains("slots: 0/2"));
        assert!(text.contains("cpu: 1/1"));
        assert!(text.contains("gpu: 0"));
    }

    #[test]
    fn format_includes_active_user() {
        let settings = SystemSettings::new_defaults();
        let users = {
            let mut users = UserManager::new();
            users.add_user("root", true).unwrap();
            users
        };
        let mut session = SessionManager::new();
        session.login(&users, "root").unwrap();

        let board = board();
        let info = build_system_info(&settings, &session, &board, SystemMetrics::default());
        let text = format_system_info(&info);
        assert!(text.contains("user: root"));
    }
}
