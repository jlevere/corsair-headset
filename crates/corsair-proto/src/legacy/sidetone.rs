//! Sidetone control (Report ID 0xFF, ReportType 10) and mute (via SetValue 0xCA).
//!
//! Sidetone feeds the headset microphone audio back into the earphones at a
//! configurable level so the user doesn't feel isolated ("audio passthrough").
//!
//! The level is sent as a logarithmic dB value using the formula:
//! `level_db = (int)(20.0 * log10(level / 100.0 + 0.000631))`
//!
//! The `0.000631` offset (~10^-3.2) prevents `log10(0)` and sets a noise floor.
//!
//! ## Report structure (SetSidetoneLevel, Report ID 0xFF)
//!
//! 11-byte payload with sub-command header:
//! ```text
//! Byte  0: 0x0B  (report size = 11)
//! Byte  1: 0x00
//! Byte  2: 0xFF  (report ID echo)
//! Byte  3: 0x04
//! Byte  4: 0x0E
//! Byte  5: 0x01
//! Byte  6: 0x05
//! Byte  7: 0x01
//! Byte  8: 0x04
//! Byte  9: 0x00
//! Byte 10: <level_db>  (logarithmic dB value)
//! ```
//!
//! ## Mute/unmute (via SetValue report ID 0xCA)
//!
//! Sidetone mute uses the SetValue report with [`ValueId::SidetoneState`].
//! Non-silent mute also sets `AudioIndicationsState` and an extended value 0x104.

use crate::legacy::types::ValueId;
use crate::report::Report;

/// Convert a linear percentage (0–100) to the logarithmic dB byte sent to the device.
///
/// Formula: `truncate(20 * log10(level / 100 + 1))`
///
/// This maps 0% → 0 dB, 50% → 3 dB, 100% → 6 dB. The `+1` offset ensures
/// log10 is always positive (log10(1) = 0 at level=0).
///
/// Verified against live VOID Elite Wireless hardware.
#[must_use]
pub fn level_to_db(percent: u8) -> u8 {
    #[cfg(feature = "std")]
    {
        let level = f64::from(percent.min(100));
        let db = 20.0 * (level / 100.0 + 1.0).log10();
        db as u8
    }
    #[cfg(not(feature = "std"))]
    {
        // Lookup table for no_std: 20 * log10(p/100 + 1)
        match percent.min(100) {
            0..=14 => 0,
            15..=34 => 1,
            35..=49 => 2,
            50..=64 => 3,
            65..=79 => 4,
            80..=94 => 5,
            95..=100 => 6,
            _ => 6,
        }
    }
}

/// Convert a dB byte back to an approximate linear percentage.
#[must_use]
pub fn db_to_level(db: u8) -> u8 {
    #[cfg(feature = "std")]
    {
        // Inverse: level = 100 * (10^(db/20) - 1)
        let level = 100.0 * (10.0_f64.powf(f64::from(db) / 20.0) - 1.0);
        (level.round() as u8).min(100)
    }
    #[cfg(not(feature = "std"))]
    {
        match db {
            0 => 0,
            1 => 22,
            2 => 41,
            3 => 58,
            4 => 74,
            5 => 88,
            6 => 100,
            _ => 100,
        }
    }
}

/// HID descriptor payload size for report ID 0xFF (feature report).
///
/// macOS requires feature report writes to exactly match this size.
pub const FEATURE_REPORT_PAYLOAD_SIZE: usize = 63;

/// Encode a set-sidetone-level report (report ID 0xFF, feature report).
///
/// The 11-byte sub-command header was extracted from the magic constant
/// `0x0105010e04ff000b` in iCUE's `setSidetoneLevel` transaction.
/// The remaining bytes are zero-padded to 63 to match the HID descriptor.
#[must_use]
pub fn encode_set_sidetone_level(percent: u8) -> Report {
    let db = level_to_db(percent);
    let mut payload = [0u8; FEATURE_REPORT_PAYLOAD_SIZE];
    // Sub-command header (11 bytes, rest is zero-padded)
    payload[0] = 0x0B; // report size
    // payload[1] = 0x00
    payload[2] = 0xFF; // report ID echo
    payload[3] = 0x04;
    payload[4] = 0x0E;
    payload[5] = 0x01;
    payload[6] = 0x05;
    payload[7] = 0x01;
    payload[8] = 0x04; // sub-command
    // payload[9] = 0x00
    payload[10] = db; // sidetone level in dB

    Report::with_payload(0xFF, &payload).unwrap_or_else(|| Report::new(0xFF))
}

/// Encode SetValue reports to mute/unmute sidetone.
///
/// `muted`: whether to mute sidetone.
/// `silent`: if false, also sends AudioIndicationsState and extended value.
///
/// Returns 1 report (silent=true) or 3 reports (silent=false).
pub fn encode_set_sidetone_mute(muted: bool, silent: bool) -> Vec<Report> {
    let mute_byte = u8::from(muted);
    let mut reports = Vec::new();

    if !silent {
        // Report 1: AudioIndicationsState = mute_byte
        reports.push(
            Report::with_payload(0xCA, &[ValueId::AudioIndicationsState as u8, mute_byte])
                .unwrap_or_else(|| Report::new(0xCA)),
        );
    }

    // Report 2 (or 1 if silent): SidetoneState = mute_byte
    reports.push(
        Report::with_payload(0xCA, &[ValueId::SidetoneState as u8, mute_byte])
            .unwrap_or_else(|| Report::new(0xCA)),
    );

    if !silent {
        // Report 3: Extended value 0x04, 0x01 = 0
        let extended_id: u8 = 0x04; // high byte of 0x0104
        reports.push(
            Report::with_payload(0xCA, &[extended_id, 0x01, 0x00])
                .unwrap_or_else(|| Report::new(0xCA)),
        );
    }

    reports
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn level_to_db_zero() {
        // 20 * log10(0/100 + 1) = 20 * log10(1) = 0
        assert_eq!(level_to_db(0), 0);
    }

    #[test]
    fn level_to_db_max() {
        // 20 * log10(100/100 + 1) = 20 * log10(2) ≈ 6.02 → truncated to 6
        assert_eq!(level_to_db(100), 6);
    }

    #[test]
    fn level_to_db_mid() {
        // 20 * log10(50/100 + 1) = 20 * log10(1.5) ≈ 3.52 → 3
        assert_eq!(level_to_db(50), 3);
    }

    #[test]
    fn encode_sidetone_report_structure() {
        let report = encode_set_sidetone_level(50);
        assert_eq!(report.id(), 0xFF);
        let p = report.payload();
        assert_eq!(p[0], 0x0B); // size
        assert_eq!(p[2], 0xFF); // report ID echo
        assert_eq!(p[8], 0x04); // sub-command
        assert_eq!(p[10], level_to_db(50));
    }

    #[test]
    fn encode_mute_silent() {
        let reports = encode_set_sidetone_mute(true, true);
        assert_eq!(reports.len(), 1);
        assert_eq!(reports[0].id(), 0xCA);
        assert_eq!(reports[0].payload()[0], ValueId::SidetoneState as u8);
        assert_eq!(reports[0].payload()[1], 1); // muted = true
    }

    #[test]
    fn encode_mute_non_silent() {
        let reports = encode_set_sidetone_mute(true, false);
        assert_eq!(reports.len(), 3);
        // First: AudioIndicationsState
        assert_eq!(reports[0].payload()[0], ValueId::AudioIndicationsState as u8);
        // Second: SidetoneState
        assert_eq!(reports[1].payload()[0], ValueId::SidetoneState as u8);
    }
}
