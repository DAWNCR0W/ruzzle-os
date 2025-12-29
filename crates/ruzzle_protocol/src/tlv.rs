extern crate alloc;

use alloc::vec::Vec;

/// A decoded TLV field.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TlvField<'a> {
    pub tlv_type: u16,
    pub value: &'a [u8],
}

/// Errors produced while decoding TLVs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TlvError {
    TruncatedHeader,
    TruncatedValue,
}

/// Reads TLV fields from a byte slice.
pub struct TlvReader<'a> {
    bytes: &'a [u8],
    offset: usize,
}

impl<'a> TlvReader<'a> {
    /// Creates a new reader starting at offset zero.
    pub fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, offset: 0 }
    }

    /// Returns the next TLV field, or None if fully consumed.
    pub fn next(&mut self) -> Result<Option<TlvField<'a>>, TlvError> {
        if self.offset == self.bytes.len() {
            return Ok(None);
        }
        if self.bytes.len() - self.offset < 4 {
            return Err(TlvError::TruncatedHeader);
        }
        let tlv_type = u16::from_le_bytes([
            self.bytes[self.offset],
            self.bytes[self.offset + 1],
        ]);
        let len = u16::from_le_bytes([
            self.bytes[self.offset + 2],
            self.bytes[self.offset + 3],
        ]) as usize;
        self.offset += 4;
        if self.offset + len > self.bytes.len() {
            return Err(TlvError::TruncatedValue);
        }
        let value = &self.bytes[self.offset..self.offset + len];
        self.offset += len;
        Ok(Some(TlvField { tlv_type, value }))
    }
}

/// Appends a TLV field to the output buffer.
pub fn write_tlv(buf: &mut Vec<u8>, tlv_type: u16, value: &[u8]) {
    buf.extend_from_slice(&tlv_type.to_le_bytes());
    buf.extend_from_slice(&(value.len() as u16).to_le_bytes());
    buf.extend_from_slice(value);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reader_returns_none_for_empty() {
        let mut reader = TlvReader::new(&[]);
        assert_eq!(reader.next(), Ok(None));
    }

    #[test]
    fn reader_rejects_truncated_header() {
        let mut reader = TlvReader::new(&[0x01, 0x00, 0x02]);
        assert_eq!(reader.next(), Err(TlvError::TruncatedHeader));
    }

    #[test]
    fn reader_rejects_truncated_value() {
        let bytes = [0x01, 0x00, 0x04, 0x00, 0xAA];
        let mut reader = TlvReader::new(&bytes);
        assert_eq!(reader.next(), Err(TlvError::TruncatedValue));
    }

    #[test]
    fn reader_parses_multiple_fields() {
        let mut bytes = Vec::new();
        write_tlv(&mut bytes, 1, &[0xAA]);
        write_tlv(&mut bytes, 2, &[0xBB, 0xCC]);

        let mut reader = TlvReader::new(&bytes);
        let first = reader.next().expect("tlv should decode").unwrap();
        assert_eq!(first.tlv_type, 1);
        assert_eq!(first.value, &[0xAA]);
        let second = reader.next().expect("tlv should decode").unwrap();
        assert_eq!(second.tlv_type, 2);
        assert_eq!(second.value, &[0xBB, 0xCC]);
        assert_eq!(reader.next(), Ok(None));
    }
}
