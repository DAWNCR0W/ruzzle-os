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
}

/// Builds a system info snapshot from active services.
pub fn build_system_info(
    settings: &SystemSettings,
    session: &SessionManager,
    board: &PuzzleBoard,
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
            PuzzleSlot::new("ruzzle.slot.console", true),
            PuzzleSlot::new("ruzzle.slot.shell", true),
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
            &["ruzzle.slot.console".to_string()],
        );
        let info = build_system_info(&settings, &session, &board);
        assert_eq!(info.hostname, "ruzzle");
        assert_eq!(info.active_user, Some("root".to_string()));
        assert_eq!(info.slots_filled, 1);
        assert_eq!(info.slots_total, 2);
    }

    #[test]
    fn format_includes_defaults_when_missing() {
        let settings = SystemSettings::new_defaults();
        let session = SessionManager::new();
        let board = board();
        let info = build_system_info(&settings, &session, &board);
        let text = format_system_info(&info);
        assert!(text.contains("hostname: ruzzle"));
        assert!(text.contains("user: <none>"));
        assert!(text.contains("slots: 0/2"));
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
        let info = build_system_info(&settings, &session, &board);
        let text = format_system_info(&info);
        assert!(text.contains("user: root"));
    }
}
