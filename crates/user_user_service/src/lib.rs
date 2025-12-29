#![cfg_attr(not(test), no_std)]

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::format;
use alloc::string::{String, ToString};
use alloc::vec::Vec;

/// Errors for the user service.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UserError {
    NotFound,
    AlreadyExists,
    InvalidName,
    NoActiveUser,
}

/// Represents a user account.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UserRecord {
    pub name: String,
    pub is_admin: bool,
    pub home_dir: String,
    pub shell: String,
}

/// In-memory user manager.
#[derive(Debug, Default, Clone)]
pub struct UserManager {
    users: BTreeMap<String, UserRecord>,
    active: Option<String>,
}

impl UserManager {
    /// Creates an empty user manager.
    pub fn new() -> Self {
        Self {
            users: BTreeMap::new(),
            active: None,
        }
    }

    /// Adds a user account.
    pub fn add_user(&mut self, name: &str, is_admin: bool) -> Result<(), UserError> {
        if !is_valid_user_name(name) {
            return Err(UserError::InvalidName);
        }
        if self.users.contains_key(name) {
            return Err(UserError::AlreadyExists);
        }
        let home_dir = default_home_dir(name);
        let shell = default_shell().to_string();
        self.users.insert(
            name.to_string(),
            UserRecord {
                name: name.to_string(),
                is_admin,
                home_dir,
                shell,
            },
        );
        if self.active.is_none() {
            self.active = Some(name.to_string());
        }
        Ok(())
    }

    /// Returns true if a user exists.
    pub fn has_user(&self, name: &str) -> bool {
        self.users.contains_key(name)
    }

    /// Returns a user record by name, if present.
    pub fn get_user(&self, name: &str) -> Option<&UserRecord> {
        self.users.get(name)
    }

    /// Removes a user account.
    pub fn remove_user(&mut self, name: &str) -> Result<(), UserError> {
        if self.users.remove(name).is_none() {
            return Err(UserError::NotFound);
        }
        if self.active.as_deref() == Some(name) {
            self.active = self.users.keys().next().cloned();
        }
        Ok(())
    }

    /// Sets the active user.
    pub fn set_active(&mut self, name: &str) -> Result<(), UserError> {
        if !self.users.contains_key(name) {
            return Err(UserError::NotFound);
        }
        self.active = Some(name.to_string());
        Ok(())
    }

    /// Returns the active user.
    pub fn active_user(&self) -> Result<&UserRecord, UserError> {
        let active = self.active.as_deref().ok_or(UserError::NoActiveUser)?;
        self.users.get(active).ok_or(UserError::NotFound)
    }

    /// Lists users sorted by name.
    pub fn list_users(&self) -> Vec<UserRecord> {
        self.users.values().cloned().collect()
    }
}

/// Validates whether a user name follows the canonical rule.
pub fn is_valid_user_name(name: &str) -> bool {
    if name.is_empty() {
        return false;
    }
    name.chars()
        .all(|ch| ch.is_ascii_lowercase() || ch.is_ascii_digit() || ch == '-')
}

/// Returns the default home directory for a user name.
pub fn default_home_dir(name: &str) -> String {
    format!("/home/{}", name)
}

/// Returns the default login shell path.
pub fn default_shell() -> &'static str {
    "/bin/ruzzle-shell"
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn add_and_list_users() {
        let mut manager = UserManager::new();
        manager.add_user("root", true).unwrap();
        manager.add_user("guest", false).unwrap();
        let list = manager.list_users();
        assert_eq!(list.len(), 2);
        assert_eq!(list[0].name, "guest");
        assert_eq!(list[1].name, "root");
        assert_eq!(list[0].home_dir, "/home/guest");
        assert_eq!(list[1].shell, "/bin/ruzzle-shell");
    }

    #[test]
    fn add_rejects_invalid_name() {
        let mut manager = UserManager::new();
        assert_eq!(manager.add_user("", false), Err(UserError::InvalidName));
        assert_eq!(manager.add_user("Root", false), Err(UserError::InvalidName));
        assert_eq!(manager.add_user("bad name", false), Err(UserError::InvalidName));
    }

    #[test]
    fn validate_user_name_rules() {
        assert!(is_valid_user_name("root"));
        assert!(is_valid_user_name("user-01"));
        assert!(!is_valid_user_name(""));
        assert!(!is_valid_user_name("Bad"));
        assert!(!is_valid_user_name("space name"));
    }

    #[test]
    fn default_paths_are_stable() {
        assert_eq!(default_home_dir("root"), "/home/root");
        assert_eq!(default_shell(), "/bin/ruzzle-shell");
    }

    #[test]
    fn add_rejects_duplicates() {
        let mut manager = UserManager::new();
        manager.add_user("root", true).unwrap();
        assert_eq!(manager.add_user("root", false), Err(UserError::AlreadyExists));
    }

    #[test]
    fn active_user_defaults_to_first() {
        let mut manager = UserManager::new();
        manager.add_user("root", true).unwrap();
        let active = manager.active_user().unwrap();
        assert_eq!(active.name, "root");
    }

    #[test]
    fn set_active_user() {
        let mut manager = UserManager::new();
        manager.add_user("root", true).unwrap();
        manager.add_user("guest", false).unwrap();
        manager.set_active("guest").unwrap();
        let active = manager.active_user().unwrap();
        assert_eq!(active.name, "guest");
    }

    #[test]
    fn has_user_reflects_state() {
        let mut manager = UserManager::new();
        assert!(!manager.has_user("root"));
        manager.add_user("root", true).unwrap();
        assert!(manager.has_user("root"));
        manager.remove_user("root").unwrap();
        assert!(!manager.has_user("root"));
    }

    #[test]
    fn get_user_returns_record() {
        let mut manager = UserManager::new();
        manager.add_user("root", true).unwrap();
        let user = manager.get_user("root").unwrap();
        assert_eq!(user.name, "root");
        assert!(manager.get_user("missing").is_none());
    }

    #[test]
    fn set_active_requires_user() {
        let mut manager = UserManager::new();
        assert_eq!(manager.set_active("root"), Err(UserError::NotFound));
    }

    #[test]
    fn remove_user_updates_active() {
        let mut manager = UserManager::new();
        manager.add_user("root", true).unwrap();
        manager.add_user("guest", false).unwrap();
        manager.set_active("guest").unwrap();
        manager.remove_user("guest").unwrap();
        let active = manager.active_user().unwrap();
        assert_eq!(active.name, "root");
    }

    #[test]
    fn remove_non_active_user_keeps_active() {
        let mut manager = UserManager::new();
        manager.add_user("root", true).unwrap();
        manager.add_user("guest", false).unwrap();
        manager.set_active("root").unwrap();
        manager.remove_user("guest").unwrap();
        let active = manager.active_user().unwrap();
        assert_eq!(active.name, "root");
    }

    #[test]
    fn remove_last_active_user_clears_active() {
        let mut manager = UserManager::new();
        manager.add_user("root", true).unwrap();
        manager.remove_user("root").unwrap();
        assert_eq!(manager.active_user(), Err(UserError::NoActiveUser));
    }

    #[test]
    fn remove_user_requires_existing() {
        let mut manager = UserManager::new();
        assert_eq!(manager.remove_user("missing"), Err(UserError::NotFound));
    }

    #[test]
    fn active_user_requires_presence() {
        let manager = UserManager::new();
        assert_eq!(manager.active_user(), Err(UserError::NoActiveUser));
    }
}
