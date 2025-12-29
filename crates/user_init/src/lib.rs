#![cfg_attr(not(test), no_std)]

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::string::{String, ToString};
use alloc::vec::Vec;

use hal::Errno;
use ruzzle_protocol::registry::{
    decode_request, encode_response, RegistryRequest, RegistryResponse, RegistryStatus, ServiceEntry,
};

/// Describes a user module and its dependencies.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModuleInfo {
    pub name: String,
    pub depends: Vec<String>,
}

/// Resolves module start order using a simple topological sort.
///
/// Returns an ordered list of module names or Errno::InvalidArg
/// when a dependency cycle is detected.
pub fn resolve_start_order(modules: &[ModuleInfo]) -> Result<Vec<String>, Errno> {
    let mut remaining: BTreeMap<String, Vec<String>> = modules
        .iter()
        .map(|module| (module.name.clone(), module.depends.clone()))
        .collect();

    let mut order = Vec::new();

    loop {
        let ready = remaining
            .iter()
            .find(|(_, deps)| deps.is_empty())
            .map(|(name, _)| name.clone());

        let Some(name) = ready else {
            break;
        };

        remaining.remove(&name);
        order.push(name.clone());

        for deps in remaining.values_mut() {
            deps.retain(|dep| dep != &name);
        }
    }

    if remaining.is_empty() {
        Ok(order)
    } else {
        Err(Errno::InvalidArg)
    }
}

/// Validates the canonical service naming rule.
pub fn is_valid_service_name(name: &str) -> bool {
    let mut parts = name.split('.');
    let prefix = parts.next().unwrap_or("");
    if prefix != "ruzzle" {
        return false;
    }

    let mut saw_segment = false;
    for segment in parts {
        if segment.is_empty() {
            return false;
        }
        if !segment
            .chars()
            .all(|ch| ch.is_ascii_lowercase() || ch.is_ascii_digit() || ch == '-')
        {
            return false;
        }
        saw_segment = true;
    }

    saw_segment
}

/// Registry mapping service names to module names.
#[derive(Debug, Default)]
pub struct ServiceRegistry {
    services: BTreeMap<String, String>,
}

impl ServiceRegistry {
    /// Creates an empty service registry.
    pub fn new() -> Self {
        Self {
            services: BTreeMap::new(),
        }
    }

    /// Returns true if a service name is registered.
    pub fn contains(&self, service: &str) -> bool {
        self.services.contains_key(service)
    }

    /// Registers a service name for a module.
    pub fn register(&mut self, service: String, module: String) -> Result<(), Errno> {
        if service.is_empty() || module.is_empty() {
            return Err(Errno::InvalidArg);
        }
        if !is_valid_service_name(&service) {
            return Err(Errno::InvalidArg);
        }
        if self.services.contains_key(&service) {
            return Err(Errno::InvalidArg);
        }
        self.services.insert(service, module);
        Ok(())
    }

    /// Unregisters a service name.
    pub fn unregister(&mut self, service: &str) -> Result<(), Errno> {
        if self.services.remove(service).is_some() {
            Ok(())
        } else {
            Err(Errno::NotFound)
        }
    }

    /// Removes all services owned by a module and returns the count removed.
    pub fn unregister_module(&mut self, module: &str) -> usize {
        let keys: Vec<String> = self
            .services
            .iter()
            .filter_map(|(service, owner)| {
                if owner == module {
                    Some(service.clone())
                } else {
                    None
                }
            })
            .collect();
        let count = keys.len();
        for key in keys {
            self.services.remove(&key);
        }
        count
    }

    /// Resolves a service name to its owning module.
    pub fn resolve(&self, service: &str) -> Result<&str, Errno> {
        self.services
            .get(service)
            .map(|name| name.as_str())
            .ok_or(Errno::NotFound)
    }

    /// Returns all registered services sorted by name.
    pub fn list(&self) -> Vec<ServiceEntry> {
        self.services
            .iter()
            .map(|(service, module)| ServiceEntry {
                service: service.clone(),
                module: module.clone(),
            })
            .collect()
    }
}

/// Module lifecycle state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModuleState {
    Stopped,
    Running,
    Failed,
}

/// Full module metadata tracked by init.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModuleRecord {
    pub name: String,
    pub depends: Vec<String>,
    pub provides: Vec<String>,
    pub requires_caps: Vec<String>,
    pub state: ModuleState,
}

impl ModuleRecord {
    /// Creates a stopped module record with the supplied metadata.
    pub fn new(
        name: String,
        depends: Vec<String>,
        provides: Vec<String>,
        requires_caps: Vec<String>,
    ) -> Self {
        Self {
            name,
            depends,
            provides,
            requires_caps,
            state: ModuleState::Stopped,
        }
    }
}

/// Summary view of a module for UI presentation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModuleSummary {
    pub name: String,
    pub state: ModuleState,
}

/// Manages module lifecycle and service registration.
#[derive(Debug, Default)]
pub struct ModuleManager {
    modules: BTreeMap<String, ModuleRecord>,
    registry: ServiceRegistry,
}

impl ModuleManager {
    /// Creates an empty module manager.
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns the service registry.
    pub fn service_registry(&self) -> &ServiceRegistry {
        &self.registry
    }

    /// Registers a module definition without starting it.
    pub fn register_module(&mut self, record: ModuleRecord) -> Result<(), Errno> {
        if record.name.is_empty() {
            return Err(Errno::InvalidArg);
        }
        if self.modules.contains_key(&record.name) {
            return Err(Errno::InvalidArg);
        }
        if record.depends.iter().any(|dep| dep == &record.name) {
            return Err(Errno::InvalidArg);
        }
        for service in &record.provides {
            if !is_valid_service_name(service) {
                return Err(Errno::InvalidArg);
            }
        }
        for cap in &record.requires_caps {
            if cap.is_empty() {
                return Err(Errno::InvalidArg);
            }
        }
        self.modules.insert(record.name.clone(), record);
        Ok(())
    }

    /// Starts a module after validating dependencies and service ownership.
    pub fn start_module(&mut self, name: &str) -> Result<(), Errno> {
        let (current_state, depends, provides, module_name) = {
            let record = self.modules.get(name).ok_or(Errno::NotFound)?;
            (
                record.state,
                record.depends.clone(),
                record.provides.clone(),
                record.name.clone(),
            )
        };

        match current_state {
            ModuleState::Running => return Ok(()),
            ModuleState::Failed => return Err(Errno::InvalidArg),
            ModuleState::Stopped => {}
        }

        for dep in &depends {
            let dep_record = self.modules.get(dep).ok_or(Errno::NotFound)?;
            if dep_record.state != ModuleState::Running {
                return Err(Errno::InvalidArg);
            }
        }

        if provides.iter().any(|service| self.registry.contains(service)) {
            let record = self.modules.get_mut(name).expect("module exists");
            record.state = ModuleState::Failed;
            return Err(Errno::InvalidArg);
        }

        for service in &provides {
            self.registry
                .register(service.clone(), module_name.clone())?;
        }

        let record = self.modules.get_mut(name).expect("module exists");
        record.state = ModuleState::Running;
        Ok(())
    }

    /// Stops a running module and unregisters its services.
    pub fn stop_module(&mut self, name: &str) -> Result<(), Errno> {
        let record = self.modules.get_mut(name).ok_or(Errno::NotFound)?;
        if record.state != ModuleState::Running {
            return Err(Errno::InvalidArg);
        }
        record.state = ModuleState::Stopped;
        self.registry.unregister_module(&record.name);
        Ok(())
    }

    /// Restarts a module, marking it failed on start errors.
    pub fn restart_module(&mut self, name: &str) -> Result<(), Errno> {
        let current_state = self
            .modules
            .get(name)
            .map(|record| record.state)
            .ok_or(Errno::NotFound)?;

        if current_state == ModuleState::Running {
            let record = self.modules.get_mut(name).expect("module exists");
            record.state = ModuleState::Stopped;
            self.registry.unregister_module(&record.name);
        }

        match self.start_module(name) {
            Ok(()) => Ok(()),
            Err(err) => {
                let record = self.modules.get_mut(name).expect("module exists");
                record.state = ModuleState::Failed;
                Err(err)
            }
        }
    }

    /// Lists modules for UI rendering.
    pub fn list_modules(&self) -> Vec<ModuleSummary> {
        self.modules
            .values()
            .map(|record| ModuleSummary {
                name: record.name.clone(),
                state: record.state,
            })
            .collect()
    }

    /// Resolves a start plan based on dependency order.
    pub fn resolve_start_plan(&self) -> Result<Vec<String>, Errno> {
        let modules: Vec<ModuleInfo> = self
            .modules
            .values()
            .map(|record| ModuleInfo {
                name: record.name.clone(),
                depends: record.depends.clone(),
            })
            .collect();
        resolve_start_order(&modules)
    }
}

/// Handles a registry request and returns the response.
pub fn handle_registry_request(
    registry: &mut ServiceRegistry,
    request: RegistryRequest,
) -> RegistryResponse {
    match request {
        RegistryRequest::Register { service, module } => {
            if !is_valid_service_name(&service) {
                return RegistryResponse::Error {
                    status: RegistryStatus::Invalid,
                };
            }
            let service_name = service.clone();
            match registry.register(service, module) {
                Ok(()) => RegistryResponse::Ack,
                Err(_) => {
                    if registry.contains(&service_name) {
                        RegistryResponse::Error {
                            status: RegistryStatus::AlreadyExists,
                        }
                    } else {
                        RegistryResponse::Error {
                            status: RegistryStatus::Invalid,
                        }
                    }
                }
            }
        }
        RegistryRequest::Lookup { service } => {
            if !is_valid_service_name(&service) {
                return RegistryResponse::Error {
                    status: RegistryStatus::Invalid,
                };
            }
            if let Ok(module) = registry.resolve(&service) {
                RegistryResponse::Lookup {
                    status: RegistryStatus::Ok,
                    module: Some(module.to_string()),
                }
            } else {
                RegistryResponse::Lookup {
                    status: RegistryStatus::NotFound,
                    module: None,
                }
            }
        }
        RegistryRequest::List => RegistryResponse::List {
            status: RegistryStatus::Ok,
            entries: registry.list(),
        },
    }
}

/// Decodes a registry request, handles it, and encodes the response.
pub fn handle_registry_request_bytes(
    registry: &mut ServiceRegistry,
    bytes: &[u8],
) -> Vec<u8> {
    let response = match decode_request(bytes) {
        Ok(request) => handle_registry_request(registry, request),
        Err(_) => RegistryResponse::Error {
            status: RegistryStatus::Invalid,
        },
    };
    encode_response(&response)
}

#[cfg(test)]
mod tests {
    use super::*;
    use ruzzle_protocol::registry::{decode_response, encode_request};

    #[test]
    fn resolve_start_order_sorts_dependencies() {
        let modules = vec![
            ModuleInfo {
                name: "tui".into(),
                depends: vec!["console".into()],
            },
            ModuleInfo {
                name: "console".into(),
                depends: vec![],
            },
            ModuleInfo {
                name: "init".into(),
                depends: vec![],
            },
        ];

        let order = resolve_start_order(&modules).expect("order should resolve");
        let init_index = order.iter().position(|name| name == "init").unwrap();
        let console_index = order.iter().position(|name| name == "console").unwrap();
        let tui_index = order.iter().position(|name| name == "tui").unwrap();
        assert!(console_index < tui_index);
        assert!(init_index < order.len());
    }

    #[test]
    fn resolve_start_order_detects_cycles() {
        let modules = vec![
            ModuleInfo {
                name: "a".into(),
                depends: vec!["b".into()],
            },
            ModuleInfo {
                name: "b".into(),
                depends: vec!["a".into()],
            },
        ];

        let result = resolve_start_order(&modules);
        assert_eq!(result, Err(Errno::InvalidArg));
    }

    #[test]
    fn service_name_validation_rules() {
        assert!(is_valid_service_name("ruzzle.console"));
        assert!(is_valid_service_name("ruzzle.fs.readonly"));
        assert!(!is_valid_service_name("console"));
        assert!(!is_valid_service_name("ruzzle."));
        assert!(!is_valid_service_name("ruzzle.Console"));
        assert!(!is_valid_service_name("ruzzle..bad"));
    }

    #[test]
    fn service_registry_registers_and_resolves() {
        let mut registry = ServiceRegistry::new();
        registry
            .register("ruzzle.console".into(), "console-service".into())
            .expect("register should succeed");

        let module = registry.resolve("ruzzle.console").expect("resolve should succeed");
        assert_eq!(module, "console-service");
    }

    #[test]
    fn service_registry_rejects_duplicates_and_missing() {
        let mut registry = ServiceRegistry::new();
        registry
            .register("ruzzle.console".into(), "console-service".into())
            .expect("register should succeed");

        let result = registry.register("ruzzle.console".into(), "other".into());
        assert_eq!(result, Err(Errno::InvalidArg));

        let result = registry.resolve("ruzzle.missing");
        assert_eq!(result, Err(Errno::NotFound));
    }

    #[test]
    fn service_registry_unregisters() {
        let mut registry = ServiceRegistry::new();
        registry
            .register("ruzzle.console".into(), "console-service".into())
            .expect("register should succeed");
        assert!(registry.unregister("ruzzle.console").is_ok());
        assert_eq!(registry.unregister("ruzzle.console"), Err(Errno::NotFound));
    }

    #[test]
    fn service_registry_unregisters_module() {
        let mut registry = ServiceRegistry::new();
        registry
            .register("ruzzle.console".into(), "console-service".into())
            .expect("register should succeed");
        registry
            .register("ruzzle.shell".into(), "tui-shell".into())
            .expect("register should succeed");
        let removed = registry.unregister_module("console-service");
        assert_eq!(removed, 1);
        assert_eq!(registry.resolve("ruzzle.console"), Err(Errno::NotFound));
        assert_eq!(registry.resolve("ruzzle.shell"), Ok("tui-shell"));
    }

    #[test]
    fn service_registry_rejects_empty_or_invalid_service() {
        let mut registry = ServiceRegistry::new();
        assert_eq!(
            registry.register("".into(), "module".into()),
            Err(Errno::InvalidArg)
        );
        assert_eq!(
            registry.register("ruzzle.console".into(), "".into()),
            Err(Errno::InvalidArg)
        );
        assert_eq!(
            registry.register("invalid".into(), "module".into()),
            Err(Errno::InvalidArg)
        );
    }

    #[test]
    fn module_manager_registers_and_starts_modules() {
        let mut manager = ModuleManager::new();
        manager
            .register_module(ModuleRecord::new(
                "console-service".to_string(),
                vec![],
                vec!["ruzzle.console".to_string()],
                vec!["ConsoleWrite".to_string()],
            ))
            .expect("register should succeed");

        manager
            .start_module("console-service")
            .expect("start should succeed");
        assert_eq!(
            manager
                .service_registry()
                .resolve("ruzzle.console")
                .unwrap(),
            "console-service"
        );

        manager
            .stop_module("console-service")
            .expect("stop should succeed");
        assert_eq!(
            manager.service_registry().resolve("ruzzle.console"),
            Err(Errno::NotFound)
        );
    }

    #[test]
    fn module_manager_rejects_missing_dependency() {
        let mut manager = ModuleManager::new();
        manager
            .register_module(ModuleRecord::new(
                "tui-shell".to_string(),
                vec!["console-service".to_string()],
                vec!["ruzzle.shell".to_string()],
                vec![],
            ))
            .expect("register should succeed");

        let result = manager.start_module("tui-shell");
        assert_eq!(result, Err(Errno::NotFound));
    }

    #[test]
    fn module_manager_rejects_invalid_records() {
        let mut manager = ModuleManager::new();
        assert_eq!(
            manager.register_module(ModuleRecord::new(
                "".to_string(),
                vec![],
                vec![],
                vec![]
            )),
            Err(Errno::InvalidArg)
        );
        manager
            .register_module(ModuleRecord::new(
                "init".to_string(),
                vec![],
                vec![],
                vec![],
            ))
            .unwrap();
        assert_eq!(
            manager.register_module(ModuleRecord::new(
                "init".to_string(),
                vec![],
                vec![],
                vec![],
            )),
            Err(Errno::InvalidArg)
        );
        assert_eq!(
            manager.register_module(ModuleRecord::new(
                "self".to_string(),
                vec!["self".to_string()],
                vec![],
                vec![],
            )),
            Err(Errno::InvalidArg)
        );
        assert_eq!(
            manager.register_module(ModuleRecord::new(
                "bad-service".to_string(),
                vec![],
                vec!["invalid".to_string()],
                vec![],
            )),
            Err(Errno::InvalidArg)
        );
        assert_eq!(
            manager.register_module(ModuleRecord::new(
                "bad-cap".to_string(),
                vec![],
                vec![],
                vec!["".to_string()],
            )),
            Err(Errno::InvalidArg)
        );
    }

    #[test]
    fn module_manager_start_and_stop_errors() {
        let mut manager = ModuleManager::new();
        assert_eq!(manager.start_module("missing"), Err(Errno::NotFound));

        manager
            .register_module(ModuleRecord::new(
                "init".to_string(),
                vec![],
                vec![],
                vec![],
            ))
            .unwrap();
        assert_eq!(manager.stop_module("init"), Err(Errno::InvalidArg));
        assert_eq!(manager.stop_module("missing"), Err(Errno::NotFound));
    }

    #[test]
    fn module_manager_start_handles_running_and_failed_states() {
        let mut manager = ModuleManager::new();
        manager
            .register_module(ModuleRecord::new(
                "mod-a".to_string(),
                vec![],
                vec!["ruzzle.console".to_string()],
                vec![],
            ))
            .unwrap();
        manager
            .register_module(ModuleRecord::new(
                "mod-b".to_string(),
                vec![],
                vec!["ruzzle.console".to_string()],
                vec![],
            ))
            .unwrap();
        manager.start_module("mod-a").unwrap();
        assert_eq!(manager.start_module("mod-a"), Ok(()));
        let _ = manager.start_module("mod-b");
        assert_eq!(manager.start_module("mod-b"), Err(Errno::InvalidArg));
    }

    #[test]
    fn module_manager_start_propagates_registry_error() {
        let mut manager = ModuleManager::new();
        manager.modules.insert(
            "bad".to_string(),
            ModuleRecord::new("bad".to_string(), vec![], vec!["invalid".to_string()], vec![]),
        );
        let result = manager.start_module("bad");
        assert_eq!(result, Err(Errno::InvalidArg));
    }

    #[test]
    fn module_manager_restart_running_module() {
        let mut manager = ModuleManager::new();
        manager
            .register_module(ModuleRecord::new(
                "console-service".to_string(),
                vec![],
                vec!["ruzzle.console".to_string()],
                vec![],
            ))
            .unwrap();
        manager.start_module("console-service").unwrap();
        manager.restart_module("console-service").unwrap();
        let state = manager
            .modules
            .get("console-service")
            .map(|record| record.state)
            .unwrap();
        assert_eq!(state, ModuleState::Running);
    }

    #[test]
    fn module_manager_restart_missing_module_returns_not_found() {
        let mut manager = ModuleManager::new();
        let result = manager.restart_module("missing");
        assert_eq!(result, Err(Errno::NotFound));
    }

    #[test]
    fn module_manager_lists_modules() {
        let mut manager = ModuleManager::new();
        manager
            .register_module(ModuleRecord::new(
                "a".to_string(),
                vec![],
                vec![],
                vec![],
            ))
            .unwrap();
        let list = manager.list_modules();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].name, "a");
    }

    #[test]
    fn module_manager_requires_dependencies_running() {
        let mut manager = ModuleManager::new();
        manager
            .register_module(ModuleRecord::new(
                "console-service".to_string(),
                vec![],
                vec!["ruzzle.console".to_string()],
                vec![],
            ))
            .expect("register should succeed");
        manager
            .register_module(ModuleRecord::new(
                "tui-shell".to_string(),
                vec!["console-service".to_string()],
                vec!["ruzzle.shell".to_string()],
                vec![],
            ))
            .expect("register should succeed");

        assert_eq!(manager.start_module("tui-shell"), Err(Errno::InvalidArg));
        manager.start_module("console-service").unwrap();
        manager.start_module("tui-shell").unwrap();
    }

    #[test]
    fn module_manager_restart_marks_failed_on_error() {
        let mut manager = ModuleManager::new();
        manager
            .register_module(ModuleRecord::new(
                "mod-a".to_string(),
                vec![],
                vec!["ruzzle.console".to_string()],
                vec![],
            ))
            .expect("register should succeed");
        manager
            .register_module(ModuleRecord::new(
                "mod-b".to_string(),
                vec![],
                vec!["ruzzle.console".to_string()],
                vec![],
            ))
            .expect("register should succeed");

        manager.start_module("mod-a").unwrap();
        let result = manager.restart_module("mod-b");
        assert_eq!(result, Err(Errno::InvalidArg));
        let state = manager
            .modules
            .get("mod-b")
            .map(|record| record.state)
            .unwrap();
        assert_eq!(state, ModuleState::Failed);
    }

    #[test]
    fn module_manager_resolves_start_plan() {
        let mut manager = ModuleManager::new();
        manager
            .register_module(ModuleRecord::new(
                "a".to_string(),
                vec![],
                vec![],
                vec![],
            ))
            .unwrap();
        manager
            .register_module(ModuleRecord::new(
                "b".to_string(),
                vec!["a".to_string()],
                vec![],
                vec![],
            ))
            .unwrap();
        let order = manager.resolve_start_plan().unwrap();
        assert_eq!(order, vec!["a".to_string(), "b".to_string()]);
    }

    #[test]
    fn handle_registry_register_and_lookup() {
        let mut registry = ServiceRegistry::new();
        let response = handle_registry_request(
            &mut registry,
            RegistryRequest::Register {
                service: "ruzzle.console".to_string(),
                module: "console-service".to_string(),
            },
        );
        assert_eq!(response, RegistryResponse::Ack);

        let response = handle_registry_request(
            &mut registry,
            RegistryRequest::Lookup {
                service: "ruzzle.console".to_string(),
            },
        );
        assert_eq!(
            response,
            RegistryResponse::Lookup {
                status: RegistryStatus::Ok,
                module: Some("console-service".to_string())
            }
        );
    }

    #[test]
    fn handle_registry_register_duplicate() {
        let mut registry = ServiceRegistry::new();
        registry
            .register("ruzzle.console".into(), "console-service".into())
            .unwrap();
        let response = handle_registry_request(
            &mut registry,
            RegistryRequest::Register {
                service: "ruzzle.console".to_string(),
                module: "other".to_string(),
            },
        );
        assert_eq!(
            response,
            RegistryResponse::Error {
                status: RegistryStatus::AlreadyExists
            }
        );
    }

    #[test]
    fn handle_registry_register_invalid_input() {
        let mut registry = ServiceRegistry::new();
        let response = handle_registry_request(
            &mut registry,
            RegistryRequest::Register {
                service: "invalid".to_string(),
                module: "console-service".to_string(),
            },
        );
        assert_eq!(
            response,
            RegistryResponse::Error {
                status: RegistryStatus::Invalid
            }
        );
    }

    #[test]
    fn handle_registry_register_empty_module_is_invalid() {
        let mut registry = ServiceRegistry::new();
        let response = handle_registry_request(
            &mut registry,
            RegistryRequest::Register {
                service: "ruzzle.console".to_string(),
                module: "".to_string(),
            },
        );
        assert_eq!(
            response,
            RegistryResponse::Error {
                status: RegistryStatus::Invalid
            }
        );
    }

    #[test]
    fn handle_registry_lookup_invalid_service() {
        let mut registry = ServiceRegistry::new();
        let response = handle_registry_request(
            &mut registry,
            RegistryRequest::Lookup {
                service: "invalid".to_string(),
            },
        );
        assert_eq!(
            response,
            RegistryResponse::Error {
                status: RegistryStatus::Invalid
            }
        );
    }

    #[test]
    fn handle_registry_lookup_missing() {
        let mut registry = ServiceRegistry::new();
        let response = handle_registry_request(
            &mut registry,
            RegistryRequest::Lookup {
                service: "ruzzle.console".to_string(),
            },
        );
        assert_eq!(
            response,
            RegistryResponse::Lookup {
                status: RegistryStatus::NotFound,
                module: None
            }
        );
    }

    #[test]
    fn handle_registry_list_returns_entries() {
        let mut registry = ServiceRegistry::new();
        registry
            .register("ruzzle.console".into(), "console-service".into())
            .unwrap();
        let response = handle_registry_request(&mut registry, RegistryRequest::List);
        assert_eq!(
            response,
            RegistryResponse::List {
                status: RegistryStatus::Ok,
                entries: vec![ServiceEntry {
                    service: "ruzzle.console".into(),
                    module: "console-service".into()
                }]
            }
        );
    }

    #[test]
    fn handle_registry_request_bytes_handles_invalid_payload() {
        let mut registry = ServiceRegistry::new();
        let bytes = handle_registry_request_bytes(&mut registry, &[]);
        let response = decode_response(&bytes).expect("decode should succeed");
        assert_eq!(
            response,
            RegistryResponse::Error {
                status: RegistryStatus::Invalid
            }
        );
    }

    #[test]
    fn handle_registry_request_bytes_roundtrip() {
        let mut registry = ServiceRegistry::new();
        let request = RegistryRequest::Register {
            service: "ruzzle.shell".to_string(),
            module: "tui-shell".to_string(),
        };
        let bytes = encode_request(&request);
        let response_bytes = handle_registry_request_bytes(&mut registry, &bytes);
        let response = decode_response(&response_bytes).expect("decode should succeed");
        assert_eq!(response, RegistryResponse::Ack);
    }
}
