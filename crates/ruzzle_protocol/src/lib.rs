#![cfg_attr(not(test), no_std)]

extern crate alloc;

pub mod caps;
pub mod console;
pub mod registry;
pub mod shell;
pub mod tlv;

/// Errors returned by protocol encoders/decoders.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProtocolError {
    /// Underlying TLV parsing error.
    Tlv(tlv::TlvError),
    /// Field length does not match expected size.
    InvalidLength(&'static str),
    /// Required field is missing.
    MissingField(&'static str),
    /// Field appears more than once.
    DuplicateField(&'static str),
    /// UTF-8 validation failed.
    InvalidUtf8,
    /// Message type is unknown for this protocol.
    UnknownMessageType(u8),
    /// Field value is semantically invalid.
    InvalidValue(&'static str),
}

impl ProtocolError {
    /// Returns a stable, human-readable error label.
    pub fn as_str(&self) -> &'static str {
        match self {
            ProtocolError::Tlv(_) => "invalid tlv",
            ProtocolError::InvalidLength(_) => "invalid length",
            ProtocolError::MissingField(_) => "missing field",
            ProtocolError::DuplicateField(_) => "duplicate field",
            ProtocolError::InvalidUtf8 => "invalid utf8",
            ProtocolError::UnknownMessageType(_) => "unknown message type",
            ProtocolError::InvalidValue(_) => "invalid value",
        }
    }
}

impl From<tlv::TlvError> for ProtocolError {
    fn from(err: tlv::TlvError) -> Self {
        ProtocolError::Tlv(err)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn protocol_error_labels_are_stable() {
        assert_eq!(ProtocolError::InvalidUtf8.as_str(), "invalid utf8");
        assert_eq!(ProtocolError::InvalidLength("x").as_str(), "invalid length");
        assert_eq!(ProtocolError::MissingField("x").as_str(), "missing field");
        assert_eq!(ProtocolError::DuplicateField("x").as_str(), "duplicate field");
        assert_eq!(ProtocolError::UnknownMessageType(1).as_str(), "unknown message type");
        assert_eq!(ProtocolError::InvalidValue("x").as_str(), "invalid value");
        assert_eq!(ProtocolError::from(tlv::TlvError::TruncatedHeader).as_str(), "invalid tlv");
    }
}
