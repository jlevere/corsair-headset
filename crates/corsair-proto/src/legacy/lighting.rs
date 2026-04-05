//! Direct LED control (Report ID 0xCB, ReportTypes 6 and 7).
//!
//! Controls headset LEDs directly via TI LP5562 and CMA (Conexant) register
//! writes. Two report variants exist:
//!
//! ## DirectLedControlReportLayout (ReportType 6) — 19 bytes payload
//!
//! ```text
//! Byte  0:      count of register/value pairs (0–8)
//! Bytes 1–16:   up to 8 pairs of (TI_register, value)
//! Bytes 17–18:  padding (0x00)
//! ```
//!
//! ## CmaDirectLedControlReportLayout (ReportType 7) — 7 bytes payload
//!
//! ```text
//! Byte  0:    count of register/value pairs (0–2)
//! Bytes 1–6:  up to 3 pairs of (CMA_register, value)
//! ```
//!
//! ## LED Zone Mapping
//!
//! | Zone ID | Name       | Colors | TI PWM registers     | TI brightness registers |
//! |---------|------------|--------|----------------------|------------------------|
//! | 0x213   | LeftLogo   | RGB    | 0x1C, 0x16, 0x17    | 0x0C, 0x06, 0x07       |
//! | 0x214   | RightLogo  | RGB    | 0x1D, 0x18, 0x19    | 0x0D, 0x08, 0x09       |
//! | 0x215   | Status     | RG     | 0x1B, 0x1A           | 0x0B, 0x0A             |
//! | 0x216   | MicMute    | R      | 0x1E                 | 0x0E                   |

use crate::report::Report;

/// HID report ID for direct LED control.
const LED_REPORT_ID: u8 = 0xCB;

/// Maximum register/value pairs in a DirectLedControl report (ReportType 6).
const TI_MAX_PAIRS: usize = 8;

/// Maximum register/value pairs in a CmaDirectLedControl report (ReportType 7).
const CMA_MAX_PAIRS: usize = 3;

// ---------------------------------------------------------------------------
// LED zones
// ---------------------------------------------------------------------------

/// LED zones on the headset.
///
/// Zone IDs match the values used in iCUE's zone manifest (0x213–0x216).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[repr(u16)]
pub enum LedZone {
    /// Left earcup logo — full RGB.
    LeftLogo = 0x213,
    /// Right earcup logo — full RGB.
    RightLogo = 0x214,
    /// Status indicator — red and green only.
    Status = 0x215,
    /// Microphone mute indicator — red only.
    MicMute = 0x216,
}

impl LedZone {
    /// Decode from a raw 16-bit zone ID.
    #[must_use]
    pub const fn from_id(id: u16) -> Option<Self> {
        match id {
            0x213 => Some(Self::LeftLogo),
            0x214 => Some(Self::RightLogo),
            0x215 => Some(Self::Status),
            0x216 => Some(Self::MicMute),
            _ => None,
        }
    }

    /// Return the TI register mapping for this zone.
    #[must_use]
    pub const fn ti_mapping(&self) -> ZoneMapping {
        match self {
            Self::LeftLogo => ZoneMapping {
                pwm_red: Some(0x1C),
                pwm_green: Some(0x16),
                pwm_blue: Some(0x17),
                bright_red: Some(0x0C),
                bright_green: Some(0x06),
                bright_blue: Some(0x07),
            },
            Self::RightLogo => ZoneMapping {
                pwm_red: Some(0x1D),
                pwm_green: Some(0x18),
                pwm_blue: Some(0x19),
                bright_red: Some(0x0D),
                bright_green: Some(0x08),
                bright_blue: Some(0x09),
            },
            Self::Status => ZoneMapping {
                pwm_red: Some(0x1B),
                pwm_green: Some(0x1A),
                pwm_blue: None,
                bright_red: Some(0x0B),
                bright_green: Some(0x0A),
                bright_blue: None,
            },
            Self::MicMute => ZoneMapping {
                pwm_red: Some(0x1E),
                pwm_green: None,
                pwm_blue: None,
                bright_red: Some(0x0E),
                bright_green: None,
                bright_blue: None,
            },
        }
    }
}

// ---------------------------------------------------------------------------
// Zone register mapping
// ---------------------------------------------------------------------------

/// TI LP5562 register addresses for a single LED zone.
///
/// Each color component has a PWM register (for color value) and a brightness
/// register. Zones with fewer color channels have `None` for missing components.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ZoneMapping {
    /// PWM register for red channel.
    pub pwm_red: Option<u8>,
    /// PWM register for green channel.
    pub pwm_green: Option<u8>,
    /// PWM register for blue channel.
    pub pwm_blue: Option<u8>,
    /// Brightness register for red channel.
    pub bright_red: Option<u8>,
    /// Brightness register for green channel.
    pub bright_green: Option<u8>,
    /// Brightness register for blue channel.
    pub bright_blue: Option<u8>,
}

impl ZoneMapping {
    /// Return an iterator over (pwm_register, brightness_register) pairs for
    /// all color channels present in this zone.
    pub fn channels(&self) -> impl Iterator<Item = (u8, u8)> + '_ {
        let pairs: [Option<(u8, u8)>; 3] = [
            self.pwm_red.zip(self.bright_red),
            self.pwm_green.zip(self.bright_green),
            self.pwm_blue.zip(self.bright_blue),
        ];
        pairs.into_iter().flatten()
    }

    /// Return all PWM registers present in this zone.
    pub fn pwm_registers(&self) -> impl Iterator<Item = u8> + '_ {
        [self.pwm_red, self.pwm_green, self.pwm_blue]
            .into_iter()
            .flatten()
    }

    /// Return all brightness registers present in this zone.
    pub fn brightness_registers(&self) -> impl Iterator<Item = u8> + '_ {
        [self.bright_red, self.bright_green, self.bright_blue]
            .into_iter()
            .flatten()
    }
}

// ---------------------------------------------------------------------------
// CMA register mapping (for RightLogo zone via CMA variant)
// ---------------------------------------------------------------------------

/// CMA (Conexant) register addresses for the right logo zone.
///
/// Used with CmaDirectLedControl (ReportType 7) to control zone 0x214
/// on headsets using the CMA path instead of TI.
pub mod cma {
    /// CMA register for the red channel.
    pub const RED: u8 = 0x1C;
    /// CMA register for the green channel.
    pub const GREEN: u8 = 0x16;
    /// CMA register for the blue channel.
    pub const BLUE: u8 = 0x17;
}

// ---------------------------------------------------------------------------
// All TI PWM registers (for clearPwm / setLogarithmicPwm)
// ---------------------------------------------------------------------------

/// All PWM registers across every zone, used by `clearPwm` and `setLogarithmicPwm`.
const ALL_PWM_REGISTERS: [u8; 8] = [
    0x1C, 0x16, 0x17, // LeftLogo  R, G, B
    0x1D, 0x18, 0x19, // RightLogo R, G, B
    0x1B, 0x1A,       // Status    R, G
];

/// All brightness registers across every zone, used by `setLogarithmicPwm`.
const ALL_BRIGHTNESS_REGISTERS: [u8; 8] = [
    0x0C, 0x06, 0x07, // LeftLogo  R, G, B
    0x0D, 0x08, 0x09, // RightLogo R, G, B
    0x0B, 0x0A,       // Status    R, G
];

/// MicMute PWM register (handled separately since it's a single channel).
const MIC_MUTE_PWM: u8 = 0x1E;

/// MicMute brightness register.
const MIC_MUTE_BRIGHTNESS: u8 = 0x0E;

// ---------------------------------------------------------------------------
// TI engine start/stop register
// ---------------------------------------------------------------------------

/// TI LP5562 engine enable register.
const TI_ENABLE_REG: u8 = 0x01;

/// Value to start TI engines (run mode for all three engines).
const TI_ENGINE_START: u8 = 0x2A;

/// Value to stop TI engines (disable all engines).
const TI_ENGINE_STOP: u8 = 0x00;

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

/// Build a DirectLedControl report (ReportType 6) from register/value pairs.
///
/// The 19-byte payload layout is:
/// - byte 0: count
/// - bytes 1–16: up to 8 (register, value) pairs
/// - bytes 17–18: padding
///
/// Silently truncates to [`TI_MAX_PAIRS`] if more pairs are provided.
#[must_use]
fn encode_ti_report(pairs: &[(u8, u8)]) -> Report {
    let count = pairs.len().min(TI_MAX_PAIRS);
    let mut payload = [0u8; 19];
    payload[0] = count as u8;
    for (i, &(reg, val)) in pairs.iter().take(count).enumerate() {
        payload[1 + i * 2] = reg;
        payload[2 + i * 2] = val;
    }
    Report::with_payload(LED_REPORT_ID, &payload).unwrap_or_else(|| Report::new(LED_REPORT_ID))
}

/// Build a CmaDirectLedControl report (ReportType 7) from register/value pairs.
///
/// The 7-byte payload layout is:
/// - byte 0: count
/// - bytes 1–6: up to 3 (register, value) pairs
///
/// Silently truncates to [`CMA_MAX_PAIRS`] if more pairs are provided.
#[must_use]
fn encode_cma_report(pairs: &[(u8, u8)]) -> Report {
    let count = pairs.len().min(CMA_MAX_PAIRS);
    let mut payload = [0u8; 7];
    payload[0] = count as u8;
    for (i, &(reg, val)) in pairs.iter().take(count).enumerate() {
        payload[1 + i * 2] = reg;
        payload[2 + i * 2] = val;
    }
    Report::with_payload(LED_REPORT_ID, &payload).unwrap_or_else(|| Report::new(LED_REPORT_ID))
}

// ---------------------------------------------------------------------------
// Public encode functions
// ---------------------------------------------------------------------------

/// Encode a "start TI engines" report.
///
/// Writes `0x2A` to the TI LP5562 enable register (0x01), activating all
/// three LED engine sequencers in run mode.
#[must_use]
pub fn encode_start_ti_engines() -> Report {
    encode_ti_report(&[(TI_ENABLE_REG, TI_ENGINE_START)])
}

/// Encode a "stop TI engines" report.
///
/// Writes `0x00` to the TI LP5562 enable register (0x01), disabling all
/// LED engine sequencers.
#[must_use]
pub fn encode_stop_ti_engines() -> Report {
    encode_ti_report(&[(TI_ENABLE_REG, TI_ENGINE_STOP)])
}

/// Encode a set-brightness report for a specific zone.
///
/// Converts a percentage (0–100) to a PWM byte (0–255) using:
/// `pwm = (level / 100.0 * 255.0) as u8`
///
/// Writes the computed PWM value to all brightness registers for the zone.
/// Returns a vector of reports (one register write may need multiple reports
/// if a zone has more channels than fit in a single report, though in practice
/// all zones fit in one).
#[must_use]
pub fn encode_set_brightness(zone: LedZone, level: u8) -> Report {
    let clamped = level.min(100);
    let pwm = (f64::from(clamped) / 100.0 * 255.0) as u8;
    let mapping = zone.ti_mapping();

    let mut pairs: [(u8, u8); 3] = [(0, 0); 3];
    let mut count = 0;
    for reg in mapping.brightness_registers() {
        pairs[count] = (reg, pwm);
        count += 1;
    }
    encode_ti_report(&pairs[..count])
}

/// Encode a set-color report for a specific zone via TI registers.
///
/// Writes RGB values to the zone's PWM registers. For zones with fewer
/// channels (Status = RG, MicMute = R only), extra components are ignored.
#[must_use]
pub fn encode_set_color(zone: LedZone, r: u8, g: u8, b: u8) -> Report {
    let mapping = zone.ti_mapping();
    let mut pairs: [(u8, u8); 3] = [(0, 0); 3];
    let mut count = 0;

    if let Some(reg) = mapping.pwm_red {
        pairs[count] = (reg, r);
        count += 1;
    }
    if let Some(reg) = mapping.pwm_green {
        pairs[count] = (reg, g);
        count += 1;
    }
    if let Some(reg) = mapping.pwm_blue {
        pairs[count] = (reg, b);
        count += 1;
    }

    encode_ti_report(&pairs[..count])
}

/// Encode a set-color report for the right logo zone via CMA registers.
///
/// Uses the CmaDirectLedControl report variant (ReportType 7) to write
/// RGB values through the Conexant audio codec path.
#[must_use]
pub fn encode_set_color_cma(r: u8, g: u8, b: u8) -> Report {
    encode_cma_report(&[(cma::RED, r), (cma::GREEN, g), (cma::BLUE, b)])
}

/// Encode a "set logarithmic PWM" report.
///
/// When `enabled` is true, writes `0x20` (bit 5 set) to the logarithmic PWM
/// registers for all zones. When false, writes `0x00`.
///
/// The TI LP5562 supports logarithmic dimming for a more perceptually linear
/// brightness curve.
///
/// Returns a vector of reports since all zones' registers must be written.
pub fn encode_set_logarithmic_pwm(enabled: bool) -> Vec<Report> {
    let value = if enabled { 0x20 } else { 0x00 };
    let mut reports = Vec::new();

    // All PWM registers for the main zones (8 registers) — fits in one TI report
    let mut pairs: [(u8, u8); TI_MAX_PAIRS] = [(0, 0); TI_MAX_PAIRS];
    for (i, &reg) in ALL_PWM_REGISTERS.iter().enumerate() {
        pairs[i] = (reg, value);
    }
    reports.push(encode_ti_report(&pairs));

    // MicMute PWM register in a separate report
    reports.push(encode_ti_report(&[(MIC_MUTE_PWM, value)]));

    reports
}

/// Encode "clear PWM" reports — zero all PWM registers across every zone.
///
/// Returns a vector of reports since all zones' registers must be written.
pub fn encode_clear_pwm() -> Vec<Report> {
    let mut reports = Vec::new();

    // All main-zone PWM registers (8 registers fits in one TI report)
    let mut pairs: [(u8, u8); TI_MAX_PAIRS] = [(0, 0); TI_MAX_PAIRS];
    for (i, &reg) in ALL_PWM_REGISTERS.iter().enumerate() {
        pairs[i] = (reg, 0x00);
    }
    reports.push(encode_ti_report(&pairs));

    // MicMute PWM register
    reports.push(encode_ti_report(&[(MIC_MUTE_PWM, 0x00)]));

    // All main-zone brightness registers (8 registers fits in one TI report)
    let mut bright_pairs: [(u8, u8); TI_MAX_PAIRS] = [(0, 0); TI_MAX_PAIRS];
    for (i, &reg) in ALL_BRIGHTNESS_REGISTERS.iter().enumerate() {
        bright_pairs[i] = (reg, 0x00);
    }
    reports.push(encode_ti_report(&bright_pairs));

    // MicMute brightness register
    reports.push(encode_ti_report(&[(MIC_MUTE_BRIGHTNESS, 0x00)]));

    reports
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn led_zone_from_id_roundtrip() {
        assert_eq!(LedZone::from_id(0x213), Some(LedZone::LeftLogo));
        assert_eq!(LedZone::from_id(0x214), Some(LedZone::RightLogo));
        assert_eq!(LedZone::from_id(0x215), Some(LedZone::Status));
        assert_eq!(LedZone::from_id(0x216), Some(LedZone::MicMute));
        assert_eq!(LedZone::from_id(0x999), None);
    }

    #[test]
    fn zone_mapping_left_logo_has_all_channels() {
        let m = LedZone::LeftLogo.ti_mapping();
        assert_eq!(m.pwm_red, Some(0x1C));
        assert_eq!(m.pwm_green, Some(0x16));
        assert_eq!(m.pwm_blue, Some(0x17));
        assert_eq!(m.bright_red, Some(0x0C));
        assert_eq!(m.bright_green, Some(0x06));
        assert_eq!(m.bright_blue, Some(0x07));
    }

    #[test]
    fn zone_mapping_status_has_rg_only() {
        let m = LedZone::Status.ti_mapping();
        assert!(m.pwm_red.is_some());
        assert!(m.pwm_green.is_some());
        assert!(m.pwm_blue.is_none());
        assert!(m.bright_blue.is_none());
    }

    #[test]
    fn zone_mapping_mic_mute_has_red_only() {
        let m = LedZone::MicMute.ti_mapping();
        assert!(m.pwm_red.is_some());
        assert!(m.pwm_green.is_none());
        assert!(m.pwm_blue.is_none());
    }

    #[test]
    fn zone_mapping_channels_iterator() {
        let m = LedZone::LeftLogo.ti_mapping();
        let channels: Vec<_> = m.channels().collect();
        assert_eq!(channels.len(), 3);
        assert_eq!(channels[0], (0x1C, 0x0C)); // red
        assert_eq!(channels[1], (0x16, 0x06)); // green
        assert_eq!(channels[2], (0x17, 0x07)); // blue

        let m = LedZone::MicMute.ti_mapping();
        let channels: Vec<_> = m.channels().collect();
        assert_eq!(channels.len(), 1);
        assert_eq!(channels[0], (0x1E, 0x0E)); // red only
    }

    #[test]
    fn start_ti_engines_report() {
        let report = encode_start_ti_engines();
        assert_eq!(report.id(), 0xCB);
        let p = report.payload();
        assert_eq!(p[0], 1);    // count = 1 pair
        assert_eq!(p[1], 0x01); // TI enable register
        assert_eq!(p[2], 0x2A); // engine start value
    }

    #[test]
    fn stop_ti_engines_report() {
        let report = encode_stop_ti_engines();
        assert_eq!(report.id(), 0xCB);
        let p = report.payload();
        assert_eq!(p[0], 1);    // count = 1 pair
        assert_eq!(p[1], 0x01); // TI enable register
        assert_eq!(p[2], 0x00); // engine stop value
    }

    #[test]
    fn set_brightness_zero() {
        let report = encode_set_brightness(LedZone::LeftLogo, 0);
        let p = report.payload();
        assert_eq!(p[0], 3); // LeftLogo has 3 brightness registers
        // All brightness values should be 0
        assert_eq!(p[2], 0); // value for first register
        assert_eq!(p[4], 0); // value for second register
        assert_eq!(p[6], 0); // value for third register
    }

    #[test]
    fn set_brightness_full() {
        let report = encode_set_brightness(LedZone::LeftLogo, 100);
        let p = report.payload();
        assert_eq!(p[0], 3);
        assert_eq!(p[2], 255); // (100/100 * 255) = 255
        assert_eq!(p[4], 255);
        assert_eq!(p[6], 255);
    }

    #[test]
    fn set_brightness_half() {
        let report = encode_set_brightness(LedZone::RightLogo, 50);
        let p = report.payload();
        assert_eq!(p[0], 3);
        // (50/100 * 255) = 127.5, truncated to 127
        assert_eq!(p[2], 127);
    }

    #[test]
    fn set_brightness_clamps_at_100() {
        let report = encode_set_brightness(LedZone::MicMute, 200);
        let p = report.payload();
        assert_eq!(p[0], 1); // MicMute has 1 brightness register
        assert_eq!(p[2], 255); // clamped to 100 -> 255
    }

    #[test]
    fn set_color_left_logo_rgb() {
        let report = encode_set_color(LedZone::LeftLogo, 0xFF, 0x80, 0x40);
        assert_eq!(report.id(), 0xCB);
        let p = report.payload();
        assert_eq!(p[0], 3);    // 3 color channels
        assert_eq!(p[1], 0x1C); // red PWM register
        assert_eq!(p[2], 0xFF); // red value
        assert_eq!(p[3], 0x16); // green PWM register
        assert_eq!(p[4], 0x80); // green value
        assert_eq!(p[5], 0x17); // blue PWM register
        assert_eq!(p[6], 0x40); // blue value
    }

    #[test]
    fn set_color_status_rg_only() {
        let report = encode_set_color(LedZone::Status, 0xAA, 0xBB, 0xCC);
        let p = report.payload();
        assert_eq!(p[0], 2);    // Status only has R and G
        assert_eq!(p[1], 0x1B); // red PWM register
        assert_eq!(p[2], 0xAA);
        assert_eq!(p[3], 0x1A); // green PWM register
        assert_eq!(p[4], 0xBB);
        // Blue is ignored for Status zone
    }

    #[test]
    fn set_color_mic_mute_red_only() {
        let report = encode_set_color(LedZone::MicMute, 0xFF, 0x00, 0x00);
        let p = report.payload();
        assert_eq!(p[0], 1);    // MicMute only has R
        assert_eq!(p[1], 0x1E); // red PWM register
        assert_eq!(p[2], 0xFF);
    }

    #[test]
    fn set_color_cma_rgb() {
        let report = encode_set_color_cma(0x11, 0x22, 0x33);
        assert_eq!(report.id(), 0xCB);
        let p = report.payload();
        assert_eq!(p[0], 3);    // 3 pairs
        assert_eq!(p[1], 0x1C); // CMA red register
        assert_eq!(p[2], 0x11);
        assert_eq!(p[3], 0x16); // CMA green register
        assert_eq!(p[4], 0x22);
        assert_eq!(p[5], 0x17); // CMA blue register
        assert_eq!(p[6], 0x33);
    }

    #[test]
    fn set_logarithmic_pwm_enabled() {
        let reports = encode_set_logarithmic_pwm(true);
        assert_eq!(reports.len(), 2);

        // First report: 8 main-zone PWM registers set to 0x20
        let p = reports[0].payload();
        assert_eq!(p[0], 8); // 8 pairs
        for i in 0..8 {
            assert_eq!(p[2 + i * 2], 0x20, "pair {i} value should be 0x20");
        }

        // Second report: MicMute PWM register
        let p = reports[1].payload();
        assert_eq!(p[0], 1);
        assert_eq!(p[1], 0x1E); // MicMute PWM register
        assert_eq!(p[2], 0x20);
    }

    #[test]
    fn set_logarithmic_pwm_disabled() {
        let reports = encode_set_logarithmic_pwm(false);
        assert_eq!(reports.len(), 2);

        let p = reports[0].payload();
        assert_eq!(p[0], 8);
        for i in 0..8 {
            assert_eq!(p[2 + i * 2], 0x00, "pair {i} value should be 0x00");
        }
    }

    #[test]
    fn clear_pwm_reports() {
        let reports = encode_clear_pwm();
        assert_eq!(reports.len(), 4);

        // All reports should use LED_REPORT_ID
        for r in &reports {
            assert_eq!(r.id(), 0xCB);
        }

        // First: 8 main-zone PWM registers zeroed
        assert_eq!(reports[0].payload()[0], 8);
        // Second: MicMute PWM zeroed
        assert_eq!(reports[1].payload()[0], 1);
        assert_eq!(reports[1].payload()[1], 0x1E);
        // Third: 8 main-zone brightness registers zeroed
        assert_eq!(reports[2].payload()[0], 8);
        // Fourth: MicMute brightness zeroed
        assert_eq!(reports[3].payload()[0], 1);
        assert_eq!(reports[3].payload()[1], 0x0E);

        // Verify all values are zero
        for report in &reports {
            let p = report.payload();
            let count = p[0] as usize;
            for i in 0..count {
                assert_eq!(p[2 + i * 2], 0x00, "all values should be zeroed");
            }
        }
    }

    #[test]
    fn ti_report_payload_is_19_bytes() {
        // The TI report payload should always be exactly 19 bytes
        let report = encode_start_ti_engines();
        // Payload length should be 19
        assert_eq!(report.len(), 19);
    }

    #[test]
    fn cma_report_payload_is_7_bytes() {
        let report = encode_set_color_cma(0, 0, 0);
        assert_eq!(report.len(), 7);
    }

    #[test]
    fn right_logo_ti_mapping() {
        let m = LedZone::RightLogo.ti_mapping();
        assert_eq!(m.pwm_red, Some(0x1D));
        assert_eq!(m.pwm_green, Some(0x18));
        assert_eq!(m.pwm_blue, Some(0x19));
        assert_eq!(m.bright_red, Some(0x0D));
        assert_eq!(m.bright_green, Some(0x08));
        assert_eq!(m.bright_blue, Some(0x09));
    }
}
