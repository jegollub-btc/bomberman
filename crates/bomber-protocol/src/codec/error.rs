use thiserror::Error;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum ProtoError {
    #[error("unexpected end of input: need {need} more byte(s) at offset {at}")]
    Truncated { at: usize, need: usize },
    #[error("unknown frame type 0x{0:02x}")]
    UnknownFrame(u8),
    #[error("unknown delta record type 0x{0:02x}")]
    UnknownRecord(u8),
    #[error("invalid value {value} for {field}")]
    InvalidValue { field: &'static str, value: u32 },
    #[error("protocol version mismatch: got {got}, expected {expected}")]
    VersionMismatch { got: u8, expected: u8 },
    #[error("frame too large: {size} bytes exceeds the {max} byte datagram budget")]
    TooLarge { size: usize, max: usize },
}

impl ProtoError {
    /// Shorthand for the common "this byte is not a valid X" case.
    pub fn invalid(field: &'static str, value: impl Into<u32>) -> Self {
        ProtoError::InvalidValue {
            field,
            value: value.into(),
        }
    }
}

pub type Result<T> = std::result::Result<T, ProtoError>;
