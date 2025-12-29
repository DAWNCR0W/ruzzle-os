extern crate alloc;

use alloc::string::{String, ToString};
use alloc::vec::Vec;

use crate::tlv::{write_tlv, TlvReader};
use crate::ProtocolError;

/// TLV type for log level.
pub const TLV_LEVEL: u16 = 1;
/// TLV type for process ID.
pub const TLV_PID: u16 = 2;
/// TLV type for log message.
pub const TLV_MESSAGE: u16 = 3;

/// Represents a decoded log record.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LogRecord {
    pub pid: u32,
    pub level: u8,
    pub message: String,
}

/// Encodes a log record into TLV bytes.
pub fn encode_log(record: &LogRecord) -> Vec<u8> {
    let mut bytes = Vec::new();
    write_tlv(&mut bytes, TLV_PID, &record.pid.to_le_bytes());
    write_tlv(&mut bytes, TLV_LEVEL, &[record.level]);
    write_tlv(&mut bytes, TLV_MESSAGE, record.message.as_bytes());
    bytes
}

/// Decodes a log record from TLV bytes.
pub fn decode_log(bytes: &[u8]) -> Result<LogRecord, ProtocolError> {
    let mut pid: Option<u32> = None;
    let mut level: Option<u8> = None;
    let mut message: Option<String> = None;

    let mut reader = TlvReader::new(bytes);
    while let Some(field) = reader.next()? {
        match field.tlv_type {
            TLV_PID => {
                if pid.is_some() {
                    return Err(ProtocolError::DuplicateField("pid"));
                }
                if field.value.len() != 4 {
                    return Err(ProtocolError::InvalidLength("pid"));
                }
                pid = Some(u32::from_le_bytes([
                    field.value[0],
                    field.value[1],
                    field.value[2],
                    field.value[3],
                ]));
            }
            TLV_LEVEL => {
                if level.is_some() {
                    return Err(ProtocolError::DuplicateField("level"));
                }
                if field.value.len() != 1 {
                    return Err(ProtocolError::InvalidLength("level"));
                }
                level = Some(field.value[0]);
            }
            TLV_MESSAGE => {
                if message.is_some() {
                    return Err(ProtocolError::DuplicateField("message"));
                }
                let text = core::str::from_utf8(field.value)
                    .map_err(|_| ProtocolError::InvalidUtf8)?;
                message = Some(text.to_string());
            }
            _ => {}
        }
    }

    Ok(LogRecord {
        pid: pid.ok_or(ProtocolError::MissingField("pid"))?,
        level: level.ok_or(ProtocolError::MissingField("level"))?,
        message: message.ok_or(ProtocolError::MissingField("message"))?,
    })
}

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
        write_tlv(&mut bytes, TLV_PID, &[0xAA, 0xBB]);
        let result = decode_log(&bytes);
        assert_eq!(result, Err(ProtocolError::InvalidLength("pid")));
    }

    #[test]
    fn decode_rejects_truncated_tlv() {
        let bytes = [TLV_MESSAGE as u8, 0x00, 0x05, 0x00, 0xAA];
        let result = decode_log(&bytes);
        assert_eq!(result, Err(ProtocolError::Tlv(crate::tlv::TlvError::TruncatedValue)));
    }

    #[test]
    fn decode_rejects_invalid_level_length() {
        let mut bytes = Vec::new();
        write_tlv(&mut bytes, TLV_PID, &7u32.to_le_bytes());
        write_tlv(&mut bytes, TLV_LEVEL, &[0x01, 0x02]);
        write_tlv(&mut bytes, TLV_MESSAGE, b"hello");
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
        write_tlv(&mut bytes, 0xFFFF, &[0x00]);
        let decoded = decode_log(&bytes).expect("decode should succeed");
        assert_eq!(decoded, record);
    }

    #[test]
    fn decode_rejects_invalid_message_utf8() {
        let mut bytes = Vec::new();
        write_tlv(&mut bytes, TLV_PID, &1u32.to_le_bytes());
        write_tlv(&mut bytes, TLV_LEVEL, &[1]);
        write_tlv(&mut bytes, TLV_MESSAGE, &[0xFF, 0xFF]);
        let result = decode_log(&bytes);
        assert_eq!(result, Err(ProtocolError::InvalidUtf8));
    }

    #[test]
    fn decode_rejects_duplicate_pid() {
        let mut bytes = Vec::new();
        write_tlv(&mut bytes, TLV_PID, &1u32.to_le_bytes());
        write_tlv(&mut bytes, TLV_PID, &2u32.to_le_bytes());
        let result = decode_log(&bytes);
        assert_eq!(result, Err(ProtocolError::DuplicateField("pid")));
    }

    #[test]
    fn decode_rejects_duplicate_level() {
        let mut bytes = Vec::new();
        write_tlv(&mut bytes, TLV_PID, &1u32.to_le_bytes());
        write_tlv(&mut bytes, TLV_LEVEL, &[1]);
        write_tlv(&mut bytes, TLV_LEVEL, &[2]);
        let result = decode_log(&bytes);
        assert_eq!(result, Err(ProtocolError::DuplicateField("level")));
    }

    #[test]
    fn decode_rejects_duplicate_message() {
        let mut bytes = Vec::new();
        write_tlv(&mut bytes, TLV_PID, &1u32.to_le_bytes());
        write_tlv(&mut bytes, TLV_LEVEL, &[1]);
        write_tlv(&mut bytes, TLV_MESSAGE, b"first");
        write_tlv(&mut bytes, TLV_MESSAGE, b"second");
        let result = decode_log(&bytes);
        assert_eq!(result, Err(ProtocolError::DuplicateField("message")));
    }

    #[test]
    fn decode_rejects_missing_level() {
        let mut bytes = Vec::new();
        write_tlv(&mut bytes, TLV_PID, &1u32.to_le_bytes());
        write_tlv(&mut bytes, TLV_MESSAGE, b"ok");
        let result = decode_log(&bytes);
        assert_eq!(result, Err(ProtocolError::MissingField("level")));
    }

    #[test]
    fn decode_rejects_missing_message() {
        let mut bytes = Vec::new();
        write_tlv(&mut bytes, TLV_PID, &1u32.to_le_bytes());
        write_tlv(&mut bytes, TLV_LEVEL, &[1]);
        let result = decode_log(&bytes);
        assert_eq!(result, Err(ProtocolError::MissingField("message")));
    }
}
