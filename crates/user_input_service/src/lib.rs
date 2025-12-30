#![cfg_attr(not(test), no_std)]

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::string::{String, ToString};
use alloc::vec::Vec;

/// Input bus types.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputBus {
    Usb,
    Virtio,
    Ps2,
    Serial,
}

/// Key press/release state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyState {
    Pressed,
    Released,
}

/// Input device metadata.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InputDevice {
    pub id: String,
    pub bus: InputBus,
    pub vendor: u16,
    pub product: u16,
}

/// Input event record.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InputEvent {
    pub device_id: String,
    pub key_code: u16,
    pub state: KeyState,
}

/// Input service errors.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InputError {
    InvalidId,
    AlreadyExists,
    NotFound,
}

/// In-memory input hub with device registry and event queue.
#[derive(Debug, Default, Clone)]
pub struct InputHub {
    devices: BTreeMap<String, InputDevice>,
    queue: Vec<InputEvent>,
}

impl InputHub {
    /// Creates a new input hub.
    pub fn new() -> Self {
        Self {
            devices: BTreeMap::new(),
            queue: Vec::new(),
        }
    }

    /// Registers a device.
    pub fn register_device(&mut self, device: InputDevice) -> Result<(), InputError> {
        if !is_valid_id(&device.id) {
            return Err(InputError::InvalidId);
        }
        if self.devices.contains_key(&device.id) {
            return Err(InputError::AlreadyExists);
        }
        self.devices.insert(device.id.clone(), device);
        Ok(())
    }

    /// Pushes a new input event.
    pub fn push_event(&mut self, event: InputEvent) -> Result<(), InputError> {
        if !self.devices.contains_key(&event.device_id) {
            return Err(InputError::NotFound);
        }
        self.queue.push(event);
        Ok(())
    }

    /// Drains the event queue.
    pub fn drain_events(&mut self) -> Vec<InputEvent> {
        let mut drained = Vec::new();
        core::mem::swap(&mut drained, &mut self.queue);
        drained
    }

    /// Returns the registered device count.
    pub fn device_count(&self) -> usize {
        self.devices.len()
    }

    /// Returns a device by id.
    pub fn device(&self, id: &str) -> Option<&InputDevice> {
        self.devices.get(id)
    }
}

fn is_valid_id(id: &str) -> bool {
    if id.is_empty() {
        return false;
    }
    id.chars()
        .all(|ch| ch.is_ascii_lowercase() || ch.is_ascii_digit() || ch == '-')
}

#[cfg(test)]
mod tests {
    use super::*;

    fn device(id: &str, bus: InputBus) -> InputDevice {
        InputDevice {
            id: id.to_string(),
            bus,
            vendor: 0x1234,
            product: 0x5678,
        }
    }

    #[test]
    fn register_and_lookup_device() {
        let mut hub = InputHub::new();
        hub.register_device(device("kbd0", InputBus::Usb)).unwrap();
        assert_eq!(hub.device_count(), 1);
        assert_eq!(hub.device("kbd0").unwrap().bus, InputBus::Usb);
    }

    #[test]
    fn register_rejects_invalid_id() {
        let mut hub = InputHub::new();
        assert_eq!(
            hub.register_device(device("BadId", InputBus::Usb)),
            Err(InputError::InvalidId)
        );
    }

    #[test]
    fn register_rejects_empty_id() {
        let mut hub = InputHub::new();
        assert_eq!(
            hub.register_device(device("", InputBus::Usb)),
            Err(InputError::InvalidId)
        );
    }

    #[test]
    fn register_rejects_duplicates() {
        let mut hub = InputHub::new();
        hub.register_device(device("kbd0", InputBus::Usb)).unwrap();
        assert_eq!(
            hub.register_device(device("kbd0", InputBus::Usb)),
            Err(InputError::AlreadyExists)
        );
    }

    #[test]
    fn push_event_and_drain() {
        let mut hub = InputHub::new();
        hub.register_device(device("kbd0", InputBus::Ps2)).unwrap();
        hub.push_event(InputEvent {
            device_id: "kbd0".to_string(),
            key_code: 0x1e,
            state: KeyState::Pressed,
        })
        .unwrap();
        let events = hub.drain_events();
        assert_eq!(events.len(), 1);
        assert!(hub.drain_events().is_empty());
    }

    #[test]
    fn push_event_rejects_missing_device() {
        let mut hub = InputHub::new();
        assert_eq!(
            hub.push_event(InputEvent {
                device_id: "kbd0".to_string(),
                key_code: 1,
                state: KeyState::Pressed,
            }),
            Err(InputError::NotFound)
        );
    }
}
