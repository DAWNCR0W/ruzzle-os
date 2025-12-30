#![cfg_attr(not(test), no_std)]

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::string::{String, ToString};
use alloc::vec::Vec;

/// Container lifecycle state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContainerState {
    Created,
    Running,
    Stopped,
}

/// Container specification.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContainerSpec {
    pub name: String,
    pub image: String,
    pub command: Vec<String>,
    pub env: Vec<(String, String)>,
}

/// Container metadata entry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContainerInfo {
    pub spec: ContainerSpec,
    pub state: ContainerState,
}

/// Errors returned by the container service.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ContainerError {
    InvalidName,
    AlreadyExists,
    NotFound,
    AlreadyRunning,
    NotRunning,
}

/// In-memory container manager.
#[derive(Debug, Default, Clone)]
pub struct ContainerManager {
    containers: BTreeMap<String, ContainerInfo>,
}

impl ContainerManager {
    /// Creates an empty manager.
    pub fn new() -> Self {
        Self {
            containers: BTreeMap::new(),
        }
    }

    /// Registers a container spec in the created state.
    pub fn create(&mut self, spec: ContainerSpec) -> Result<(), ContainerError> {
        if !is_valid_name(&spec.name) {
            return Err(ContainerError::InvalidName);
        }
        if self.containers.contains_key(&spec.name) {
            return Err(ContainerError::AlreadyExists);
        }
        self.containers.insert(
            spec.name.clone(),
            ContainerInfo {
                spec,
                state: ContainerState::Created,
            },
        );
        Ok(())
    }

    /// Starts a container.
    pub fn start(&mut self, name: &str) -> Result<(), ContainerError> {
        let container = self
            .containers
            .get_mut(name)
            .ok_or(ContainerError::NotFound)?;
        if container.state == ContainerState::Running {
            return Err(ContainerError::AlreadyRunning);
        }
        container.state = ContainerState::Running;
        Ok(())
    }

    /// Stops a running container.
    pub fn stop(&mut self, name: &str) -> Result<(), ContainerError> {
        let container = self
            .containers
            .get_mut(name)
            .ok_or(ContainerError::NotFound)?;
        if container.state != ContainerState::Running {
            return Err(ContainerError::NotRunning);
        }
        container.state = ContainerState::Stopped;
        Ok(())
    }

    /// Removes a container and its metadata.
    pub fn remove(&mut self, name: &str) -> Result<(), ContainerError> {
        if self.containers.remove(name).is_some() {
            Ok(())
        } else {
            Err(ContainerError::NotFound)
        }
    }

    /// Returns the current state of a container.
    pub fn state(&self, name: &str) -> Result<ContainerState, ContainerError> {
        self.containers
            .get(name)
            .map(|info| info.state)
            .ok_or(ContainerError::NotFound)
    }

    /// Lists all containers sorted by name.
    pub fn list(&self) -> Vec<ContainerInfo> {
        self.containers.values().cloned().collect()
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

    fn spec(name: &str) -> ContainerSpec {
        ContainerSpec {
            name: name.to_string(),
            image: "base:latest".to_string(),
            command: vec!["/bin/app".to_string()],
            env: vec![("RUST_LOG".to_string(), "info".to_string())],
        }
    }

    #[test]
    fn create_and_list_containers() {
        let mut manager = ContainerManager::new();
        manager.create(spec("web")).unwrap();
        let list = manager.list();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].state, ContainerState::Created);
    }

    #[test]
    fn create_rejects_invalid_name() {
        let mut manager = ContainerManager::new();
        assert_eq!(manager.create(spec("BadName")), Err(ContainerError::InvalidName));
    }

    #[test]
    fn create_rejects_empty_name() {
        let mut manager = ContainerManager::new();
        assert_eq!(manager.create(spec("")), Err(ContainerError::InvalidName));
    }

    #[test]
    fn create_rejects_duplicates() {
        let mut manager = ContainerManager::new();
        manager.create(spec("api")).unwrap();
        assert_eq!(
            manager.create(spec("api")),
            Err(ContainerError::AlreadyExists)
        );
    }

    #[test]
    fn start_and_stop_container() {
        let mut manager = ContainerManager::new();
        manager.create(spec("worker")).unwrap();
        manager.start("worker").unwrap();
        assert_eq!(manager.state("worker").unwrap(), ContainerState::Running);
        manager.stop("worker").unwrap();
        assert_eq!(manager.state("worker").unwrap(), ContainerState::Stopped);
    }

    #[test]
    fn start_rejects_running() {
        let mut manager = ContainerManager::new();
        manager.create(spec("cache")).unwrap();
        manager.start("cache").unwrap();
        assert_eq!(
            manager.start("cache"),
            Err(ContainerError::AlreadyRunning)
        );
    }

    #[test]
    fn start_rejects_missing_container() {
        let mut manager = ContainerManager::new();
        assert_eq!(manager.start("missing"), Err(ContainerError::NotFound));
    }

    #[test]
    fn stop_rejects_non_running() {
        let mut manager = ContainerManager::new();
        manager.create(spec("db")).unwrap();
        assert_eq!(manager.stop("db"), Err(ContainerError::NotRunning));
    }

    #[test]
    fn stop_rejects_missing_container() {
        let mut manager = ContainerManager::new();
        assert_eq!(manager.stop("missing"), Err(ContainerError::NotFound));
    }

    #[test]
    fn remove_container() {
        let mut manager = ContainerManager::new();
        manager.create(spec("api")).unwrap();
        manager.remove("api").unwrap();
        assert_eq!(manager.remove("api"), Err(ContainerError::NotFound));
    }

    #[test]
    fn state_rejects_missing_container() {
        let manager = ContainerManager::new();
        assert_eq!(manager.state("missing"), Err(ContainerError::NotFound));
    }
}
