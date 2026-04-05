//! Device operating mode (Report ID 0x65 input, 0xC8 output).
//!
//! 2-byte payload:
//! - Byte 0: `OperatingMode` enum (Hardware=0, Software=1)
//! - Byte 1: bit 0 = media events disabled flag

use crate::error::DecodeError;
use crate::legacy::types::{OperatingMode, ReportId};
use crate::report::Report;

/// Decoded device mode report.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct DeviceMode {
    pub mode: OperatingMode,
    pub media_events_disabled: bool,
}

/// Decode a device mode input report (report ID 0x65).
pub fn decode_device_mode(report: &Report) -> Result<DeviceMode, DecodeError> {
    if report.id() != ReportId::DeviceModeIn as u8 {
        return Err(DecodeError::WrongReportId {
            expected: ReportId::DeviceModeIn as u8,
            actual: report.id(),
        });
    }

    if report.len() < 2 {
        return Err(DecodeError::TooShort {
            need: 2,
            got: report.len(),
        });
    }
    let p = report.payload();

    let mode = OperatingMode::from_byte(p[0]).ok_or(DecodeError::InvalidValue {
        field: "operating_mode",
        value: p[0],
    })?;

    let media_events_disabled = (p[1] & 0x01) != 0;

    Ok(DeviceMode {
        mode,
        media_events_disabled,
    })
}

/// Encode a set-device-mode output report (report ID 0xC8).
#[must_use]
pub fn encode_set_device_mode(mode: OperatingMode) -> Report {
    Report::with_payload(ReportId::DeviceModeOut as u8, &[mode as u8])
        .unwrap_or_else(|| Report::new(ReportId::DeviceModeOut as u8))
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn decode_hardware_mode() {
        let report = Report::from_input(&[0x65, 0x00, 0x00]).unwrap();
        let dm = decode_device_mode(&report).unwrap();
        assert_eq!(dm.mode, OperatingMode::Hardware);
        assert!(!dm.media_events_disabled);
    }

    #[test]
    fn decode_software_mode_media_disabled() {
        let report = Report::from_input(&[0x65, 0x01, 0x01]).unwrap();
        let dm = decode_device_mode(&report).unwrap();
        assert_eq!(dm.mode, OperatingMode::Software);
        assert!(dm.media_events_disabled);
    }

    #[test]
    fn encode_software_mode() {
        let report = encode_set_device_mode(OperatingMode::Software);
        assert_eq!(report.id(), 0xC8);
        assert_eq!(report.payload()[0], 1);
    }
}
