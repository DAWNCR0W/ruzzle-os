#![cfg_attr(not(test), no_std)]

/// Small toggle helper for pieces.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Toggle {
    enabled: bool,
}

/// Errors returned by the toggle.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToggleError {
    AlreadyOn,
    AlreadyOff,
}

impl Toggle {
    /// Creates a new toggle (disabled).
    pub const fn new() -> Self {
        Self { enabled: false }
    }

    /// Enables the toggle.
    pub fn enable(&mut self) -> Result<(), ToggleError> {
        if self.enabled {
            return Err(ToggleError::AlreadyOn);
        }
        self.enabled = true;
        Ok(())
    }

    /// Disables the toggle.
    pub fn disable(&mut self) -> Result<(), ToggleError> {
        if !self.enabled {
            return Err(ToggleError::AlreadyOff);
        }
        self.enabled = false;
        Ok(())
    }

    /// Returns the current state.
    pub fn is_enabled(&self) -> bool {
        self.enabled
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn toggle_starts_off() {
        let toggle = Toggle::new();
        assert!(!toggle.is_enabled());
    }

    #[test]
    fn enable_disable_roundtrip() {
        let mut toggle = Toggle::new();
        assert_eq!(toggle.enable(), Ok(()));
        assert!(toggle.is_enabled());
        assert_eq!(toggle.disable(), Ok(()));
        assert!(!toggle.is_enabled());
    }

    #[test]
    fn enable_rejects_when_already_on() {
        let mut toggle = Toggle::new();
        toggle.enable().unwrap();
        assert_eq!(toggle.enable(), Err(ToggleError::AlreadyOn));
    }

    #[test]
    fn disable_rejects_when_already_off() {
        let mut toggle = Toggle::new();
        assert_eq!(toggle.disable(), Err(ToggleError::AlreadyOff));
    }
}
