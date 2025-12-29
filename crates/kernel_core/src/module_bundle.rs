extern crate alloc;

use alloc::string::{String, ToString};
use alloc::vec::Vec;

use crate::module::{parse_module_manifest, ModuleManifest};
use crate::Errno;

const BUNDLE_MAGIC: &[u8; 4] = b"RMOD";
const BUNDLE_VERSION: u16 = 1;
const HEADER_LEN: usize = 4 + 2 + 4 + 4;

/// A parsed module bundle containing manifest metadata and ELF payload bytes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModuleBundle {
    pub manifest_text: String,
    pub manifest: ModuleManifest,
    pub payload: Vec<u8>,
}

/// Builds a module bundle from a manifest string and raw payload.
pub fn build_module_bundle(manifest_text: &str, payload: &[u8]) -> Result<Vec<u8>, Errno> {
    if manifest_text.is_empty() || payload.is_empty() {
        return Err(Errno::InvalidArg);
    }

    let _ = parse_module_manifest(manifest_text)?;

    let manifest_bytes = manifest_text.as_bytes();
    let manifest_len = manifest_bytes.len() as u32;
    let payload_len = payload.len() as u32;

    let mut out = Vec::with_capacity(HEADER_LEN + manifest_bytes.len() + payload.len());
    out.extend_from_slice(BUNDLE_MAGIC);
    out.extend_from_slice(&BUNDLE_VERSION.to_le_bytes());
    out.extend_from_slice(&manifest_len.to_le_bytes());
    out.extend_from_slice(&payload_len.to_le_bytes());
    out.extend_from_slice(manifest_bytes);
    out.extend_from_slice(payload);
    Ok(out)
}

/// Parses a module bundle from bytes.
pub fn parse_module_bundle(bytes: &[u8]) -> Result<ModuleBundle, Errno> {
    if bytes.len() < HEADER_LEN {
        return Err(Errno::InvalidArg);
    }

    if &bytes[..4] != BUNDLE_MAGIC {
        return Err(Errno::InvalidArg);
    }

    let version = u16::from_le_bytes([bytes[4], bytes[5]]);
    if version != BUNDLE_VERSION {
        return Err(Errno::InvalidArg);
    }

    let manifest_len = u32::from_le_bytes([bytes[6], bytes[7], bytes[8], bytes[9]]) as usize;
    let payload_len = u32::from_le_bytes([bytes[10], bytes[11], bytes[12], bytes[13]]) as usize;

    let available = bytes.len() - HEADER_LEN;
    if manifest_len > available {
        return Err(Errno::InvalidArg);
    }
    let remaining = available - manifest_len;
    if payload_len > remaining {
        return Err(Errno::InvalidArg);
    }

    let manifest_start = HEADER_LEN;
    let manifest_end = manifest_start + manifest_len;
    let payload_end = manifest_end + payload_len;

    let manifest_bytes = &bytes[manifest_start..manifest_end];
    let payload = bytes[manifest_end..payload_end].to_vec();

    let manifest_text = core::str::from_utf8(manifest_bytes)
        .map_err(|_| Errno::InvalidArg)?
        .to_string();
    let manifest = parse_module_manifest(&manifest_text)?;

    Ok(ModuleBundle {
        manifest_text,
        manifest,
        payload,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn example_manifest() -> &'static str {
        r#"
name = "fs-service"
version = "0.1.0"
provides = ["ruzzle.fs"]
slots = ["ruzzle.slot.fs"]
requires_caps = ["FsRoot"]
depends = []
"#
    }

    #[test]
    fn build_and_parse_roundtrip() {
        let payload = vec![1u8, 2, 3, 4];
        let bytes = build_module_bundle(example_manifest(), &payload).unwrap();
        let bundle = parse_module_bundle(&bytes).unwrap();
        assert_eq!(bundle.manifest.name, "fs-service");
        assert_eq!(bundle.manifest.provides, vec!["ruzzle.fs"]);
        assert_eq!(bundle.payload, payload);
    }

    #[test]
    fn parse_rejects_bad_magic() {
        let mut bytes = build_module_bundle(example_manifest(), &[1, 2]).unwrap();
        bytes[0] = 0;
        assert_eq!(parse_module_bundle(&bytes), Err(Errno::InvalidArg));
    }

    #[test]
    fn parse_rejects_bad_version() {
        let mut bytes = build_module_bundle(example_manifest(), &[1, 2]).unwrap();
        bytes[4] = 0xFF;
        bytes[5] = 0x00;
        assert_eq!(parse_module_bundle(&bytes), Err(Errno::InvalidArg));
    }

    #[test]
    fn parse_rejects_truncated_header() {
        let bytes = vec![0u8; HEADER_LEN - 1];
        assert_eq!(parse_module_bundle(&bytes), Err(Errno::InvalidArg));
    }

    #[test]
    fn parse_rejects_truncated_manifest() {
        let mut bytes = build_module_bundle(example_manifest(), &[1, 2]).unwrap();
        bytes.truncate(bytes.len() - 4);
        assert_eq!(parse_module_bundle(&bytes), Err(Errno::InvalidArg));
    }

    #[test]
    fn parse_rejects_truncated_payload() {
        let mut bytes = build_module_bundle(example_manifest(), &[1, 2, 3, 4]).unwrap();
        bytes.truncate(bytes.len() - 1);
        assert_eq!(parse_module_bundle(&bytes), Err(Errno::InvalidArg));
    }

    #[test]
    fn parse_rejects_invalid_manifest_utf8() {
        let mut bytes = build_module_bundle(example_manifest(), &[1, 2]).unwrap();
        let manifest_len = u32::from_le_bytes([bytes[6], bytes[7], bytes[8], bytes[9]]) as usize;
        let manifest_start = HEADER_LEN;
        bytes[manifest_start + manifest_len - 1] = 0xFF;
        assert_eq!(parse_module_bundle(&bytes), Err(Errno::InvalidArg));
    }

    #[test]
    fn parse_rejects_invalid_manifest_contents() {
        let manifest_text = "name = \"fs-service\"";
        let mut bytes = Vec::new();
        bytes.extend_from_slice(BUNDLE_MAGIC);
        bytes.extend_from_slice(&BUNDLE_VERSION.to_le_bytes());
        bytes.extend_from_slice(&(manifest_text.len() as u32).to_le_bytes());
        bytes.extend_from_slice(&2u32.to_le_bytes());
        bytes.extend_from_slice(manifest_text.as_bytes());
        bytes.extend_from_slice(&[1, 2]);
        assert_eq!(parse_module_bundle(&bytes), Err(Errno::InvalidArg));
    }

    #[test]
    fn build_rejects_empty_inputs() {
        assert_eq!(build_module_bundle("", &[1, 2]), Err(Errno::InvalidArg));
        assert_eq!(build_module_bundle(example_manifest(), &[]), Err(Errno::InvalidArg));
    }

    #[test]
    fn build_rejects_invalid_manifest() {
        let manifest_text = "name = \"fs-service\"";
        assert_eq!(build_module_bundle(manifest_text, &[1, 2]), Err(Errno::InvalidArg));
    }

    #[test]
    fn parse_rejects_length_overflow() {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(BUNDLE_MAGIC);
        bytes.extend_from_slice(&BUNDLE_VERSION.to_le_bytes());
        bytes.extend_from_slice(&u32::MAX.to_le_bytes());
        bytes.extend_from_slice(&u32::MAX.to_le_bytes());
        assert_eq!(parse_module_bundle(&bytes), Err(Errno::InvalidArg));
    }
}
