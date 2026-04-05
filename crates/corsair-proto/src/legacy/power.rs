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

use crate::legacy::types::PowerDownState;
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

/// Encode a set-auto-shutdown-timer report (report ID 0xFF, type 9).
///
/// `timeout_minutes`: auto-shutdown timeout in minutes (0 = disabled).
///
/// The payload is 10 bytes. The exact sub-command header is TBD and will be
/// updated once the full `SetAutoShutdownReportLayout` is confirmed from
/// disassembly. Currently the timeout is placed at bytes 8-9 (LE) with the
/// preceding bytes zeroed as placeholders.
#[must_use]
pub fn encode_set_auto_shutdown(timeout_minutes: u16) -> Report {
    let [lo, hi] = timeout_minutes.to_le_bytes();
    let payload: [u8; 10] = [
        0x00, // placeholder byte 0
        0x00, // placeholder byte 1
        0x00, // placeholder byte 2
        0x00, // placeholder byte 3
        0x00, // placeholder byte 4
        0x00, // placeholder byte 5
        0x00, // placeholder byte 6
        0x00, // placeholder byte 7
        lo,   // timeout minutes low byte
        hi,   // timeout minutes high byte
    ];
    Report::with_payload(0xFF, &payload).unwrap_or_else(|| Report::new(0xFF))
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
    fn encode_auto_shutdown_payload_size() {
        let report = encode_set_auto_shutdown(30);
        assert_eq!(report.id(), 0xFF);
        assert_eq!(report.len(), 10);
    }

    #[test]
    fn encode_auto_shutdown_timeout_encoding() {
        let report = encode_set_auto_shutdown(300);
        let p = report.payload();
        // 300 = 0x012C -> LE: [0x2C, 0x01]
        assert_eq!(p[8], 0x2C);
        assert_eq!(p[9], 0x01);
    }

    #[test]
    fn encode_auto_shutdown_disabled() {
        let report = encode_set_auto_shutdown(0);
        let p = report.payload();
        assert_eq!(p[8], 0x00);
        assert_eq!(p[9], 0x00);
    }
}
