#![cfg_attr(not(test), no_std)]

extern crate alloc;

use alloc::string::{String, ToString};

/// Errors returned when updating system settings.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SettingsError {
    InvalidHostname,
    InvalidLocale,
    InvalidTimezone,
    InvalidKeyboard,
}

/// System-wide settings configured during first boot.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SystemSettings {
    hostname: String,
    locale: String,
    timezone: String,
    keyboard: String,
}

impl SystemSettings {
    /// Creates settings with safe defaults.
    pub fn new_defaults() -> Self {
        Self {
            hostname: "ruzzle".to_string(),
            locale: "en_US.UTF-8".to_string(),
            timezone: "UTC".to_string(),
            keyboard: "us".to_string(),
        }
    }

    /// Returns the configured hostname.
    pub fn hostname(&self) -> &str {
        &self.hostname
    }

    /// Returns the configured locale.
    pub fn locale(&self) -> &str {
        &self.locale
    }

    /// Returns the configured timezone.
    pub fn timezone(&self) -> &str {
        &self.timezone
    }

    /// Returns the configured keyboard layout.
    pub fn keyboard(&self) -> &str {
        &self.keyboard
    }

    /// Updates the hostname.
    pub fn set_hostname(&mut self, hostname: &str) -> Result<(), SettingsError> {
        if !is_valid_hostname(hostname) {
            return Err(SettingsError::InvalidHostname);
        }
        self.hostname = hostname.to_string();
        Ok(())
    }

    /// Updates the locale.
    pub fn set_locale(&mut self, locale: &str) -> Result<(), SettingsError> {
        if !is_valid_locale(locale) {
            return Err(SettingsError::InvalidLocale);
        }
        self.locale = locale.to_string();
        Ok(())
    }

    /// Updates the timezone identifier.
    pub fn set_timezone(&mut self, timezone: &str) -> Result<(), SettingsError> {
        if !is_valid_timezone(timezone) {
            return Err(SettingsError::InvalidTimezone);
        }
        self.timezone = timezone.to_string();
        Ok(())
    }

    /// Updates the keyboard layout.
    pub fn set_keyboard(&mut self, keyboard: &str) -> Result<(), SettingsError> {
        if !is_valid_keyboard(keyboard) {
            return Err(SettingsError::InvalidKeyboard);
        }
        self.keyboard = keyboard.to_string();
        Ok(())
    }

    /// Serializes settings into a simple config text.
    pub fn to_config_text(&self) -> String {
        let mut out = String::new();
        out.push_str("hostname=");
        out.push_str(&self.hostname);
        out.push('\n');
        out.push_str("locale=");
        out.push_str(&self.locale);
        out.push('\n');
        out.push_str("timezone=");
        out.push_str(&self.timezone);
        out.push('\n');
        out.push_str("keyboard=");
        out.push_str(&self.keyboard);
        out.push('\n');
        out
    }
}

fn is_valid_hostname(hostname: &str) -> bool {
    let trimmed = hostname.trim();
    if trimmed.is_empty() || trimmed.len() > 63 {
        return false;
    }
    let mut saw_label = false;
    for label in trimmed.split('.') {
        if label.is_empty() || label.len() > 63 {
            return false;
        }
        if label.starts_with('-') || label.ends_with('-') {
            return false;
        }
        if !label
            .chars()
            .all(|ch| ch.is_ascii_lowercase() || ch.is_ascii_digit() || ch == '-')
        {
            return false;
        }
        saw_label = true;
    }
    saw_label
}

fn is_valid_locale(locale: &str) -> bool {
    let trimmed = locale.trim();
    if trimmed.is_empty() {
        return false;
    }
    trimmed
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '_' | '-' | '.' | '@'))
}

fn is_valid_timezone(timezone: &str) -> bool {
    let trimmed = timezone.trim();
    if trimmed.is_empty() {
        return false;
    }
    if trimmed.starts_with('/') || trimmed.ends_with('/') {
        return false;
    }
    trimmed
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '/' | '_' | '-' | '+'))
}

fn is_valid_keyboard(keyboard: &str) -> bool {
    let trimmed = keyboard.trim();
    if trimmed.is_empty() {
        return false;
    }
    trimmed
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '_' | '-'))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_are_stable() {
        let settings = SystemSettings::new_defaults();
        assert_eq!(settings.hostname(), "ruzzle");
        assert_eq!(settings.locale(), "en_US.UTF-8");
        assert_eq!(settings.timezone(), "UTC");
        assert_eq!(settings.keyboard(), "us");
    }

    #[test]
    fn hostname_validation_rules() {
        assert!(is_valid_hostname("ruzzle"));
        assert!(is_valid_hostname("ruzzle-core"));
        assert!(is_valid_hostname("node-1.local"));
        assert!(!is_valid_hostname("bad..host"));
        assert!(!is_valid_hostname(""));
        assert!(!is_valid_hostname("-bad"));
        assert!(!is_valid_hostname("bad-"));
        assert!(!is_valid_hostname("Bad"));
        assert!(!is_valid_hostname("has space"));
    }

    #[test]
    fn locale_validation_rules() {
        assert!(is_valid_locale("en_US.UTF-8"));
        assert!(is_valid_locale("ko_KR"));
        assert!(!is_valid_locale(""));
        assert!(!is_valid_locale("en US"));
    }

    #[test]
    fn timezone_validation_rules() {
        assert!(is_valid_timezone("UTC"));
        assert!(is_valid_timezone("Asia/Seoul"));
        assert!(is_valid_timezone("Etc/GMT+9"));
        assert!(!is_valid_timezone(""));
        assert!(!is_valid_timezone("/UTC"));
        assert!(!is_valid_timezone("UTC/"));
        assert!(!is_valid_timezone("Bad Zone"));
    }

    #[test]
    fn keyboard_validation_rules() {
        assert!(is_valid_keyboard("us"));
        assert!(is_valid_keyboard("kr"));
        assert!(is_valid_keyboard("us-intl"));
        assert!(!is_valid_keyboard(""));
        assert!(!is_valid_keyboard("kr layout"));
    }

    #[test]
    fn setters_update_values() {
        let mut settings = SystemSettings::new_defaults();
        settings.set_hostname("ruzzle-box").unwrap();
        settings.set_locale("ko_KR.UTF-8").unwrap();
        settings.set_timezone("Asia/Seoul").unwrap();
        settings.set_keyboard("kr").unwrap();

        assert_eq!(settings.hostname(), "ruzzle-box");
        assert_eq!(settings.locale(), "ko_KR.UTF-8");
        assert_eq!(settings.timezone(), "Asia/Seoul");
        assert_eq!(settings.keyboard(), "kr");
    }

    #[test]
    fn setters_reject_invalid_inputs() {
        let mut settings = SystemSettings::new_defaults();
        assert_eq!(
            settings.set_hostname("Bad"),
            Err(SettingsError::InvalidHostname)
        );
        assert_eq!(
            settings.set_locale("bad locale"),
            Err(SettingsError::InvalidLocale)
        );
        assert_eq!(
            settings.set_timezone("UTC/"),
            Err(SettingsError::InvalidTimezone)
        );
        assert_eq!(
            settings.set_keyboard("kr layout"),
            Err(SettingsError::InvalidKeyboard)
        );
    }

    #[test]
    fn config_text_contains_all_fields() {
        let settings = SystemSettings::new_defaults();
        let text = settings.to_config_text();
        assert!(text.contains("hostname=ruzzle"));
        assert!(text.contains("locale=en_US.UTF-8"));
        assert!(text.contains("timezone=UTC"));
        assert!(text.contains("keyboard=us"));
    }
}
