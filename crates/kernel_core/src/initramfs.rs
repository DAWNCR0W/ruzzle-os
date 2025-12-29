use alloc::string::{String, ToString};
use alloc::vec::Vec;

use hal::Errno;

const MAGIC: &[u8; 8] = b"RUZZLEFS";
const VERSION: u16 = 1;
const HEADER_SIZE: usize = 8 + 2 + 2;

/// An initramfs entry containing a name and payload bytes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InitramfsEntry {
    pub name: String,
    pub data: Vec<u8>,
}

/// Parses a Ruzzle initramfs image into entries.
pub fn parse_initramfs(bytes: &[u8]) -> Result<Vec<InitramfsEntry>, Errno> {
    if bytes.len() < HEADER_SIZE {
        return Err(Errno::InvalidArg);
    }
    if &bytes[..8] != MAGIC {
        return Err(Errno::InvalidArg);
    }
    let version = u16::from_le_bytes([bytes[8], bytes[9]]);
    if version != VERSION {
        return Err(Errno::InvalidArg);
    }
    let file_count = u16::from_le_bytes([bytes[10], bytes[11]]) as usize;

    let mut offset = HEADER_SIZE;
    let mut entries = Vec::with_capacity(file_count);
    for _ in 0..file_count {
        if offset + 2 + 8 > bytes.len() {
            return Err(Errno::InvalidArg);
        }
        let name_len = u16::from_le_bytes([bytes[offset], bytes[offset + 1]]) as usize;
        offset += 2;
        let data_len = u64::from_le_bytes([
            bytes[offset],
            bytes[offset + 1],
            bytes[offset + 2],
            bytes[offset + 3],
            bytes[offset + 4],
            bytes[offset + 5],
            bytes[offset + 6],
            bytes[offset + 7],
        ]) as usize;
        offset += 8;

        if offset + name_len + data_len > bytes.len() {
            return Err(Errno::InvalidArg);
        }
        let name_bytes = &bytes[offset..offset + name_len];
        let name = core::str::from_utf8(name_bytes).map_err(|_| Errno::InvalidArg)?;
        offset += name_len;

        let data = bytes[offset..offset + data_len].to_vec();
        offset += data_len;
        offset = align_up(offset, 8);

        entries.push(InitramfsEntry {
            name: name.to_string(),
            data,
        });
    }

    Ok(entries)
}

/// Serializes initramfs entries into a Ruzzle initramfs image.
pub fn build_initramfs(entries: &[InitramfsEntry]) -> Vec<u8> {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(MAGIC);
    bytes.extend_from_slice(&VERSION.to_le_bytes());
    bytes.extend_from_slice(&(entries.len() as u16).to_le_bytes());

    for entry in entries {
        let name_bytes = entry.name.as_bytes();
        bytes.extend_from_slice(&(name_bytes.len() as u16).to_le_bytes());
        bytes.extend_from_slice(&(entry.data.len() as u64).to_le_bytes());
        bytes.extend_from_slice(name_bytes);
        bytes.extend_from_slice(&entry.data);
        let padded_len = align_up(bytes.len(), 8);
        bytes.resize(padded_len, 0);
    }

    bytes
}

fn align_up(value: usize, align: usize) -> usize {
    if value % align == 0 {
        value
    } else {
        value + (align - (value % align))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn initramfs_roundtrip() {
        let entries = vec![
            InitramfsEntry {
                name: "init".to_string(),
                data: vec![1, 2, 3],
            },
            InitramfsEntry {
                name: "console".to_string(),
                data: vec![4, 5, 6, 7],
            },
        ];

        let image = build_initramfs(&entries);
        let parsed = parse_initramfs(&image).expect("parse should succeed");
        assert_eq!(parsed, entries);
    }

    #[test]
    fn initramfs_empty_parses() {
        let image = build_initramfs(&[]);
        let parsed = parse_initramfs(&image).expect("parse should succeed");
        assert!(parsed.is_empty());
    }

    #[test]
    fn initramfs_invalid_magic() {
        let mut image = build_initramfs(&[]);
        image[0] = 0x00;
        let result = parse_initramfs(&image);
        assert_eq!(result, Err(Errno::InvalidArg));
    }

    #[test]
    fn initramfs_invalid_version() {
        let mut image = build_initramfs(&[]);
        image[8] = 0x02;
        let result = parse_initramfs(&image);
        assert_eq!(result, Err(Errno::InvalidArg));
    }

    #[test]
    fn initramfs_truncated_header() {
        let result = parse_initramfs(&[0u8; 4]);
        assert_eq!(result, Err(Errno::InvalidArg));
    }

    #[test]
    fn initramfs_invalid_utf8_name() {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(MAGIC);
        bytes.extend_from_slice(&VERSION.to_le_bytes());
        bytes.extend_from_slice(&1u16.to_le_bytes());
        bytes.extend_from_slice(&1u16.to_le_bytes());
        bytes.extend_from_slice(&1u64.to_le_bytes());
        bytes.push(0xFF);
        bytes.push(0x00);

        let result = parse_initramfs(&bytes);
        assert_eq!(result, Err(Errno::InvalidArg));
    }

    #[test]
    fn initramfs_truncated_entry_header() {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(MAGIC);
        bytes.extend_from_slice(&VERSION.to_le_bytes());
        bytes.extend_from_slice(&1u16.to_le_bytes());
        let result = parse_initramfs(&bytes);
        assert_eq!(result, Err(Errno::InvalidArg));
    }

    #[test]
    fn initramfs_truncated_entry_data() {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(MAGIC);
        bytes.extend_from_slice(&VERSION.to_le_bytes());
        bytes.extend_from_slice(&1u16.to_le_bytes());
        bytes.extend_from_slice(&3u16.to_le_bytes());
        bytes.extend_from_slice(&5u64.to_le_bytes());
        bytes.extend_from_slice(b"ab");
        let result = parse_initramfs(&bytes);
        assert_eq!(result, Err(Errno::InvalidArg));
    }

    #[test]
    fn align_up_no_padding_needed() {
        assert_eq!(align_up(8, 8), 8);
    }

    #[test]
    fn align_up_with_padding() {
        assert_eq!(align_up(9, 8), 16);
    }
}
