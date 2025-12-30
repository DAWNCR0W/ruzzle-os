#![cfg_attr(not(test), no_std)]

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::string::{String, ToString};
use alloc::vec::Vec;

/// Device category.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeviceKind {
    Input,
    Storage,
    Network,
    Gpu,
    Audio,
    Other,
}

/// Device descriptor.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Device {
    pub id: String,
    pub kind: DeviceKind,
    pub driver: Option<String>,
    pub enabled: bool,
}

/// Errors for device management.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DeviceError {
    InvalidId,
    AlreadyExists,
    NotFound,
    DriverAlreadyBound,
    DriverNotBound,
}

/// Registry of devices and driver bindings.
#[derive(Debug, Default, Clone)]
pub struct DeviceRegistry {
    devices: BTreeMap<String, Device>,
}

impl DeviceRegistry {
    /// Creates an empty registry.
    pub fn new() -> Self {
        Self {
            devices: BTreeMap::new(),
        }
    }

    /// Registers a new device.
    pub fn register(&mut self, id: &str, kind: DeviceKind) -> Result<(), DeviceError> {
        if !is_valid_id(id) {
            return Err(DeviceError::InvalidId);
        }
        if self.devices.contains_key(id) {
            return Err(DeviceError::AlreadyExists);
        }
        self.devices.insert(
            id.to_string(),
            Device {
                id: id.to_string(),
                kind,
                driver: None,
                enabled: false,
            },
        );
        Ok(())
    }

    /// Binds a driver to a device.
    pub fn bind_driver(&mut self, id: &str, driver: &str) -> Result<(), DeviceError> {
        let device = self.devices.get_mut(id).ok_or(DeviceError::NotFound)?;
        if device.driver.is_some() {
            return Err(DeviceError::DriverAlreadyBound);
        }
        device.driver = Some(driver.to_string());
        Ok(())
    }

    /// Unbinds a driver from a device.
    pub fn unbind_driver(&mut self, id: &str) -> Result<(), DeviceError> {
        let device = self.devices.get_mut(id).ok_or(DeviceError::NotFound)?;
        if device.driver.is_none() {
            return Err(DeviceError::DriverNotBound);
        }
        device.driver = None;
        Ok(())
    }

    /// Enables or disables a device.
    pub fn set_enabled(&mut self, id: &str, enabled: bool) -> Result<(), DeviceError> {
        let device = self.devices.get_mut(id).ok_or(DeviceError::NotFound)?;
        device.enabled = enabled;
        Ok(())
    }

    /// Lists devices sorted by id.
    pub fn list(&self) -> Vec<Device> {
        self.devices.values().cloned().collect()
    }
}

fn is_valid_id(id: &str) -> bool {
    if id.is_empty() {
        return false;
    }
    id.chars()
        .all(|ch| ch.is_ascii_lowercase() || ch.is_ascii_digit() || ch == '-' || ch == '.')
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn register_and_list_devices() {
        let mut registry = DeviceRegistry::new();
        registry.register("kbd0", DeviceKind::Input).unwrap();
        let list = registry.list();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].kind, DeviceKind::Input);
    }

    #[test]
    fn register_rejects_invalid_id() {
        let mut registry = DeviceRegistry::new();
        assert_eq!(
            registry.register("BadId", DeviceKind::Input),
            Err(DeviceError::InvalidId)
        );
    }

    #[test]
    fn register_rejects_empty_id() {
        let mut registry = DeviceRegistry::new();
        assert_eq!(registry.register("", DeviceKind::Input), Err(DeviceError::InvalidId));
    }

    #[test]
    fn register_rejects_duplicates() {
        let mut registry = DeviceRegistry::new();
        registry.register("eth0", DeviceKind::Network).unwrap();
        assert_eq!(
            registry.register("eth0", DeviceKind::Network),
            Err(DeviceError::AlreadyExists)
        );
    }

    #[test]
    fn bind_and_unbind_driver() {
        let mut registry = DeviceRegistry::new();
        registry.register("gpu0", DeviceKind::Gpu).unwrap();
        registry.bind_driver("gpu0", "gpu-service").unwrap();
        assert_eq!(
            registry.bind_driver("gpu0", "gpu-service"),
            Err(DeviceError::DriverAlreadyBound)
        );
        registry.unbind_driver("gpu0").unwrap();
        assert_eq!(
            registry.unbind_driver("gpu0"),
            Err(DeviceError::DriverNotBound)
        );
    }

    #[test]
    fn set_enabled_updates_state() {
        let mut registry = DeviceRegistry::new();
        registry.register("disk0", DeviceKind::Storage).unwrap();
        registry.set_enabled("disk0", true).unwrap();
        let device = registry.list().pop().unwrap();
        assert!(device.enabled);
    }

    #[test]
    fn operations_reject_missing_device() {
        let mut registry = DeviceRegistry::new();
        assert_eq!(
            registry.bind_driver("missing", "driver"),
            Err(DeviceError::NotFound)
        );
        assert_eq!(
            registry.unbind_driver("missing"),
            Err(DeviceError::NotFound)
        );
        assert_eq!(
            registry.set_enabled("missing", true),
            Err(DeviceError::NotFound)
        );
    }
}
