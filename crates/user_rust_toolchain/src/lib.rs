#![cfg_attr(not(test), no_std)]

extern crate alloc;

use alloc::string::{String, ToString};
use alloc::vec::Vec;

/// Describes a Rust toolchain available on the host.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Toolchain {
    version: String,
    host: String,
    targets: Vec<String>,
}

/// Build specification for a piece crate.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BuildSpec {
    pub crate_name: String,
    pub target: String,
    pub release: bool,
}

/// Planned host build command and output.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BuildPlan {
    pub command: String,
    pub output: String,
}

/// Errors from toolchain planning.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ToolchainError {
    InvalidName,
    UnsupportedTarget,
}

impl Toolchain {
    /// Builds a new toolchain snapshot.
    pub fn new(version: &str, host: &str, targets: &[&str]) -> Self {
        Self {
            version: version.to_string(),
            host: host.to_string(),
            targets: targets.iter().map(|t| t.to_string()).collect(),
        }
    }

    /// Returns the toolchain version string.
    pub fn version(&self) -> &str {
        &self.version
    }

    /// Returns the host triple.
    pub fn host(&self) -> &str {
        &self.host
    }

    /// Returns true if a target is supported.
    pub fn supports_target(&self, target: &str) -> bool {
        self.targets.iter().any(|item| item == target)
    }

    /// Builds a host-side build plan for packaging a piece.
    pub fn plan_build(&self, spec: &BuildSpec) -> Result<BuildPlan, ToolchainError> {
        if !is_valid_crate_name(&spec.crate_name) {
            return Err(ToolchainError::InvalidName);
        }
        if !self.supports_target(&spec.target) {
            return Err(ToolchainError::UnsupportedTarget);
        }
        let mut command = String::from("cargo build");
        if spec.release {
            command.push_str(" --release");
        }
        command.push_str(" --target ");
        command.push_str(&spec.target);
        command.push_str(" -p ");
        command.push_str(&spec.crate_name);

        let profile = if spec.release { "release" } else { "debug" };
        let mut output = String::from("target/");
        output.push_str(&spec.target);
        output.push('/');
        output.push_str(profile);
        output.push('/');
        output.push_str(&spec.crate_name);

        Ok(BuildPlan { command, output })
    }
}

fn is_valid_crate_name(name: &str) -> bool {
    if name.is_empty() {
        return false;
    }
    name.chars()
        .all(|ch| ch.is_ascii_lowercase() || ch.is_ascii_digit() || ch == '-')
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn toolchain_reports_metadata() {
        let toolchain = Toolchain::new("1.78.0", "x86_64", &["x86_64-unknown-none"]);
        assert_eq!(toolchain.version(), "1.78.0");
        assert_eq!(toolchain.host(), "x86_64");
    }

    #[test]
    fn supports_target_matches() {
        let toolchain = Toolchain::new("1.78.0", "x86_64", &["x86_64-unknown-none"]);
        assert!(toolchain.supports_target("x86_64-unknown-none"));
        assert!(!toolchain.supports_target("aarch64-unknown-none"));
    }

    #[test]
    fn plan_build_rejects_invalid_name() {
        let toolchain = Toolchain::new("1.78.0", "x86_64", &["x86_64-unknown-none"]);
        let spec = BuildSpec {
            crate_name: "BadName".to_string(),
            target: "x86_64-unknown-none".to_string(),
            release: true,
        };
        assert_eq!(
            toolchain.plan_build(&spec),
            Err(ToolchainError::InvalidName)
        );
    }

    #[test]
    fn plan_build_rejects_unsupported_target() {
        let toolchain = Toolchain::new("1.78.0", "x86_64", &["x86_64-unknown-none"]);
        let spec = BuildSpec {
            crate_name: "demo-piece".to_string(),
            target: "aarch64-unknown-none".to_string(),
            release: false,
        };
        assert_eq!(
            toolchain.plan_build(&spec),
            Err(ToolchainError::UnsupportedTarget)
        );
    }

    #[test]
    fn plan_build_emits_release_command() {
        let toolchain = Toolchain::new("1.78.0", "x86_64", &["x86_64-unknown-none"]);
        let spec = BuildSpec {
            crate_name: "demo-piece".to_string(),
            target: "x86_64-unknown-none".to_string(),
            release: true,
        };
        let plan = toolchain.plan_build(&spec).unwrap();
        assert!(plan.command.contains("--release"));
        assert!(plan.command.contains("-p demo-piece"));
        assert!(plan.output.ends_with("/release/demo-piece"));
    }

    #[test]
    fn plan_build_emits_debug_command() {
        let toolchain = Toolchain::new("1.78.0", "x86_64", &["x86_64-unknown-none"]);
        let spec = BuildSpec {
            crate_name: "demo-piece".to_string(),
            target: "x86_64-unknown-none".to_string(),
            release: false,
        };
        let plan = toolchain.plan_build(&spec).unwrap();
        assert!(!plan.command.contains("--release"));
        assert!(plan.output.ends_with("/debug/demo-piece"));
    }

    #[test]
    fn crate_name_validation_rules() {
        assert!(is_valid_crate_name("demo-piece"));
        assert!(!is_valid_crate_name(""));
        assert!(!is_valid_crate_name("demo piece"));
        assert!(!is_valid_crate_name("Demo"));
    }
}
