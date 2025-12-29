#![cfg_attr(not(test), no_std)]

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::string::{String, ToString};
use alloc::vec::Vec;

/// Errors for the net service model.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NetError {
    NotFound,
    AlreadyExists,
    InvalidName,
    InvalidAddress,
}

/// Simple representation of a network interface.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NetInterface {
    pub name: String,
    pub up: bool,
    pub ipv4: Option<String>,
}

/// In-memory network configuration manager.
#[derive(Debug, Default, Clone)]
pub struct NetManager {
    interfaces: BTreeMap<String, NetInterface>,
}

impl NetManager {
    /// Creates an empty network manager.
    pub fn new() -> Self {
        Self {
            interfaces: BTreeMap::new(),
        }
    }

    /// Adds an interface by name.
    pub fn add_interface(&mut self, name: &str) -> Result<(), NetError> {
        if !is_valid_iface_name(name) {
            return Err(NetError::InvalidName);
        }
        if self.interfaces.contains_key(name) {
            return Err(NetError::AlreadyExists);
        }
        self.interfaces.insert(
            name.to_string(),
            NetInterface {
                name: name.to_string(),
                up: false,
                ipv4: None,
            },
        );
        Ok(())
    }

    /// Removes an interface.
    pub fn remove_interface(&mut self, name: &str) -> Result<(), NetError> {
        if self.interfaces.remove(name).is_some() {
            Ok(())
        } else {
            Err(NetError::NotFound)
        }
    }

    /// Sets interface up/down state.
    pub fn set_up(&mut self, name: &str, up: bool) -> Result<(), NetError> {
        let iface = self.interfaces.get_mut(name).ok_or(NetError::NotFound)?;
        iface.up = up;
        Ok(())
    }

    /// Sets or clears an IPv4 address.
    pub fn set_ipv4(&mut self, name: &str, addr: Option<&str>) -> Result<(), NetError> {
        let iface = self.interfaces.get_mut(name).ok_or(NetError::NotFound)?;
        if let Some(addr) = addr {
            if !is_valid_ipv4(addr) {
                return Err(NetError::InvalidAddress);
            }
            iface.ipv4 = Some(addr.to_string());
        } else {
            iface.ipv4 = None;
        }
        Ok(())
    }

    /// Lists interfaces sorted by name.
    pub fn list(&self) -> Vec<NetInterface> {
        self.interfaces.values().cloned().collect()
    }
}

fn is_valid_iface_name(name: &str) -> bool {
    if name.is_empty() {
        return false;
    }
    name.chars()
        .all(|ch| ch.is_ascii_lowercase() || ch.is_ascii_digit() || ch == '-')
}

fn is_valid_ipv4(addr: &str) -> bool {
    let parts: Vec<&str> = addr.split('.').collect();
    if parts.len() != 4 {
        return false;
    }
    for part in parts {
        if part.is_empty() || part.len() > 3 {
            return false;
        }
        if let Ok(value) = part.parse::<u8>() {
            let normalized = value.to_string();
            if normalized != part && part.starts_with('0') && part != "0" {
                return false;
            }
        } else {
            return false;
        }
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn add_and_list_interfaces() {
        let mut manager = NetManager::new();
        manager.add_interface("eth0").unwrap();
        manager.add_interface("wlan0").unwrap();
        let list = manager.list();
        assert_eq!(list.len(), 2);
        assert_eq!(list[0].name, "eth0");
        assert_eq!(list[1].name, "wlan0");
    }

    #[test]
    fn add_rejects_invalid_names() {
        let mut manager = NetManager::new();
        assert_eq!(manager.add_interface(""), Err(NetError::InvalidName));
        assert_eq!(manager.add_interface("Eth0"), Err(NetError::InvalidName));
        assert_eq!(manager.add_interface("eth 0"), Err(NetError::InvalidName));
    }

    #[test]
    fn add_rejects_duplicates() {
        let mut manager = NetManager::new();
        manager.add_interface("eth0").unwrap();
        assert_eq!(manager.add_interface("eth0"), Err(NetError::AlreadyExists));
    }

    #[test]
    fn remove_interface() {
        let mut manager = NetManager::new();
        manager.add_interface("eth0").unwrap();
        manager.remove_interface("eth0").unwrap();
        assert_eq!(manager.remove_interface("eth0"), Err(NetError::NotFound));
    }

    #[test]
    fn set_up_down() {
        let mut manager = NetManager::new();
        manager.add_interface("eth0").unwrap();
        manager.set_up("eth0", true).unwrap();
        assert_eq!(manager.list()[0].up, true);
        manager.set_up("eth0", false).unwrap();
        assert_eq!(manager.list()[0].up, false);
    }

    #[test]
    fn set_up_requires_interface() {
        let mut manager = NetManager::new();
        assert_eq!(manager.set_up("eth0", true), Err(NetError::NotFound));
    }

    #[test]
    fn set_ipv4_and_clear() {
        let mut manager = NetManager::new();
        manager.add_interface("eth0").unwrap();
        manager.set_ipv4("eth0", Some("192.168.0.10")).unwrap();
        assert_eq!(manager.list()[0].ipv4, Some("192.168.0.10".to_string()));
        manager.set_ipv4("eth0", None).unwrap();
        assert_eq!(manager.list()[0].ipv4, None);
    }

    #[test]
    fn set_ipv4_rejects_invalid() {
        let mut manager = NetManager::new();
        manager.add_interface("eth0").unwrap();
        assert_eq!(
            manager.set_ipv4("eth0", Some("300.1.1.1")),
            Err(NetError::InvalidAddress)
        );
        assert_eq!(
            manager.set_ipv4("eth0", Some("10.0.0")),
            Err(NetError::InvalidAddress)
        );
        assert_eq!(
            manager.set_ipv4("eth0", Some("10.0.0.01")),
            Err(NetError::InvalidAddress)
        );
        assert_eq!(
            manager.set_ipv4("eth0", Some("10..0.1")),
            Err(NetError::InvalidAddress)
        );
        assert_eq!(
            manager.set_ipv4("eth0", Some("10.0.0.0000")),
            Err(NetError::InvalidAddress)
        );
    }

    #[test]
    fn set_ipv4_requires_interface() {
        let mut manager = NetManager::new();
        assert_eq!(
            manager.set_ipv4("eth0", Some("10.0.0.1")),
            Err(NetError::NotFound)
        );
    }
}
