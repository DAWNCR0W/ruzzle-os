pub use ruzzle_protocol::console::{decode_log, encode_log, LogRecord, TLV_LEVEL, TLV_MESSAGE, TLV_PID};
pub use ruzzle_protocol::ProtocolError;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encode_decode_roundtrip() {
        let record = LogRecord {
            pid: 7,
            level: 1,
            message: "hello".to_string(),
        };
        let bytes = encode_log(&record);
        let decoded = decode_log(&bytes).expect("decode should succeed");
        assert_eq!(decoded, record);
    }

    #[test]
    fn decode_rejects_missing_fields() {
        let bytes = vec![];
        let result = decode_log(&bytes);
        assert_eq!(result, Err(ProtocolError::MissingField("pid")));
    }

    #[test]
    fn decode_rejects_invalid_pid_length() {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&TLV_PID.to_le_bytes());
        bytes.extend_from_slice(&2u16.to_le_bytes());
        bytes.extend_from_slice(&[0xAA, 0xBB]);
        let result = decode_log(&bytes);
        assert_eq!(result, Err(ProtocolError::InvalidLength("pid")));
    }

    #[test]
    fn decode_rejects_truncated_tlv() {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&TLV_MESSAGE.to_le_bytes());
        bytes.extend_from_slice(&5u16.to_le_bytes());
        bytes.extend_from_slice(b"hi");
        let result = decode_log(&bytes);
        assert_eq!(result, Err(ProtocolError::Tlv(ruzzle_protocol::tlv::TlvError::TruncatedValue)));
    }

    #[test]
    fn decode_rejects_invalid_level_length() {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&TLV_PID.to_le_bytes());
        bytes.extend_from_slice(&4u16.to_le_bytes());
        bytes.extend_from_slice(&7u32.to_le_bytes());
        bytes.extend_from_slice(&TLV_LEVEL.to_le_bytes());
        bytes.extend_from_slice(&2u16.to_le_bytes());
        bytes.extend_from_slice(&[0x01, 0x02]);
        bytes.extend_from_slice(&TLV_MESSAGE.to_le_bytes());
        bytes.extend_from_slice(&5u16.to_le_bytes());
        bytes.extend_from_slice(b"hello");
        let result = decode_log(&bytes);
        assert_eq!(result, Err(ProtocolError::InvalidLength("level")));
    }

    #[test]
    fn decode_ignores_unknown_tlvs() {
        let record = LogRecord {
            pid: 1,
            level: 2,
            message: "hi".to_string(),
        };
        let mut bytes = encode_log(&record);
        bytes.extend_from_slice(&0xFFFFu16.to_le_bytes());
        bytes.extend_from_slice(&1u16.to_le_bytes());
        bytes.push(0x00);
        let decoded = decode_log(&bytes).expect("decode should succeed");
        assert_eq!(decoded, record);
    }

    #[test]
    fn decode_rejects_invalid_message_utf8() {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&TLV_PID.to_le_bytes());
        bytes.extend_from_slice(&4u16.to_le_bytes());
        bytes.extend_from_slice(&1u32.to_le_bytes());
        bytes.extend_from_slice(&TLV_LEVEL.to_le_bytes());
        bytes.extend_from_slice(&1u16.to_le_bytes());
        bytes.push(1);
        bytes.extend_from_slice(&TLV_MESSAGE.to_le_bytes());
        bytes.extend_from_slice(&2u16.to_le_bytes());
        bytes.extend_from_slice(&[0xFF, 0xFF]);
        let result = decode_log(&bytes);
        assert_eq!(result, Err(ProtocolError::InvalidUtf8));
    }

    #[test]
    fn decode_rejects_missing_level() {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&TLV_PID.to_le_bytes());
        bytes.extend_from_slice(&4u16.to_le_bytes());
        bytes.extend_from_slice(&1u32.to_le_bytes());
        bytes.extend_from_slice(&TLV_MESSAGE.to_le_bytes());
        bytes.extend_from_slice(&2u16.to_le_bytes());
        bytes.extend_from_slice(b"ok");
        let result = decode_log(&bytes);
        assert_eq!(result, Err(ProtocolError::MissingField("level")));
    }

    #[test]
    fn decode_rejects_missing_message() {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&TLV_PID.to_le_bytes());
        bytes.extend_from_slice(&4u16.to_le_bytes());
        bytes.extend_from_slice(&1u32.to_le_bytes());
        bytes.extend_from_slice(&TLV_LEVEL.to_le_bytes());
        bytes.extend_from_slice(&1u16.to_le_bytes());
        bytes.push(1);
        let result = decode_log(&bytes);
        assert_eq!(result, Err(ProtocolError::MissingField("message")));
    }
}
