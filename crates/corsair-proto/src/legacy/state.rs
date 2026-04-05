//! State report (Report ID 0x64) — battery, mic boom, buttons, link state.
//!
//! The HID descriptor declares a **9-byte** payload for this report. The
//! first 4 bytes contain the core state fields; bytes 4–8 contain
//! additional flags (I2C state, device state, error state, power state)
//! that are not yet fully decoded.
//!
//! ```text
//! Byte 0: Pressed buttons bitmap
//! Byte 1: Battery level (bits 0–6 = 0–127), bit 7 = mic boom state
//! Byte 2: LinkState (lower nibble, bits 0–3)
//! Byte 3: BatteryState enum (lower 3 bits)
//! Bytes 4–8: Additional flags (TBD)
//! ```

use crate::error::DecodeError;
use crate::legacy::types::{BatteryState, LinkState, ReportId};
use crate::report::Report;

/// Decoded state report.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct HeadsetState {
    /// Pressed buttons bitmap.
    pub buttons: u8,
    /// Battery level as a percentage (0–100, clamped from 0–127 raw).
    pub battery_percent: u8,
    /// Whether the mic boom is up (mic muted on most models).
    pub mic_boom_up: bool,
    /// Wireless link state.
    pub link_state: LinkState,
    /// Battery charging state.
    pub battery_state: BatteryState,
}

/// Decode a state report (report ID 0x64).
///
/// `report` should be parsed from a HID input callback via [`Report::from_input`].
pub fn decode_state(report: &Report) -> Result<HeadsetState, DecodeError> {
    if report.id() != ReportId::State as u8 {
        return Err(DecodeError::WrongReportId {
            expected: ReportId::State as u8,
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

    let buttons = p[0];

    // Byte 1: bits 0–6 = battery %, bit 7 = mic boom (0=up, 1=down per sign check).
    let raw_battery = p[1];
    let battery_percent = (raw_battery & 0x7F).min(100);
    // Sign bit check: `cmp byte, 0; cmovns` → bit 7 set means boom is down (not up).
    let mic_boom_up = (raw_battery & 0x80) == 0;

    // Byte 2: lower nibble = LinkState.
    let link_state = LinkState::from_nibble(p[2]).unwrap_or(LinkState::Invalid);

    // Byte 3: lower 3 bits = BatteryState.
    let battery_state = BatteryState::from_bits(p[3]).ok_or(DecodeError::InvalidValue {
        field: "battery_state",
        value: p[3],
    })?;

    Ok(HeadsetState {
        buttons,
        battery_percent,
        mic_boom_up,
        link_state,
        battery_state,
    })
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn decode_state_basic() {
        // buttons=0, battery=75% boom down, link=active, state=ok
        let report = Report::from_input(&[0x64, 0x00, 0x80 | 75, 0x01, 0x01]).unwrap();
        let state = decode_state(&report).unwrap();
        assert_eq!(state.buttons, 0);
        assert_eq!(state.battery_percent, 75);
        assert!(!state.mic_boom_up); // bit 7 set = boom down
        assert_eq!(state.link_state, LinkState::Active);
        assert_eq!(state.battery_state, BatteryState::Ok);
    }

    #[test]
    fn decode_state_mic_boom_up() {
        // battery=50 (no bit 7 = boom up), link=standby, charging
        let report = Report::from_input(&[0x64, 0x00, 50, 0x04, 0x05]).unwrap();
        let state = decode_state(&report).unwrap();
        assert_eq!(state.battery_percent, 50);
        assert!(state.mic_boom_up);
        assert_eq!(state.link_state, LinkState::Standby);
        assert_eq!(state.battery_state, BatteryState::Charging);
        assert!(state.battery_state.is_charging());
    }

    #[test]
    fn decode_state_wrong_id() {
        let report = Report::from_input(&[0x65, 0x00, 0x00, 0x00, 0x00]).unwrap();
        assert!(decode_state(&report).is_err());
    }

    #[test]
    fn decode_state_too_short() {
        let report = Report::from_input(&[0x64, 0x00, 0x00]).unwrap();
        assert!(decode_state(&report).is_err());
    }
}
