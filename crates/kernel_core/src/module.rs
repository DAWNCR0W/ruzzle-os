use alloc::string::String;
use alloc::vec::Vec;

use hal::Errno;

/// Manifest metadata describing a module ("puzzle piece").
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModuleManifest {
    pub name: String,
    pub version: String,
    pub provides: Vec<String>,
    pub slots: Vec<String>,
    pub requires_caps: Vec<String>,
    pub depends: Vec<String>,
}

/// Parses a minimal `module.toml` manifest.
pub fn parse_module_manifest(input: &str) -> Result<ModuleManifest, Errno> {
    let mut name: Option<String> = None;
    let mut version: Option<String> = None;
    let mut provides: Option<Vec<String>> = None;
    let mut slots: Option<Vec<String>> = None;
    let mut requires_caps: Option<Vec<String>> = None;
    let mut depends: Option<Vec<String>> = None;

    for line in input.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        let mut parts = trimmed.splitn(2, '=');
        let key = parts.next().map(str::trim).unwrap_or("");
        let value = parts.next().map(str::trim).ok_or(Errno::InvalidArg)?;
        match key {
            "name" => {
                ensure_unset(&name)?;
                name = Some(parse_string(value)?);
            }
            "version" => {
                ensure_unset(&version)?;
                version = Some(parse_string(value)?);
            }
            "provides" => {
                ensure_unset(&provides)?;
                provides = Some(parse_list(value)?);
            }
            "slots" => {
                ensure_unset(&slots)?;
                slots = Some(parse_list(value)?);
            }
            "requires_caps" => {
                ensure_unset(&requires_caps)?;
                requires_caps = Some(parse_list(value)?);
            }
            "depends" => {
                ensure_unset(&depends)?;
                depends = Some(parse_list(value)?);
            }
            _ => {
                return Err(Errno::InvalidArg);
            }
        }
    }

    let name = name.ok_or(Errno::InvalidArg)?;
    let version = version.ok_or(Errno::InvalidArg)?;
    Ok(ModuleManifest {
        name,
        version,
        provides: provides.unwrap_or_default(),
        slots: slots.unwrap_or_default(),
        requires_caps: requires_caps.unwrap_or_default(),
        depends: depends.unwrap_or_default(),
    })
}

fn ensure_unset<T>(field: &Option<T>) -> Result<(), Errno> {
    if field.is_some() {
        return Err(Errno::InvalidArg);
    }
    Ok(())
}

fn parse_string(value: &str) -> Result<String, Errno> {
    let trimmed = value.trim();
    if !trimmed.starts_with('"') || !trimmed.ends_with('"') || trimmed.len() < 2 {
        return Err(Errno::InvalidArg);
    }
    Ok(String::from(&trimmed[1..trimmed.len() - 1]))
}

fn parse_list(value: &str) -> Result<Vec<String>, Errno> {
    let trimmed = value.trim();
    if !trimmed.starts_with('[') || !trimmed.ends_with(']') {
        return Err(Errno::InvalidArg);
    }
    let inner = trimmed[1..trimmed.len() - 1].trim();
    if inner.is_empty() {
        return Ok(Vec::new());
    }
    let mut items = Vec::new();
    for raw in inner.split(',') {
        let item = raw.trim();
        if item.is_empty() {
            return Err(Errno::InvalidArg);
        }
        items.push(parse_string(item)?);
    }
    Ok(items)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_manifest_success() {
        let manifest = parse_module_manifest(
            r#"
            name = "console-service"
            version = "0.1.0"
            provides = ["ruzzle.console"]
            slots = ["ruzzle.slot.console"]
            requires_caps = ["ConsoleWrite", "EndpointCreate"]
            depends = []
            "#,
        )
        .expect("manifest should parse");

        assert_eq!(manifest.name, "console-service");
        assert_eq!(manifest.version, "0.1.0");
        assert_eq!(manifest.provides, vec!["ruzzle.console"]);
        assert_eq!(manifest.slots, vec!["ruzzle.slot.console"]);
        assert_eq!(
            manifest.requires_caps,
            vec!["ConsoleWrite", "EndpointCreate"]
        );
        assert!(manifest.depends.is_empty());
    }

    #[test]
    fn parse_manifest_accepts_empty_lists() {
        let manifest = parse_module_manifest(
            r#"
            name = "init"
            version = "0.1.0"
            provides = []
            slots = []
            requires_caps = []
            depends = []
            "#,
        )
        .expect("manifest should parse");

        assert!(manifest.provides.is_empty());
        assert!(manifest.slots.is_empty());
        assert!(manifest.requires_caps.is_empty());
        assert!(manifest.depends.is_empty());
    }

    #[test]
    fn parse_manifest_rejects_missing_fields() {
        let result = parse_module_manifest(r#"version = "0.1.0""#);
        assert_eq!(result, Err(Errno::InvalidArg));
    }

    #[test]
    fn parse_manifest_rejects_missing_version() {
        let result = parse_module_manifest(r#"name = "init""#);
        assert_eq!(result, Err(Errno::InvalidArg));
    }

    #[test]
    fn parse_manifest_rejects_duplicate_version() {
        let result = parse_module_manifest(
            r#"
            name = "init"
            version = "0.1.0"
            version = "0.2.0"
            "#,
        );
        assert_eq!(result, Err(Errno::InvalidArg));
    }

    #[test]
    fn parse_manifest_rejects_invalid_version_string() {
        let result = parse_module_manifest(
            r#"
            name = "init"
            version = 0.1.0
            "#,
        );
        assert_eq!(result, Err(Errno::InvalidArg));
    }

    #[test]
    fn parse_manifest_rejects_duplicate_keys() {
        let result = parse_module_manifest(
            r#"
            name = "init"
            name = "init2"
            version = "0.1.0"
            "#,
        );
        assert_eq!(result, Err(Errno::InvalidArg));
    }

    #[test]
    fn parse_manifest_rejects_duplicate_list_key() {
        let result = parse_module_manifest(
            r#"
            name = "init"
            version = "0.1.0"
            provides = ["a"]
            provides = ["b"]
            "#,
        );
        assert_eq!(result, Err(Errno::InvalidArg));
    }

    #[test]
    fn parse_manifest_rejects_duplicate_slots() {
        let result = parse_module_manifest(
            r#"
            name = "init"
            version = "0.1.0"
            slots = ["a"]
            slots = ["b"]
            "#,
        );
        assert_eq!(result, Err(Errno::InvalidArg));
    }

    #[test]
    fn parse_manifest_rejects_slots_invalid_list() {
        let result = parse_module_manifest(
            r#"
            name = "init"
            version = "0.1.0"
            slots = [bad]
            "#,
        );
        assert_eq!(result, Err(Errno::InvalidArg));
    }

    #[test]
    fn parse_manifest_rejects_duplicate_requires_caps() {
        let result = parse_module_manifest(
            r#"
            name = "init"
            version = "0.1.0"
            requires_caps = ["ConsoleWrite"]
            requires_caps = []
            "#,
        );
        assert_eq!(result, Err(Errno::InvalidArg));
    }

    #[test]
    fn parse_manifest_rejects_requires_caps_invalid_list() {
        let result = parse_module_manifest(
            r#"
            name = "init"
            version = "0.1.0"
            requires_caps = [bad]
            "#,
        );
        assert_eq!(result, Err(Errno::InvalidArg));
    }

    #[test]
    fn parse_manifest_rejects_duplicate_depends() {
        let result = parse_module_manifest(
            r#"
            name = "init"
            version = "0.1.0"
            depends = ["a"]
            depends = ["b"]
            "#,
        );
        assert_eq!(result, Err(Errno::InvalidArg));
    }

    #[test]
    fn parse_manifest_rejects_depends_invalid_list() {
        let result = parse_module_manifest(
            r#"
            name = "init"
            version = "0.1.0"
            depends = ["a"
            "#,
        );
        assert_eq!(result, Err(Errno::InvalidArg));
    }

    #[test]
    fn parse_manifest_rejects_invalid_list() {
        let result = parse_module_manifest(
            r#"
            name = "init"
            version = "0.1.0"
            provides = [bad]
            "#,
        );
        assert_eq!(result, Err(Errno::InvalidArg));
    }

    #[test]
    fn parse_manifest_rejects_invalid_list_brackets() {
        let result = parse_module_manifest(
            r#"
            name = "init"
            version = "0.1.0"
            provides = ["a"
            "#,
        );
        assert_eq!(result, Err(Errno::InvalidArg));
    }

    #[test]
    fn parse_manifest_rejects_empty_list_item() {
        let result = parse_module_manifest(
            r#"
            name = "init"
            version = "0.1.0"
            provides = ["a", ]
            "#,
        );
        assert_eq!(result, Err(Errno::InvalidArg));
    }

    #[test]
    fn parse_manifest_rejects_invalid_string() {
        let result = parse_module_manifest(
            r#"
            name = init
            version = "0.1.0"
            "#,
        );
        assert_eq!(result, Err(Errno::InvalidArg));
    }

    #[test]
    fn parse_manifest_rejects_missing_equals() {
        let result = parse_module_manifest(
            r#"
            name "init"
            version = "0.1.0"
            "#,
        );
        assert_eq!(result, Err(Errno::InvalidArg));
    }

    #[test]
    fn parse_manifest_rejects_unknown_key() {
        let result = parse_module_manifest(
            r#"
            name = "init"
            version = "0.1.0"
            extra = "nope"
            "#,
        );
        assert_eq!(result, Err(Errno::InvalidArg));
    }
}
