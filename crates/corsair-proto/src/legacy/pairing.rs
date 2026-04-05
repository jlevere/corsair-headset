//! Wireless pairing (Report ID 0xFF, type 12) and link state (type 13).
//!
//! ## StartPairing (Report ID 0xFF, type 12)
//!
//! 3-byte payload:
//! ```text
//! Byte 0: 0x02
//! Byte 1: 0x00
//! Byte 2: 0x40  (start pairing command)
//! ```
//!
//! ## LinkStateNotify (Report ID 0xFF, type 13)
//!
//! 5-byte payload. Exact sub-command bytes not fully confirmed from
//! disassembly — the `toString()` method returns an empty string and no
//! named fields were found. Zeroed placeholder until live testing reveals
//! the correct header.

use crate::legacy::types::ReportId;
use crate::report::Report;

/// Encode a start-pairing report (report ID 0xFF, type 12).
///
/// Sends the 3-byte pairing initiation command extracted from the protocol's
/// `sendStartPairingCommand` at `0x1199e0` (`AudioFwHidUtilities`).
#[must_use]
pub fn encode_start_pairing() -> Report {
    let payload: [u8; 3] = [0x02, 0x00, 0x40];
    // SAFETY: 3 bytes always fits in REPORT_SIZE (64).
    Report::with_payload(ReportId::Extended as u8, &payload)
        .unwrap_or_else(|| Report::new(ReportId::Extended as u8))
}

/// Encode a link-state-notify report (report ID 0xFF, type 13).
///
/// 5-byte payload. **Placeholder** — exact sub-command header has not been
/// confirmed from disassembly. The `toString()` method returns an empty
/// string and no named fields were found. This report may need updating
/// after live device testing.
#[must_use]
pub fn encode_link_state_notify() -> Report {
    let payload: [u8; 5] = [0x00; 5];
    Report::with_payload(ReportId::Extended as u8, &payload)
        .unwrap_or_else(|| Report::new(ReportId::Extended as u8))
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn start_pairing_structure() {
        let report = encode_start_pairing();
        assert_eq!(report.id(), ReportId::Extended as u8);
        assert_eq!(report.len(), 3);
        let p = report.payload();
        assert_eq!(p[0], 0x02);
        assert_eq!(p[1], 0x00);
        assert_eq!(p[2], 0x40);
    }

    #[test]
    fn link_state_notify_structure() {
        let report = encode_link_state_notify();
        assert_eq!(report.id(), ReportId::Extended as u8);
        assert_eq!(report.len(), 5);
    }
}
