#![cfg_attr(not(test), no_std)]

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::string::{String, ToString};
use alloc::vec::Vec;

use user_net_service::{NetError, NetManager, RouteError};

/// Supported network profiles.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NetProfile {
    Dhcp { iface: String },
    Static { iface: String, ipv4: String, gateway: Option<String> },
}

/// Errors raised by the net manager.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NetProfileError {
    InvalidName,
    AlreadyExists,
    NotFound,
    Net(NetError),
    Route(RouteError),
}

/// Profile collection and application logic.
#[derive(Debug, Default, Clone)]
pub struct NetProfileManager {
    profiles: BTreeMap<String, NetProfile>,
}

impl NetProfileManager {
    /// Creates an empty profile manager.
    pub fn new() -> Self {
        Self {
            profiles: BTreeMap::new(),
        }
    }

    /// Adds a new profile.
    pub fn add_profile(&mut self, name: &str, profile: NetProfile) -> Result<(), NetProfileError> {
        if !is_valid_name(name) {
            return Err(NetProfileError::InvalidName);
        }
        if self.profiles.contains_key(name) {
            return Err(NetProfileError::AlreadyExists);
        }
        self.profiles.insert(name.to_string(), profile);
        Ok(())
    }

    /// Removes a profile by name.
    pub fn remove_profile(&mut self, name: &str) -> Result<(), NetProfileError> {
        if self.profiles.remove(name).is_some() {
            Ok(())
        } else {
            Err(NetProfileError::NotFound)
        }
    }

    /// Applies a profile to the given network manager.
    pub fn apply_profile(
        &self,
        name: &str,
        net: &mut NetManager,
    ) -> Result<(), NetProfileError> {
        let profile = self.profiles.get(name).ok_or(NetProfileError::NotFound)?;
        match profile {
            NetProfile::Dhcp { iface } => {
                net.set_up(iface, true).map_err(NetProfileError::Net)?;
                let _ = net.set_ipv4(iface, None);
                Ok(())
            }
            NetProfile::Static {
                iface,
                ipv4,
                gateway,
            } => {
                net.set_up(iface, true).map_err(NetProfileError::Net)?;
                net.set_ipv4(iface, Some(ipv4))
                    .map_err(NetProfileError::Net)?;
                if gateway.is_some() {
                    net.add_route("default", iface)
                        .map_err(NetProfileError::Route)?;
                }
                Ok(())
            }
        }
    }

    /// Lists profile names.
    pub fn list_profiles(&self) -> Vec<String> {
        self.profiles.keys().cloned().collect()
    }
}

fn is_valid_name(name: &str) -> bool {
    if name.is_empty() {
        return false;
    }
    name.chars()
        .all(|ch| ch.is_ascii_lowercase() || ch.is_ascii_digit() || ch == '-')
}

#[cfg(test)]
mod tests {
    use super::*;

    fn manager_with_iface() -> NetManager {
        let mut net = NetManager::new();
        net.add_interface("eth0").unwrap();
        net
    }

    #[test]
    fn add_and_list_profiles() {
        let mut profiles = NetProfileManager::new();
        profiles
            .add_profile(
                "office",
                NetProfile::Static {
                    iface: "eth0".to_string(),
                    ipv4: "10.0.0.2".to_string(),
                    gateway: Some("10.0.0.1".to_string()),
                },
            )
            .unwrap();
        assert_eq!(profiles.list_profiles(), vec!["office".to_string()]);
    }

    #[test]
    fn add_profile_rejects_invalid_name() {
        let mut profiles = NetProfileManager::new();
        assert_eq!(
            profiles.add_profile(
                "Office",
                NetProfile::Dhcp {
                    iface: "eth0".to_string(),
                }
            ),
            Err(NetProfileError::InvalidName)
        );
    }

    #[test]
    fn add_profile_rejects_empty_name() {
        let mut profiles = NetProfileManager::new();
        assert_eq!(
            profiles.add_profile(
                "",
                NetProfile::Dhcp {
                    iface: "eth0".to_string(),
                }
            ),
            Err(NetProfileError::InvalidName)
        );
    }

    #[test]
    fn add_profile_accepts_hyphen_and_digits() {
        let mut profiles = NetProfileManager::new();
        profiles
            .add_profile(
                "net-1",
                NetProfile::Dhcp {
                    iface: "eth0".to_string(),
                },
            )
            .unwrap();
        assert_eq!(profiles.list_profiles(), vec!["net-1".to_string()]);
    }

    #[test]
    fn add_profile_rejects_duplicates() {
        let mut profiles = NetProfileManager::new();
        profiles
            .add_profile(
                "dhcp",
                NetProfile::Dhcp {
                    iface: "eth0".to_string(),
                },
            )
            .unwrap();
        assert_eq!(
            profiles.add_profile(
                "dhcp",
                NetProfile::Dhcp {
                    iface: "eth0".to_string(),
                },
            ),
            Err(NetProfileError::AlreadyExists)
        );
    }

    #[test]
    fn apply_dhcp_profile_clears_ip() {
        let mut profiles = NetProfileManager::new();
        profiles
            .add_profile(
                "dhcp",
                NetProfile::Dhcp {
                    iface: "eth0".to_string(),
                },
            )
            .unwrap();
        let mut net = manager_with_iface();
        net.set_ipv4("eth0", Some("10.0.0.2")).unwrap();
        profiles.apply_profile("dhcp", &mut net).unwrap();
        let iface = net.list().pop().unwrap();
        assert!(iface.ipv4.is_none());
        assert!(iface.up);
    }

    #[test]
    fn apply_dhcp_profile_rejects_missing_interface() {
        let mut profiles = NetProfileManager::new();
        profiles
            .add_profile(
                "dhcp",
                NetProfile::Dhcp {
                    iface: "eth0".to_string(),
                },
            )
            .unwrap();
        let mut net = NetManager::new();
        assert_eq!(
            profiles.apply_profile("dhcp", &mut net),
            Err(NetProfileError::Net(NetError::NotFound))
        );
    }

    #[test]
    fn apply_static_profile_sets_route() {
        let mut profiles = NetProfileManager::new();
        profiles
            .add_profile(
                "static",
                NetProfile::Static {
                    iface: "eth0".to_string(),
                    ipv4: "10.0.0.3".to_string(),
                    gateway: Some("10.0.0.1".to_string()),
                },
            )
            .unwrap();
        let mut net = manager_with_iface();
        profiles.apply_profile("static", &mut net).unwrap();
        assert_eq!(net.list_routes().len(), 1);
    }

    #[test]
    fn apply_static_profile_rejects_invalid_ipv4() {
        let mut profiles = NetProfileManager::new();
        profiles
            .add_profile(
                "static",
                NetProfile::Static {
                    iface: "eth0".to_string(),
                    ipv4: "999.0.0.1".to_string(),
                    gateway: None,
                },
            )
            .unwrap();
        let mut net = manager_with_iface();
        assert_eq!(
            profiles.apply_profile("static", &mut net),
            Err(NetProfileError::Net(NetError::InvalidAddress))
        );
    }

    #[test]
    fn apply_static_profile_rejects_missing_interface() {
        let mut profiles = NetProfileManager::new();
        profiles
            .add_profile(
                "static",
                NetProfile::Static {
                    iface: "eth0".to_string(),
                    ipv4: "10.0.0.10".to_string(),
                    gateway: None,
                },
            )
            .unwrap();
        let mut net = NetManager::new();
        assert_eq!(
            profiles.apply_profile("static", &mut net),
            Err(NetProfileError::Net(NetError::NotFound))
        );
    }

    #[test]
    fn apply_static_profile_without_gateway() {
        let mut profiles = NetProfileManager::new();
        profiles
            .add_profile(
                "static",
                NetProfile::Static {
                    iface: "eth0".to_string(),
                    ipv4: "10.0.0.3".to_string(),
                    gateway: None,
                },
            )
            .unwrap();
        let mut net = manager_with_iface();
        profiles.apply_profile("static", &mut net).unwrap();
        assert!(net.list_routes().is_empty());
    }

    #[test]
    fn apply_static_profile_rejects_route_error() {
        let mut profiles = NetProfileManager::new();
        profiles
            .add_profile(
                "static",
                NetProfile::Static {
                    iface: "eth0".to_string(),
                    ipv4: "10.0.0.10".to_string(),
                    gateway: Some("10.0.0.1".to_string()),
                },
            )
            .unwrap();
        let mut net = manager_with_iface();
        net.add_route("default", "eth0").unwrap();
        assert_eq!(
            profiles.apply_profile("static", &mut net),
            Err(NetProfileError::Route(RouteError::AlreadyExists))
        );
    }

    #[test]
    fn apply_profile_rejects_missing() {
        let profiles = NetProfileManager::new();
        let mut net = manager_with_iface();
        assert_eq!(
            profiles.apply_profile("missing", &mut net),
            Err(NetProfileError::NotFound)
        );
    }

    #[test]
    fn remove_profile() {
        let mut profiles = NetProfileManager::new();
        profiles
            .add_profile(
                "dhcp",
                NetProfile::Dhcp {
                    iface: "eth0".to_string(),
                },
            )
            .unwrap();
        profiles.remove_profile("dhcp").unwrap();
        assert_eq!(
            profiles.remove_profile("dhcp"),
            Err(NetProfileError::NotFound)
        );
    }
}
