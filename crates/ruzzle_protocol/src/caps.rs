extern crate alloc;

use alloc::string::{String, ToString};
use alloc::vec::Vec;

use crate::tlv::{write_tlv, TlvReader};
use crate::ProtocolError;

/// TLV type for a capability name.
pub const TLV_CAP_NAME: u16 = 50;

/// Encodes a list of capability names into TLVs.
pub fn encode_caps(caps: &[String]) -> Vec<u8> {
    let mut bytes = Vec::new();
    for cap in caps {
        write_tlv(&mut bytes, TLV_CAP_NAME, cap.as_bytes());
    }
    bytes
}

/// Decodes a list of capability names from TLVs.
pub fn decode_caps(bytes: &[u8]) -> Result<Vec<String>, ProtocolError> {
    let mut caps = Vec::new();
    let mut reader = TlvReader::new(bytes);
    while let Some(field) = reader.next()? {
        if field.tlv_type != TLV_CAP_NAME {
            continue;
        }
        let text = core::str::from_utf8(field.value)
            .map_err(|_| ProtocolError::InvalidUtf8)?;
        if text.is_empty() {
            return Err(ProtocolError::InvalidValue("cap"));
        }
        if caps.iter().any(|cap| cap == text) {
            return Err(ProtocolError::DuplicateField("cap"));
        }
        caps.push(text.to_string());
    }
    Ok(caps)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encode_decode_caps_roundtrip() {
        let caps = vec!["ConsoleWrite".to_string(), "EndpointCreate".to_string()];
        let bytes = encode_caps(&caps);
        let decoded = decode_caps(&bytes).expect("decode should succeed");
        assert_eq!(decoded, caps);
    }

    #[test]
    fn decode_caps_accepts_empty() {
        let decoded = decode_caps(&[]).expect("empty should decode");
        assert!(decoded.is_empty());
    }

    #[test]
    fn decode_caps_rejects_invalid_utf8() {
        let bytes = [
            TLV_CAP_NAME as u8,
            0x00,
            0x01,
            0x00,
            0xFF,
        ];
        let result = decode_caps(&bytes);
        assert_eq!(result, Err(ProtocolError::InvalidUtf8));
    }

    #[test]
    fn decode_caps_rejects_empty_name() {
        let bytes = [TLV_CAP_NAME as u8, 0x00, 0x00, 0x00];
        let result = decode_caps(&bytes);
        assert_eq!(result, Err(ProtocolError::InvalidValue("cap")));
    }

    #[test]
    fn decode_caps_rejects_duplicates() {
        let mut bytes = Vec::new();
        write_tlv(&mut bytes, TLV_CAP_NAME, b"ConsoleWrite");
        write_tlv(&mut bytes, TLV_CAP_NAME, b"ConsoleWrite");
        let result = decode_caps(&bytes);
        assert_eq!(result, Err(ProtocolError::DuplicateField("cap")));
    }

    #[test]
    fn decode_caps_ignores_unknown_tlv() {
        let mut bytes = Vec::new();
        write_tlv(&mut bytes, 0x9999, b"ignored");
        write_tlv(&mut bytes, TLV_CAP_NAME, b"ConsoleWrite");
        let result = decode_caps(&bytes).expect("decode should succeed");
        assert_eq!(result, vec!["ConsoleWrite".to_string()]);
    }

    #[test]
    fn decode_caps_rejects_truncated_tlv() {
        let bytes = [TLV_CAP_NAME as u8, 0x00, 0x02];
        let result = decode_caps(&bytes);
        assert_eq!(
            result,
            Err(ProtocolError::Tlv(crate::tlv::TlvError::TruncatedHeader))
        );
    }
}
