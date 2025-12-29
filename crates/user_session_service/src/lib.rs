#![cfg_attr(not(test), no_std)]

extern crate alloc;

use alloc::string::{String, ToString};

use user_user_service::UserManager;

/// Errors returned by session management.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SessionError {
    UserNotFound,
    AlreadyLoggedIn,
    NotLoggedIn,
}

/// Tracks the active login session.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct SessionManager {
    active: Option<String>,
}

impl SessionManager {
    /// Creates an empty session manager.
    pub fn new() -> Self {
        Self { active: None }
    }

    /// Returns true if a user is logged in.
    pub fn is_logged_in(&self) -> bool {
        self.active.is_some()
    }

    /// Returns the active user name, if any.
    pub fn active_user(&self) -> Option<&str> {
        self.active.as_deref()
    }

    /// Logs in a user if they exist.
    pub fn login(&mut self, users: &UserManager, name: &str) -> Result<(), SessionError> {
        if self.active.is_some() {
            return Err(SessionError::AlreadyLoggedIn);
        }
        if !users.has_user(name) {
            return Err(SessionError::UserNotFound);
        }
        self.active = Some(name.to_string());
        Ok(())
    }

    /// Logs out the active user.
    pub fn logout(&mut self) -> Result<(), SessionError> {
        if self.active.is_none() {
            return Err(SessionError::NotLoggedIn);
        }
        self.active = None;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use user_user_service::UserManager;

    #[test]
    fn login_and_logout_flow() {
        let mut users = UserManager::new();
        users.add_user("root", true).unwrap();

        let mut session = SessionManager::new();
        assert!(!session.is_logged_in());
        session.login(&users, "root").unwrap();
        assert!(session.is_logged_in());
        assert_eq!(session.active_user(), Some("root"));

        session.logout().unwrap();
        assert!(!session.is_logged_in());
        assert_eq!(session.active_user(), None);
    }

    #[test]
    fn login_rejects_missing_user() {
        let users = UserManager::new();
        let mut session = SessionManager::new();
        assert_eq!(session.login(&users, "root"), Err(SessionError::UserNotFound));
    }

    #[test]
    fn login_rejects_when_already_logged_in() {
        let mut users = UserManager::new();
        users.add_user("root", true).unwrap();
        users.add_user("guest", false).unwrap();

        let mut session = SessionManager::new();
        session.login(&users, "root").unwrap();
        assert_eq!(
            session.login(&users, "guest"),
            Err(SessionError::AlreadyLoggedIn)
        );
    }

    #[test]
    fn logout_requires_active_session() {
        let mut session = SessionManager::new();
        assert_eq!(session.logout(), Err(SessionError::NotLoggedIn));
    }
}
