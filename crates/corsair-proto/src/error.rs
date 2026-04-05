/// Device-side protocol error returned in error responses.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum ProtocolError {
    /// Unknown or unrecognized error.
    Unknown,
    /// Invalid argument in the request.
    InvalidArgument,
    /// Value out of allowed range.
    OutOfRange,
    /// Hardware error on the device.
    HardwareError,
    /// Device is busy, try again.
    Busy,
    /// Requested feature/command not supported.
    Unsupported,
    /// Unrecognized error code from device.
    Other(u8),
}

impl ProtocolError {
    /// Decode an error code byte from the device.
    #[must_use]
    pub const fn from_code(code: u8) -> Self {
        match code {
            0x01 => Self::Unknown,
            0x02 => Self::InvalidArgument,
            0x03 => Self::OutOfRange,
            0x04 => Self::HardwareError,
            0x08 => Self::Busy,
            0x09 => Self::Unsupported,
            _ => Self::Other(code),
        }
    }
}

impl core::fmt::Display for ProtocolError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Unknown => write!(f, "unknown error"),
            Self::InvalidArgument => write!(f, "invalid argument"),
            Self::OutOfRange => write!(f, "out of range"),
            Self::HardwareError => write!(f, "hardware error"),
            Self::Busy => write!(f, "device busy"),
            Self::Unsupported => write!(f, "unsupported"),
            Self::Other(code) => write!(f, "error code 0x{code:02X}"),
        }
    }
}

/// Errors encountered when decoding a report.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DecodeError {
    /// The report payload was too short to parse.
    TooShort {
        need: usize,
        got: usize,
    },
    /// Unexpected report ID.
    WrongReportId {
        expected: u8,
        actual: u8,
    },
    /// Device returned a protocol error.
    DeviceError(ProtocolError),
    /// The value of a field was not a recognized enum variant.
    InvalidValue {
        field: &'static str,
        value: u8,
    },
}

impl core::fmt::Display for DecodeError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::TooShort { need, got } => {
                write!(f, "report too short: need {need} bytes, got {got}")
            }
            Self::WrongReportId { expected, actual } => {
                write!(
                    f,
                    "wrong report ID: expected 0x{expected:02X}, got 0x{actual:02X}"
                )
            }
            Self::DeviceError(e) => write!(f, "device error: {e}"),
            Self::InvalidValue { field, value } => {
                write!(f, "invalid {field} value: 0x{value:02X}")
            }
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for DecodeError {}

#[cfg(feature = "std")]
impl std::error::Error for ProtocolError {}
