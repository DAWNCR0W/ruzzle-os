extern crate alloc;

use alloc::string::{String, ToString};
use alloc::vec::Vec;

use crate::tlv::{write_tlv, TlvReader};
use crate::ProtocolError;

/// TLV type for message type.
pub const TLV_MSG_TYPE: u16 = 1;
/// TLV type for status code.
pub const TLV_STATUS: u16 = 2;
/// TLV type for service name.
pub const TLV_SERVICE: u16 = 3;
/// TLV type for module name.
pub const TLV_MODULE: u16 = 4;

/// Registry message: register service.
pub const MSG_REGISTER: u8 = 1;
/// Registry message: lookup service.
pub const MSG_LOOKUP: u8 = 2;
/// Registry message: list services.
pub const MSG_LIST: u8 = 3;
/// Registry response: ack.
pub const MSG_ACK: u8 = 100;
/// Registry response: lookup reply.
pub const MSG_LOOKUP_REPLY: u8 = 101;
/// Registry response: list reply.
pub const MSG_LIST_REPLY: u8 = 102;
/// Registry response: error.
pub const MSG_ERROR: u8 = 255;

/// Status codes for registry responses.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RegistryStatus {
    Ok,
    NotFound,
    Invalid,
    AlreadyExists,
}

impl RegistryStatus {
    pub fn as_u8(self) -> u8 {
        match self {
            RegistryStatus::Ok => 0,
            RegistryStatus::NotFound => 1,
            RegistryStatus::Invalid => 2,
            RegistryStatus::AlreadyExists => 3,
        }
    }

    pub fn from_u8(value: u8) -> Result<Self, ProtocolError> {
        match value {
            0 => Ok(RegistryStatus::Ok),
            1 => Ok(RegistryStatus::NotFound),
            2 => Ok(RegistryStatus::Invalid),
            3 => Ok(RegistryStatus::AlreadyExists),
            other => Err(ProtocolError::InvalidValue(match other {
                _ => "status",
            })),
        }
    }
}

/// Registry request messages.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RegistryRequest {
    Register { service: String, module: String },
    Lookup { service: String },
    List,
}

/// Registry response messages.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RegistryResponse {
    Ack,
    Lookup { status: RegistryStatus, module: Option<String> },
    List { status: RegistryStatus, entries: Vec<ServiceEntry> },
    Error { status: RegistryStatus },
}

/// Maps a service name to its owning module.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServiceEntry {
    pub service: String,
    pub module: String,
}

/// Encodes a registry request to TLV bytes.
pub fn encode_request(request: &RegistryRequest) -> Vec<u8> {
    let mut bytes = Vec::new();
    match request {
        RegistryRequest::Register { service, module } => {
            write_tlv(&mut bytes, TLV_MSG_TYPE, &[MSG_REGISTER]);
            write_tlv(&mut bytes, TLV_SERVICE, service.as_bytes());
            write_tlv(&mut bytes, TLV_MODULE, module.as_bytes());
        }
        RegistryRequest::Lookup { service } => {
            write_tlv(&mut bytes, TLV_MSG_TYPE, &[MSG_LOOKUP]);
            write_tlv(&mut bytes, TLV_SERVICE, service.as_bytes());
        }
        RegistryRequest::List => {
            write_tlv(&mut bytes, TLV_MSG_TYPE, &[MSG_LIST]);
        }
    }
    bytes
}

/// Decodes a registry request from TLV bytes.
pub fn decode_request(bytes: &[u8]) -> Result<RegistryRequest, ProtocolError> {
    let mut msg_type: Option<u8> = None;
    let mut service: Option<String> = None;
    let mut module: Option<String> = None;

    let mut reader = TlvReader::new(bytes);
    while let Some(field) = reader.next()? {
        match field.tlv_type {
            TLV_MSG_TYPE => {
                if msg_type.is_some() {
                    return Err(ProtocolError::DuplicateField("msg_type"));
                }
                if field.value.len() != 1 {
                    return Err(ProtocolError::InvalidLength("msg_type"));
                }
                msg_type = Some(field.value[0]);
            }
            TLV_SERVICE => {
                if service.is_some() {
                    return Err(ProtocolError::DuplicateField("service"));
                }
                service = Some(parse_string(field.value)?);
            }
            TLV_MODULE => {
                if module.is_some() {
                    return Err(ProtocolError::DuplicateField("module"));
                }
                module = Some(parse_string(field.value)?);
            }
            _ => {}
        }
    }

    let msg_type = msg_type.ok_or(ProtocolError::MissingField("msg_type"))?;
    match msg_type {
        MSG_REGISTER => Ok(RegistryRequest::Register {
            service: service.ok_or(ProtocolError::MissingField("service"))?,
            module: module.ok_or(ProtocolError::MissingField("module"))?,
        }),
        MSG_LOOKUP => Ok(RegistryRequest::Lookup {
            service: service.ok_or(ProtocolError::MissingField("service"))?,
        }),
        MSG_LIST => {
            if service.is_some() || module.is_some() {
                return Err(ProtocolError::InvalidValue("unexpected field"));
            }
            Ok(RegistryRequest::List)
        }
        other => Err(ProtocolError::UnknownMessageType(other)),
    }
}

/// Encodes a registry response to TLV bytes.
pub fn encode_response(response: &RegistryResponse) -> Vec<u8> {
    let mut bytes = Vec::new();
    match response {
        RegistryResponse::Ack => {
            write_tlv(&mut bytes, TLV_MSG_TYPE, &[MSG_ACK]);
            write_tlv(&mut bytes, TLV_STATUS, &[RegistryStatus::Ok.as_u8()]);
        }
        RegistryResponse::Lookup { status, module } => {
            write_tlv(&mut bytes, TLV_MSG_TYPE, &[MSG_LOOKUP_REPLY]);
            write_tlv(&mut bytes, TLV_STATUS, &[status.as_u8()]);
            if let Some(module) = module {
                write_tlv(&mut bytes, TLV_MODULE, module.as_bytes());
            }
        }
        RegistryResponse::List { status, entries } => {
            write_tlv(&mut bytes, TLV_MSG_TYPE, &[MSG_LIST_REPLY]);
            write_tlv(&mut bytes, TLV_STATUS, &[status.as_u8()]);
            if *status == RegistryStatus::Ok {
                for entry in entries {
                    write_tlv(&mut bytes, TLV_SERVICE, entry.service.as_bytes());
                    write_tlv(&mut bytes, TLV_MODULE, entry.module.as_bytes());
                }
            }
        }
        RegistryResponse::Error { status } => {
            write_tlv(&mut bytes, TLV_MSG_TYPE, &[MSG_ERROR]);
            write_tlv(&mut bytes, TLV_STATUS, &[status.as_u8()]);
        }
    }
    bytes
}

/// Decodes a registry response from TLV bytes.
pub fn decode_response(bytes: &[u8]) -> Result<RegistryResponse, ProtocolError> {
    let mut msg_type: Option<u8> = None;
    let mut status: Option<RegistryStatus> = None;
    let mut module: Option<String> = None;
    let mut entries: Vec<ServiceEntry> = Vec::new();
    let mut pending_service: Option<String> = None;

    let mut reader = TlvReader::new(bytes);
    while let Some(field) = reader.next()? {
        match field.tlv_type {
            TLV_MSG_TYPE => {
                if msg_type.is_some() {
                    return Err(ProtocolError::DuplicateField("msg_type"));
                }
                if field.value.len() != 1 {
                    return Err(ProtocolError::InvalidLength("msg_type"));
                }
                msg_type = Some(field.value[0]);
            }
            TLV_STATUS => {
                if status.is_some() {
                    return Err(ProtocolError::DuplicateField("status"));
                }
                if field.value.len() != 1 {
                    return Err(ProtocolError::InvalidLength("status"));
                }
                status = Some(RegistryStatus::from_u8(field.value[0])?);
            }
            TLV_MODULE => {
                let value = parse_string(field.value)?;
                if let Some(service) = pending_service.take() {
                    if entries.iter().any(|entry| entry.service == service) {
                        return Err(ProtocolError::DuplicateField("service"));
                    }
                    entries.push(ServiceEntry {
                        service,
                        module: value,
                    });
                } else {
                    if module.is_some() {
                        return Err(ProtocolError::DuplicateField("module"));
                    }
                    module = Some(value);
                }
            }
            TLV_SERVICE => {
                if pending_service.is_some() {
                    return Err(ProtocolError::MissingField("module"));
                }
                pending_service = Some(parse_string(field.value)?);
            }
            _ => {}
        }
    }

    if pending_service.is_some() {
        return Err(ProtocolError::MissingField("module"));
    }

    let msg_type = msg_type.ok_or(ProtocolError::MissingField("msg_type"))?;
    let status = status.ok_or(ProtocolError::MissingField("status"))?;

    match msg_type {
        MSG_ACK => {
            if status != RegistryStatus::Ok {
                return Err(ProtocolError::InvalidValue("status"));
            }
            Ok(RegistryResponse::Ack)
        }
        MSG_LOOKUP_REPLY => {
            if status == RegistryStatus::Ok && module.is_none() {
                return Err(ProtocolError::MissingField("module"));
            }
            if status != RegistryStatus::Ok && module.is_some() {
                return Err(ProtocolError::InvalidValue("module"));
            }
            Ok(RegistryResponse::Lookup { status, module })
        }
        MSG_LIST_REPLY => {
            if status != RegistryStatus::Ok && !entries.is_empty() {
                return Err(ProtocolError::InvalidValue("entries"));
            }
            Ok(RegistryResponse::List { status, entries })
        }
        MSG_ERROR => {
            if status == RegistryStatus::Ok {
                return Err(ProtocolError::InvalidValue("status"));
            }
            Ok(RegistryResponse::Error { status })
        }
        other => Err(ProtocolError::UnknownMessageType(other)),
    }
}

fn parse_string(value: &[u8]) -> Result<String, ProtocolError> {
    let text = core::str::from_utf8(value).map_err(|_| ProtocolError::InvalidUtf8)?;
    if text.is_empty() {
        return Err(ProtocolError::InvalidValue("string"));
    }
    Ok(text.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encode_decode_register_request() {
        let request = RegistryRequest::Register {
            service: "ruzzle.console".to_string(),
            module: "console-service".to_string(),
        };
        let bytes = encode_request(&request);
        let decoded = decode_request(&bytes).expect("decode should succeed");
        assert_eq!(decoded, request);
    }

    #[test]
    fn decode_request_rejects_missing_msg_type() {
        let result = decode_request(&[]);
        assert_eq!(result, Err(ProtocolError::MissingField("msg_type")));
    }

    #[test]
    fn decode_request_rejects_invalid_msg_type_length() {
        let mut bytes = Vec::new();
        write_tlv(&mut bytes, TLV_MSG_TYPE, &[0x01, 0x02]);
        let result = decode_request(&bytes);
        assert_eq!(result, Err(ProtocolError::InvalidLength("msg_type")));
    }

    #[test]
    fn decode_request_rejects_duplicate_service() {
        let mut bytes = Vec::new();
        write_tlv(&mut bytes, TLV_MSG_TYPE, &[MSG_LOOKUP]);
        write_tlv(&mut bytes, TLV_SERVICE, b"ruzzle.console");
        write_tlv(&mut bytes, TLV_SERVICE, b"ruzzle.console");
        let result = decode_request(&bytes);
        assert_eq!(result, Err(ProtocolError::DuplicateField("service")));
    }

    #[test]
    fn decode_request_rejects_unexpected_fields_for_list() {
        let mut bytes = Vec::new();
        write_tlv(&mut bytes, TLV_MSG_TYPE, &[MSG_LIST]);
        write_tlv(&mut bytes, TLV_SERVICE, b"ruzzle.console");
        let result = decode_request(&bytes);
        assert_eq!(result, Err(ProtocolError::InvalidValue("unexpected field")));
    }

    #[test]
    fn decode_request_rejects_unknown_type() {
        let mut bytes = Vec::new();
        write_tlv(&mut bytes, TLV_MSG_TYPE, &[0x42]);
        let result = decode_request(&bytes);
        assert_eq!(result, Err(ProtocolError::UnknownMessageType(0x42)));
    }

    #[test]
    fn encode_decode_lookup_response() {
        let response = RegistryResponse::Lookup {
            status: RegistryStatus::Ok,
            module: Some("console-service".to_string()),
        };
        let bytes = encode_response(&response);
        let decoded = decode_response(&bytes).expect("decode should succeed");
        assert_eq!(decoded, response);
    }

    #[test]
    fn encode_decode_ack_response() {
        let response = RegistryResponse::Ack;
        let bytes = encode_response(&response);
        let decoded = decode_response(&bytes).expect("decode should succeed");
        assert_eq!(decoded, response);
    }

    #[test]
    fn decode_response_rejects_missing_status() {
        let mut bytes = Vec::new();
        write_tlv(&mut bytes, TLV_MSG_TYPE, &[MSG_ACK]);
        let result = decode_response(&bytes);
        assert_eq!(result, Err(ProtocolError::MissingField("status")));
    }

    #[test]
    fn decode_response_rejects_missing_msg_type() {
        let mut bytes = Vec::new();
        write_tlv(&mut bytes, TLV_STATUS, &[RegistryStatus::Ok.as_u8()]);
        let result = decode_response(&bytes);
        assert_eq!(result, Err(ProtocolError::MissingField("msg_type")));
    }

    #[test]
    fn decode_response_rejects_invalid_status_length() {
        let mut bytes = Vec::new();
        write_tlv(&mut bytes, TLV_MSG_TYPE, &[MSG_ACK]);
        write_tlv(&mut bytes, TLV_STATUS, &[0x01, 0x02]);
        let result = decode_response(&bytes);
        assert_eq!(result, Err(ProtocolError::InvalidLength("status")));
    }

    #[test]
    fn decode_response_rejects_ack_with_error_status() {
        let mut bytes = Vec::new();
        write_tlv(&mut bytes, TLV_MSG_TYPE, &[MSG_ACK]);
        write_tlv(&mut bytes, TLV_STATUS, &[RegistryStatus::Invalid.as_u8()]);
        let result = decode_response(&bytes);
        assert_eq!(result, Err(ProtocolError::InvalidValue("status")));
    }

    #[test]
    fn decode_response_rejects_lookup_missing_module() {
        let mut bytes = Vec::new();
        write_tlv(&mut bytes, TLV_MSG_TYPE, &[MSG_LOOKUP_REPLY]);
        write_tlv(&mut bytes, TLV_STATUS, &[RegistryStatus::Ok.as_u8()]);
        let result = decode_response(&bytes);
        assert_eq!(result, Err(ProtocolError::MissingField("module")));
    }

    #[test]
    fn decode_response_rejects_lookup_with_module_on_error() {
        let mut bytes = Vec::new();
        write_tlv(&mut bytes, TLV_MSG_TYPE, &[MSG_LOOKUP_REPLY]);
        write_tlv(&mut bytes, TLV_STATUS, &[RegistryStatus::NotFound.as_u8()]);
        write_tlv(&mut bytes, TLV_MODULE, b"console-service");
        let result = decode_response(&bytes);
        assert_eq!(result, Err(ProtocolError::InvalidValue("module")));
    }

    #[test]
    fn decode_response_rejects_list_with_entries_on_error() {
        let mut bytes = Vec::new();
        write_tlv(&mut bytes, TLV_MSG_TYPE, &[MSG_LIST_REPLY]);
        write_tlv(&mut bytes, TLV_STATUS, &[RegistryStatus::Invalid.as_u8()]);
        write_tlv(&mut bytes, TLV_SERVICE, b"ruzzle.console");
        write_tlv(&mut bytes, TLV_MODULE, b"console-service");
        let result = decode_response(&bytes);
        assert_eq!(result, Err(ProtocolError::InvalidValue("entries")));
    }

    #[test]
    fn decode_response_rejects_service_without_module() {
        let mut bytes = Vec::new();
        write_tlv(&mut bytes, TLV_MSG_TYPE, &[MSG_LIST_REPLY]);
        write_tlv(&mut bytes, TLV_STATUS, &[RegistryStatus::Ok.as_u8()]);
        write_tlv(&mut bytes, TLV_SERVICE, b"ruzzle.console");
        let result = decode_response(&bytes);
        assert_eq!(result, Err(ProtocolError::MissingField("module")));
    }

    #[test]
    fn decode_response_rejects_duplicate_service_entries() {
        let mut bytes = Vec::new();
        write_tlv(&mut bytes, TLV_MSG_TYPE, &[MSG_LIST_REPLY]);
        write_tlv(&mut bytes, TLV_STATUS, &[RegistryStatus::Ok.as_u8()]);
        write_tlv(&mut bytes, TLV_SERVICE, b"ruzzle.console");
        write_tlv(&mut bytes, TLV_MODULE, b"console-service");
        write_tlv(&mut bytes, TLV_SERVICE, b"ruzzle.console");
        write_tlv(&mut bytes, TLV_MODULE, b"console-service");
        let result = decode_response(&bytes);
        assert_eq!(result, Err(ProtocolError::DuplicateField("service")));
    }

    #[test]
    fn decode_response_handles_error_message() {
        let response = RegistryResponse::Error {
            status: RegistryStatus::Invalid,
        };
        let bytes = encode_response(&response);
        let decoded = decode_response(&bytes).expect("decode should succeed");
        assert_eq!(decoded, response);
    }

    #[test]
    fn decode_response_ignores_unknown_tlv() {
        let mut bytes = Vec::new();
        write_tlv(&mut bytes, TLV_MSG_TYPE, &[MSG_ACK]);
        write_tlv(&mut bytes, TLV_STATUS, &[RegistryStatus::Ok.as_u8()]);
        write_tlv(&mut bytes, 0x9999, b"ignored");
        let decoded = decode_response(&bytes).expect("decode should succeed");
        assert_eq!(decoded, RegistryResponse::Ack);
    }

    #[test]
    fn registry_status_roundtrip_values() {
        assert_eq!(RegistryStatus::AlreadyExists.as_u8(), 3);
        assert_eq!(RegistryStatus::from_u8(3).unwrap(), RegistryStatus::AlreadyExists);
        assert_eq!(
            RegistryStatus::from_u8(9),
            Err(ProtocolError::InvalidValue("status"))
        );
    }

    #[test]
    fn encode_decode_lookup_and_list_requests() {
        let lookup = RegistryRequest::Lookup {
            service: "ruzzle.console".to_string(),
        };
        let lookup_bytes = encode_request(&lookup);
        let lookup_decoded = decode_request(&lookup_bytes).expect("decode should succeed");
        assert_eq!(lookup_decoded, lookup);

        let list = RegistryRequest::List;
        let list_bytes = encode_request(&list);
        let list_decoded = decode_request(&list_bytes).expect("decode should succeed");
        assert_eq!(list_decoded, list);
    }

    #[test]
    fn decode_request_rejects_duplicate_msg_type() {
        let mut bytes = Vec::new();
        write_tlv(&mut bytes, TLV_MSG_TYPE, &[MSG_LOOKUP]);
        write_tlv(&mut bytes, TLV_MSG_TYPE, &[MSG_LOOKUP]);
        let result = decode_request(&bytes);
        assert_eq!(result, Err(ProtocolError::DuplicateField("msg_type")));
    }

    #[test]
    fn decode_request_rejects_duplicate_module() {
        let mut bytes = Vec::new();
        write_tlv(&mut bytes, TLV_MSG_TYPE, &[MSG_REGISTER]);
        write_tlv(&mut bytes, TLV_SERVICE, b"ruzzle.console");
        write_tlv(&mut bytes, TLV_MODULE, b"console-service");
        write_tlv(&mut bytes, TLV_MODULE, b"console-service");
        let result = decode_request(&bytes);
        assert_eq!(result, Err(ProtocolError::DuplicateField("module")));
    }

    #[test]
    fn decode_request_rejects_invalid_service_utf8() {
        let mut bytes = Vec::new();
        write_tlv(&mut bytes, TLV_MSG_TYPE, &[MSG_LOOKUP]);
        write_tlv(&mut bytes, TLV_SERVICE, &[0xFF]);
        let result = decode_request(&bytes);
        assert_eq!(result, Err(ProtocolError::InvalidUtf8));
    }

    #[test]
    fn decode_request_rejects_invalid_module_utf8() {
        let mut bytes = Vec::new();
        write_tlv(&mut bytes, TLV_MSG_TYPE, &[MSG_REGISTER]);
        write_tlv(&mut bytes, TLV_SERVICE, b"ruzzle.console");
        write_tlv(&mut bytes, TLV_MODULE, &[0xFF]);
        let result = decode_request(&bytes);
        assert_eq!(result, Err(ProtocolError::InvalidUtf8));
    }

    #[test]
    fn decode_request_rejects_empty_service() {
        let mut bytes = Vec::new();
        write_tlv(&mut bytes, TLV_MSG_TYPE, &[MSG_LOOKUP]);
        write_tlv(&mut bytes, TLV_SERVICE, &[]);
        let result = decode_request(&bytes);
        assert_eq!(result, Err(ProtocolError::InvalidValue("string")));
    }

    #[test]
    fn decode_request_rejects_missing_fields_for_register() {
        let mut bytes = Vec::new();
        write_tlv(&mut bytes, TLV_MSG_TYPE, &[MSG_REGISTER]);
        write_tlv(&mut bytes, TLV_MODULE, b"console-service");
        let result = decode_request(&bytes);
        assert_eq!(result, Err(ProtocolError::MissingField("service")));
    }

    #[test]
    fn decode_request_rejects_missing_module_for_register() {
        let mut bytes = Vec::new();
        write_tlv(&mut bytes, TLV_MSG_TYPE, &[MSG_REGISTER]);
        write_tlv(&mut bytes, TLV_SERVICE, b"ruzzle.console");
        let result = decode_request(&bytes);
        assert_eq!(result, Err(ProtocolError::MissingField("module")));
    }

    #[test]
    fn decode_request_rejects_missing_service_for_lookup() {
        let mut bytes = Vec::new();
        write_tlv(&mut bytes, TLV_MSG_TYPE, &[MSG_LOOKUP]);
        let result = decode_request(&bytes);
        assert_eq!(result, Err(ProtocolError::MissingField("service")));
    }

    #[test]
    fn decode_request_ignores_unknown_tlv() {
        let mut bytes = Vec::new();
        write_tlv(&mut bytes, TLV_MSG_TYPE, &[MSG_REGISTER]);
        write_tlv(&mut bytes, 0x9999, b"ignored");
        write_tlv(&mut bytes, TLV_SERVICE, b"ruzzle.console");
        write_tlv(&mut bytes, TLV_MODULE, b"console-service");
        let result = decode_request(&bytes).expect("decode should succeed");
        assert_eq!(
            result,
            RegistryRequest::Register {
                service: "ruzzle.console".to_string(),
                module: "console-service".to_string(),
            }
        );
    }

    #[test]
    fn decode_request_rejects_truncated_tlv() {
        let bytes = [TLV_MSG_TYPE as u8, 0x00, 0x01];
        let result = decode_request(&bytes);
        assert_eq!(
            result,
            Err(ProtocolError::Tlv(crate::tlv::TlvError::TruncatedHeader))
        );
    }

    #[test]
    fn encode_decode_list_response() {
        let response = RegistryResponse::List {
            status: RegistryStatus::Ok,
            entries: vec![ServiceEntry {
                service: "ruzzle.console".to_string(),
                module: "console-service".to_string(),
            }],
        };
        let bytes = encode_response(&response);
        let decoded = decode_response(&bytes).expect("decode should succeed");
        assert_eq!(decoded, response);
    }

    #[test]
    fn encode_decode_list_response_with_error_status() {
        let response = RegistryResponse::List {
            status: RegistryStatus::Invalid,
            entries: vec![],
        };
        let bytes = encode_response(&response);
        let decoded = decode_response(&bytes).expect("decode should succeed");
        assert_eq!(decoded, response);
    }

    #[test]
    fn encode_decode_lookup_response_without_module() {
        let response = RegistryResponse::Lookup {
            status: RegistryStatus::NotFound,
            module: None,
        };
        let bytes = encode_response(&response);
        let decoded = decode_response(&bytes).expect("decode should succeed");
        assert_eq!(decoded, response);
    }

    #[test]
    fn decode_response_rejects_duplicate_msg_type() {
        let mut bytes = Vec::new();
        write_tlv(&mut bytes, TLV_MSG_TYPE, &[MSG_ACK]);
        write_tlv(&mut bytes, TLV_MSG_TYPE, &[MSG_ACK]);
        write_tlv(&mut bytes, TLV_STATUS, &[RegistryStatus::Ok.as_u8()]);
        let result = decode_response(&bytes);
        assert_eq!(result, Err(ProtocolError::DuplicateField("msg_type")));
    }

    #[test]
    fn decode_response_rejects_duplicate_status() {
        let mut bytes = Vec::new();
        write_tlv(&mut bytes, TLV_MSG_TYPE, &[MSG_ACK]);
        write_tlv(&mut bytes, TLV_STATUS, &[RegistryStatus::Ok.as_u8()]);
        write_tlv(&mut bytes, TLV_STATUS, &[RegistryStatus::Ok.as_u8()]);
        let result = decode_response(&bytes);
        assert_eq!(result, Err(ProtocolError::DuplicateField("status")));
    }

    #[test]
    fn decode_response_rejects_invalid_msg_type_length() {
        let mut bytes = Vec::new();
        write_tlv(&mut bytes, TLV_MSG_TYPE, &[MSG_ACK, 0x00]);
        write_tlv(&mut bytes, TLV_STATUS, &[RegistryStatus::Ok.as_u8()]);
        let result = decode_response(&bytes);
        assert_eq!(result, Err(ProtocolError::InvalidLength("msg_type")));
    }

    #[test]
    fn decode_response_rejects_invalid_status_value() {
        let mut bytes = Vec::new();
        write_tlv(&mut bytes, TLV_MSG_TYPE, &[MSG_ACK]);
        write_tlv(&mut bytes, TLV_STATUS, &[9]);
        let result = decode_response(&bytes);
        assert_eq!(result, Err(ProtocolError::InvalidValue("status")));
    }

    #[test]
    fn decode_response_rejects_error_with_ok_status() {
        let mut bytes = Vec::new();
        write_tlv(&mut bytes, TLV_MSG_TYPE, &[MSG_ERROR]);
        write_tlv(&mut bytes, TLV_STATUS, &[RegistryStatus::Ok.as_u8()]);
        let result = decode_response(&bytes);
        assert_eq!(result, Err(ProtocolError::InvalidValue("status")));
    }

    #[test]
    fn decode_response_rejects_duplicate_module() {
        let mut bytes = Vec::new();
        write_tlv(&mut bytes, TLV_MSG_TYPE, &[MSG_LOOKUP_REPLY]);
        write_tlv(&mut bytes, TLV_STATUS, &[RegistryStatus::Ok.as_u8()]);
        write_tlv(&mut bytes, TLV_MODULE, b"console-service");
        write_tlv(&mut bytes, TLV_MODULE, b"console-service");
        let result = decode_response(&bytes);
        assert_eq!(result, Err(ProtocolError::DuplicateField("module")));
    }

    #[test]
    fn decode_response_rejects_double_service_without_module() {
        let mut bytes = Vec::new();
        write_tlv(&mut bytes, TLV_MSG_TYPE, &[MSG_LIST_REPLY]);
        write_tlv(&mut bytes, TLV_STATUS, &[RegistryStatus::Ok.as_u8()]);
        write_tlv(&mut bytes, TLV_SERVICE, b"ruzzle.console");
        write_tlv(&mut bytes, TLV_SERVICE, b"ruzzle.shell");
        let result = decode_response(&bytes);
        assert_eq!(result, Err(ProtocolError::MissingField("module")));
    }

    #[test]
    fn decode_response_rejects_invalid_utf8_service() {
        let mut bytes = Vec::new();
        write_tlv(&mut bytes, TLV_MSG_TYPE, &[MSG_LIST_REPLY]);
        write_tlv(&mut bytes, TLV_STATUS, &[RegistryStatus::Ok.as_u8()]);
        write_tlv(&mut bytes, TLV_SERVICE, &[0xFF]);
        let result = decode_response(&bytes);
        assert_eq!(result, Err(ProtocolError::InvalidUtf8));
    }

    #[test]
    fn decode_response_rejects_invalid_utf8_module() {
        let mut bytes = Vec::new();
        write_tlv(&mut bytes, TLV_MSG_TYPE, &[MSG_LOOKUP_REPLY]);
        write_tlv(&mut bytes, TLV_STATUS, &[RegistryStatus::Ok.as_u8()]);
        write_tlv(&mut bytes, TLV_MODULE, &[0xFF]);
        let result = decode_response(&bytes);
        assert_eq!(result, Err(ProtocolError::InvalidUtf8));
    }

    #[test]
    fn decode_response_rejects_empty_module() {
        let mut bytes = Vec::new();
        write_tlv(&mut bytes, TLV_MSG_TYPE, &[MSG_LOOKUP_REPLY]);
        write_tlv(&mut bytes, TLV_STATUS, &[RegistryStatus::Ok.as_u8()]);
        write_tlv(&mut bytes, TLV_MODULE, &[]);
        let result = decode_response(&bytes);
        assert_eq!(result, Err(ProtocolError::InvalidValue("string")));
    }

    #[test]
    fn decode_response_rejects_truncated_tlv() {
        let bytes = [TLV_MSG_TYPE as u8, 0x00, 0x01];
        let result = decode_response(&bytes);
        assert_eq!(
            result,
            Err(ProtocolError::Tlv(crate::tlv::TlvError::TruncatedHeader))
        );
    }

    #[test]
    fn decode_response_rejects_unknown_message_type() {
        let mut bytes = Vec::new();
        write_tlv(&mut bytes, TLV_MSG_TYPE, &[0x77]);
        write_tlv(&mut bytes, TLV_STATUS, &[RegistryStatus::Ok.as_u8()]);
        let result = decode_response(&bytes);
        assert_eq!(result, Err(ProtocolError::UnknownMessageType(0x77)));
    }
}
