//! Power state and auto-shutdown (Report IDs 0xCC and 0xFF type 9).
//!
//! ## SetPowerState (Report ID 0xCC)
//!
//! 1-byte payload:
//! ```text
//! Byte 0: PowerDownState enum (0=Invalid, 1=Reset, 2=Shutdown)
//! ```
//!
//! ## SetAutoShutdown (Report ID 0xFF, type 9)
//!
//! 10-byte payload. Exact sub-command header not fully confirmed; the
//! timeout is encoded as a `u16` in minutes. Placeholder until
//! disassembly of the full header is complete.

use crate::legacy::types::{PowerDownState, ReportId};
use crate::report::Report;

/// Encode a set-power-state report (report ID 0xCC).
///
/// Sends a single-byte payload containing the [`PowerDownState`] discriminant.
#[must_use]
pub fn encode_set_power_state(state: PowerDownState) -> Report {
    Report::with_payload(0xCC, &[state as u8]).unwrap_or_else(|| Report::new(0xCC))
}

/// Convenience: encode a reset command (report ID 0xCC, [`PowerDownState::Reset`]).
#[must_use]
pub fn encode_reset() -> Report {
    encode_set_power_state(PowerDownState::Reset)
}

/// Convenience: encode a shutdown command (report ID 0xCC, [`PowerDownState::Shutdown`]).
#[must_use]
pub fn encode_shutdown() -> Report {
    encode_set_power_state(PowerDownState::Shutdown)
}

/// Encode an auto-shutdown trigger report (report ID 0xFF, type 9).
///
/// This is a **trigger**, not a timer configuration. When sent, the headset
/// beeps and powers down. The host is responsible for tracking inactivity
/// and deciding when to send this (hence "HostControlled" in the manifest).
///
/// Header bytes extracted from `LegacyHeadsetResetFeature::Impl::autoShutdown()`
/// magic constant `0x7003010004ff0008`.
///
/// The payload is padded to 63 bytes for the 0xFF feature report.
#[must_use]
pub fn encode_auto_shutdown_trigger() -> Report {
    let mut payload = [0u8; super::sidetone::FEATURE_REPORT_PAYLOAD_SIZE];
    payload[0] = 0x08; // report size
    // payload[1] = 0x00
    payload[2] = 0xFF; // report ID echo
    payload[3] = 0x04;
    // payload[4] = 0x00
    payload[5] = 0x01;
    payload[6] = 0x03;
    payload[7] = 0x70;
    payload[8] = 0x07; // beep/mode flag
    // payload[9] = 0x00

    Report::with_payload(ReportId::Extended as u8, &payload)
        .unwrap_or_else(|| Report::new(ReportId::Extended as u8))
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn encode_power_state_reset() {
        let report = encode_set_power_state(PowerDownState::Reset);
        assert_eq!(report.id(), 0xCC);
        assert_eq!(report.payload()[0], PowerDownState::Reset as u8);
        assert_eq!(report.payload()[0], 1);
    }

    #[test]
    fn encode_power_state_shutdown() {
        let report = encode_set_power_state(PowerDownState::Shutdown);
        assert_eq!(report.id(), 0xCC);
        assert_eq!(report.payload()[0], PowerDownState::Shutdown as u8);
        assert_eq!(report.payload()[0], 2);
    }

    #[test]
    fn encode_power_state_invalid() {
        let report = encode_set_power_state(PowerDownState::Invalid);
        assert_eq!(report.id(), 0xCC);
        assert_eq!(report.payload()[0], 0);
    }

    #[test]
    fn encode_reset_convenience() {
        let report = encode_reset();
        assert_eq!(report.id(), 0xCC);
        assert_eq!(report.payload()[0], PowerDownState::Reset as u8);
    }

    #[test]
    fn encode_shutdown_convenience() {
        let report = encode_shutdown();
        assert_eq!(report.id(), 0xCC);
        assert_eq!(report.payload()[0], PowerDownState::Shutdown as u8);
    }

    #[test]
    fn auto_shutdown_trigger_header() {
        let report = encode_auto_shutdown_trigger();
        assert_eq!(report.id(), 0xFF);
        assert_eq!(report.len(), 63); // padded to feature report size
        let p = report.payload();
        assert_eq!(p[0], 0x08); // report size field
        assert_eq!(p[2], 0xFF); // report ID echo
        assert_eq!(p[7], 0x70); // header byte 7
        assert_eq!(p[8], 0x07); // beep/mode flag
    }
}
