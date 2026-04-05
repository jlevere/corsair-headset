//! Set value (Report ID 0xCA) and request data (Report ID 0xC9).
//!
//! ## SetValue (0xCA)
//!
//! Generic "set parameter" command:
//! ```text
//! Byte 0: ValueId enum
//! Byte 1: value byte
//! ```
//!
//! ## RequestData (0xC9)
//!
//! Polls the device for a specific input report:
//! ```text
//! Byte 0: ReportId of the report to request
//! ```

use crate::legacy::types::{ReportId, ValueId};
use crate::report::Report;

// ---------------------------------------------------------------------------
// SetValue (0xCA)
// ---------------------------------------------------------------------------

/// Encode a set-value report with a typed [`ValueId`].
#[must_use]
pub fn encode_set_value(id: ValueId, value: u8) -> Report {
    // 2 bytes always fits in REPORT_SIZE.
    Report::with_payload(ReportId::SetValue as u8, &[id as u8, value])
        .unwrap_or_else(|| Report::new(ReportId::SetValue as u8))
}

/// Encode a mic mute command.
#[must_use]
pub fn encode_set_mic_mute(muted: bool) -> Report {
    encode_set_value(ValueId::MicState, u8::from(muted))
}

/// Encode an EQ preset index selection.
#[must_use]
pub fn encode_set_eq_preset(index: u8) -> Report {
    encode_set_value(ValueId::EqIndex, index)
}

/// Encode surround sound toggle.
#[must_use]
pub fn encode_set_surround(enabled: bool) -> Report {
    encode_set_value(ValueId::SurroundState, u8::from(enabled))
}

// ---------------------------------------------------------------------------
// RequestData (0xC9)
// ---------------------------------------------------------------------------

/// Encode a request-data report (report ID 0xC9).
///
/// The single payload byte is the [`ReportId`] of the input report to poll.
/// The device responds asynchronously with the requested report.
#[must_use]
pub fn encode_request_data(report_id: ReportId) -> Report {
    // 1 byte always fits in REPORT_SIZE.
    Report::with_payload(ReportId::RequestData as u8, &[report_id as u8])
        .unwrap_or_else(|| Report::new(ReportId::RequestData as u8))
}

/// Request the device send a state report (battery, mic boom, buttons, link).
#[must_use]
pub fn encode_request_state() -> Report {
    encode_request_data(ReportId::State)
}

/// Request the device send its current operating mode.
#[must_use]
pub fn encode_request_device_mode() -> Report {
    encode_request_data(ReportId::DeviceModeIn)
}

/// Request the device send its firmware version.
#[must_use]
pub fn encode_request_firmware_version() -> Report {
    encode_request_data(ReportId::FirmwareVersion)
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn set_value_mic_mute() {
        let report = encode_set_mic_mute(true);
        assert_eq!(report.id(), ReportId::SetValue as u8);
        assert_eq!(report.payload()[0], ValueId::MicState as u8);
        assert_eq!(report.payload()[1], 1);
    }

    #[test]
    fn set_value_eq_preset() {
        let report = encode_set_eq_preset(2);
        assert_eq!(report.payload()[0], ValueId::EqIndex as u8);
        assert_eq!(report.payload()[1], 2);
    }

    #[test]
    fn request_data_carries_report_id() {
        let report = encode_request_data(ReportId::State);
        assert_eq!(report.id(), ReportId::RequestData as u8);
        assert_eq!(report.payload()[0], ReportId::State as u8);
    }

    #[test]
    fn request_state() {
        let report = encode_request_state();
        assert_eq!(report.id(), ReportId::RequestData as u8);
        assert_eq!(report.payload()[0], ReportId::State as u8);
    }

    #[test]
    fn request_device_mode() {
        let report = encode_request_device_mode();
        assert_eq!(report.payload()[0], ReportId::DeviceModeIn as u8);
    }

    #[test]
    fn request_firmware_version() {
        let report = encode_request_firmware_version();
        assert_eq!(report.payload()[0], ReportId::FirmwareVersion as u8);
    }
}
