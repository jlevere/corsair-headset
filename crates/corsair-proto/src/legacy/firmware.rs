//! Firmware version report (Report ID 0x66).
//!
//! 4-byte payload: 2-byte transmitter version + 2-byte receiver version.

use crate::error::DecodeError;
use crate::legacy::types::ReportId;
use crate::report::Report;

/// Decoded firmware version.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct FirmwareVersion {
    pub major: u8,
    pub minor: u8,
}

impl core::fmt::Display for FirmwareVersion {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{}.{}", self.major, self.minor)
    }
}

/// Decoded firmware version report.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct FirmwareInfo {
    /// Transmitter (headset) firmware version.
    pub transmitter: FirmwareVersion,
    /// Receiver (dongle) firmware version.
    pub receiver: FirmwareVersion,
}

/// Decode a firmware version report (report ID 0x66).
pub fn decode_firmware_version(report: &Report) -> Result<FirmwareInfo, DecodeError> {
    if report.id() != ReportId::FirmwareVersion as u8 {
        return Err(DecodeError::WrongReportId {
            expected: ReportId::FirmwareVersion as u8,
            actual: report.id(),
        });
    }

    if report.len() < 4 {
        return Err(DecodeError::TooShort {
            need: 4,
            got: report.len(),
        });
    }
    let p = report.payload();

    Ok(FirmwareInfo {
        transmitter: FirmwareVersion {
            major: p[0],
            minor: p[1],
        },
        receiver: FirmwareVersion {
            major: p[2],
            minor: p[3],
        },
    })
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use crate::report::Report;

    #[test]
    fn decode_firmware() {
        let report = Report::from_input(&[0x66, 0x03, 0x0A, 0x01, 0x05]).unwrap();
        let info = decode_firmware_version(&report).unwrap();
        assert_eq!(info.transmitter, FirmwareVersion { major: 3, minor: 10 });
        assert_eq!(info.receiver, FirmwareVersion { major: 1, minor: 5 });
        assert_eq!(info.transmitter.to_string(), "3.10");
    }
}
