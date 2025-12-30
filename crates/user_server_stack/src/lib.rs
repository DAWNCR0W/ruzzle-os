#![cfg_attr(not(test), no_std)]

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::string::{String, ToString};
use alloc::vec::Vec;

/// Server configuration snapshot.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
    pub tls_enabled: bool,
    pub metrics_enabled: bool,
}

/// HTTP request model.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HttpRequest {
    pub method: String,
    pub path: String,
    pub body: String,
}

/// HTTP response model.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HttpResponse {
    pub status: u16,
    pub body: String,
}

/// Errors for the server stack.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ServerError {
    AlreadyRunning,
    NotRunning,
    RouteExists,
}

/// Simple server stack for in-memory routing.
#[derive(Debug, Clone)]
pub struct ServerStack {
    config: ServerConfig,
    routes: BTreeMap<(String, String), HttpResponse>,
    running: bool,
}

impl ServerStack {
    /// Creates a new server stack.
    pub fn new(config: ServerConfig) -> Self {
        Self {
            config,
            routes: BTreeMap::new(),
            running: false,
        }
    }

    /// Returns the current configuration.
    pub fn config(&self) -> &ServerConfig {
        &self.config
    }

    /// Registers a static route response.
    pub fn register_route(
        &mut self,
        method: &str,
        path: &str,
        response: HttpResponse,
    ) -> Result<(), ServerError> {
        let key = (method.to_string(), path.to_string());
        if self.routes.contains_key(&key) {
            return Err(ServerError::RouteExists);
        }
        self.routes.insert(key, response);
        Ok(())
    }

    /// Starts the server stack.
    pub fn start(&mut self) -> Result<(), ServerError> {
        if self.running {
            return Err(ServerError::AlreadyRunning);
        }
        self.running = true;
        Ok(())
    }

    /// Stops the server stack.
    pub fn stop(&mut self) -> Result<(), ServerError> {
        if !self.running {
            return Err(ServerError::NotRunning);
        }
        self.running = false;
        Ok(())
    }

    /// Handles a request with the registered routes.
    pub fn handle(&self, request: &HttpRequest) -> HttpResponse {
        let key = (request.method.clone(), request.path.clone());
        if let Some(response) = self.routes.get(&key) {
            return response.clone();
        }
        HttpResponse {
            status: 404,
            body: "not found".to_string(),
        }
    }

    /// Returns whether the server is running.
    pub fn is_running(&self) -> bool {
        self.running
    }

    /// Lists registered routes.
    pub fn list_routes(&self) -> Vec<(String, String)> {
        self.routes.keys().cloned().collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn config() -> ServerConfig {
        ServerConfig {
            host: "0.0.0.0".to_string(),
            port: 8080,
            tls_enabled: false,
            metrics_enabled: true,
        }
    }

    #[test]
    fn register_and_handle_route() {
        let mut server = ServerStack::new(config());
        server
            .register_route(
                "GET",
                "/",
                HttpResponse {
                    status: 200,
                    body: "ok".to_string(),
                },
            )
            .unwrap();
        let response = server.handle(&HttpRequest {
            method: "GET".to_string(),
            path: "/".to_string(),
            body: "".to_string(),
        });
        assert_eq!(response.status, 200);
    }

    #[test]
    fn register_route_rejects_duplicates() {
        let mut server = ServerStack::new(config());
        server
            .register_route(
                "GET",
                "/health",
                HttpResponse {
                    status: 200,
                    body: "ok".to_string(),
                },
            )
            .unwrap();
        assert_eq!(
            server.register_route(
                "GET",
                "/health",
                HttpResponse {
                    status: 503,
                    body: "oops".to_string(),
                },
            ),
            Err(ServerError::RouteExists)
        );
    }

    #[test]
    fn handle_missing_route_returns_404() {
        let server = ServerStack::new(config());
        let response = server.handle(&HttpRequest {
            method: "GET".to_string(),
            path: "/missing".to_string(),
            body: "".to_string(),
        });
        assert_eq!(response.status, 404);
    }

    #[test]
    fn start_and_stop_flow() {
        let mut server = ServerStack::new(config());
        server.start().unwrap();
        assert!(server.is_running());
        server.stop().unwrap();
        assert!(!server.is_running());
    }

    #[test]
    fn config_is_exposed() {
        let server = ServerStack::new(config());
        let cfg = server.config();
        assert_eq!(cfg.port, 8080);
        assert!(cfg.metrics_enabled);
    }

    #[test]
    fn list_routes_returns_keys() {
        let mut server = ServerStack::new(config());
        server
            .register_route(
                "GET",
                "/health",
                HttpResponse {
                    status: 200,
                    body: "ok".to_string(),
                },
            )
            .unwrap();
        let routes = server.list_routes();
        assert_eq!(routes, vec![("GET".to_string(), "/health".to_string())]);
    }

    #[test]
    fn start_rejects_when_running() {
        let mut server = ServerStack::new(config());
        server.start().unwrap();
        assert_eq!(server.start(), Err(ServerError::AlreadyRunning));
    }

    #[test]
    fn stop_rejects_when_stopped() {
        let mut server = ServerStack::new(config());
        assert_eq!(server.stop(), Err(ServerError::NotRunning));
    }
}
