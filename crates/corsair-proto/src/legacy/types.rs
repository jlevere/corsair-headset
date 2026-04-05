//! Core types for the Legacy Headset Protocol.
//!
//! ## HID Usage Pages
//!
//! | Page   | Use |
//! |--------|-----|
//! | 0x000C | Consumer Control (volume, media keys) |
//! | 0xFFC5 | Corsair vendor output/input reports |
//! | 0xFF00 | Corsair vendor feature reports |
//! | 0x000A | Ordinal (LED, SetValue, extended I/O) |
//!
//! ## Report Payload Sizes (from HID descriptor)
//!
//! macOS IOKit requires writes to exactly match these sizes.
//!
//! | Report ID | Type    | Payload | Usage Page |
//! |-----------|---------|---------|------------|
//! | 0x64      | Input   | 9 B     | 0xFFC5     |
//! | 0x65      | Input   | 2 B     | 0xFFC5     |
//! | 0x66      | Input   | 4 B     | 0xFFC5     |
//! | 0xC8      | Output  | 2 B     | 0xFFC5     |
//! | 0xC9      | Output  | 1 B     | 0xFFC5     |
//! | 0xCA      | Output  | 4 B     | 0x000A     |
//! | 0xCB      | Output  | 19 B    | 0x000A     |
//! | 0xCC      | Output  | 1 B     | 0xFFC5     |
//! | 0xFF      | Feature | 63 B    | 0x000A     |

/// HID usage page for vendor output/input reports.
pub const USAGE_PAGE_OUTPUT: u16 = 0xFFC5;

/// HID usage page for vendor feature reports.
pub const USAGE_PAGE_FEATURE: u16 = 0xFF00;

/// HID usage page for ordinal reports (LED, SetValue, extended I/O).
pub const USAGE_PAGE_ORDINAL: u16 = 0x000A;

/// Inter-report delay within a transaction (nanoseconds).
pub const TRANSACTION_DELAY_NS: u64 = 35_000_000; // 35ms

/// Timeout for a single report send (milliseconds).
pub const REPORT_TIMEOUT_MS: u64 = 200;

/// Timeout for feature report operations (milliseconds).
pub const FEATURE_REPORT_TIMEOUT_MS: u64 = 5000;

// ---------------------------------------------------------------------------
// Report IDs
// ---------------------------------------------------------------------------

/// HID report IDs used by the Legacy Headset Protocol.
///
/// Input reports (device -> host): 0x64–0x66.
/// Output reports (host -> device): 0xC8–0xCC, 0xFF.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum ReportId {
    // -- Input (device -> host) --
    /// State report: battery, mic boom, buttons, link state.
    State = 0x64,
    /// Device operating mode notification.
    DeviceModeIn = 0x65,
    /// Firmware version (transmitter + receiver).
    FirmwareVersion = 0x66,

    // -- Output (host -> device) --
    /// Set device operating mode.
    DeviceModeOut = 0xC8,
    /// Request data from device.
    RequestData = 0xC9,
    /// Set a device value (by [`ValueId`]).
    SetValue = 0xCA,
    /// Direct LED color control.
    DirectLedControl = 0xCB,
    /// Set power state (shutdown/reset).
    SetPowerState = 0xCC,
    /// Multipurpose report ID for sidetone, auto-shutdown, pairing, link state.
    Extended = 0xFF,
}

impl ReportId {
    /// Convert from a raw byte, returning `None` for unrecognized IDs.
    #[must_use]
    pub const fn from_byte(b: u8) -> Option<Self> {
        match b {
            0x64 => Some(Self::State),
            0x65 => Some(Self::DeviceModeIn),
            0x66 => Some(Self::FirmwareVersion),
            0xC8 => Some(Self::DeviceModeOut),
            0xC9 => Some(Self::RequestData),
            0xCA => Some(Self::SetValue),
            0xCB => Some(Self::DirectLedControl),
            0xCC => Some(Self::SetPowerState),
            0xFF => Some(Self::Extended),
            _ => None,
        }
    }
}

// ---------------------------------------------------------------------------
// Report type enum (matches the headset_reports::ReportType enum)
// ---------------------------------------------------------------------------

/// Internal report type discriminator matching the protocol's `ReportType` enum.
///
/// Multiple types can share the same [`ReportId`] (e.g., types 9, 10, 12, 13
/// all use report ID 0xFF and are differentiated by sub-command header bytes).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum ReportType {
    /// Input: state (battery, mic boom, buttons, link state). Report ID 0x64.
    State = 0,
    /// Input: device mode notification. Report ID 0x65.
    DeviceModeIn = 1,
    /// Input: firmware version. Report ID 0x66.
    FirmwareVersion = 2,
    /// Output: set device mode. Report ID 0xC8.
    DeviceModeOut = 3,
    /// Output: request data. Report ID 0xC9.
    RequestData = 4,
    /// Output: set value. Report ID 0xCA.
    SetValue = 5,
    /// Output: direct LED control. Report ID 0xCB.
    DirectLedControl = 6,
    /// Output: CMA-variant LED control. Report ID 0xCB.
    CmaDirectLedControl = 7,
    /// Output: set power state. Report ID 0xCC.
    SetPowerState = 8,
    /// Output: set auto-shutdown timer. Report ID 0xFF.
    SetAutoShutdown = 9,
    /// Output: set sidetone level. Report ID 0xFF.
    SetSidetoneLevel = 10,
    /// Output: apply sidetone level. Report ID 0xFF.
    ApplySidetoneLevel = 11,
    /// Output: start wireless pairing. Report ID 0xFF.
    StartPairing = 12,
    /// Output: link state notification. Report ID 0xFF.
    LinkStateNotify = 13,
}

// ---------------------------------------------------------------------------
// HID report type (output vs feature)
// ---------------------------------------------------------------------------

/// Which HID report type to use when sending a report.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum HidReportType {
    /// Sent via `IOHIDDeviceSetReport` with `kIOHIDReportTypeOutput` on usage page 0xFFC5.
    Output = 0,
    /// Sent via `IOHIDDeviceSetReport` with `kIOHIDReportTypeFeature` on usage page 0xFF00.
    Feature = 1,
}

// ---------------------------------------------------------------------------
// Battery state (from QMetaObject BatteryState enum)
// ---------------------------------------------------------------------------

/// Battery charging/discharging state.
///
/// Encoded as the lower 3 bits of the state report's byte 3.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[repr(u8)]
pub enum BatteryState {
    /// Unknown or not reported.
    Invalid = 0,
    /// Normal battery level.
    Ok = 1,
    /// Low battery.
    Low = 2,
    /// Critical — shutting down.
    Shutdown = 3,
    /// Fully charged.
    ChargeComplete = 4,
    /// Currently charging.
    Charging = 5,
    /// Battery error.
    Error = 6,
}

impl BatteryState {
    /// Decode from the raw 3-bit value.
    #[must_use]
    pub const fn from_bits(val: u8) -> Option<Self> {
        match val & 0x07 {
            0 => Some(Self::Invalid),
            1 => Some(Self::Ok),
            2 => Some(Self::Low),
            3 => Some(Self::Shutdown),
            4 => Some(Self::ChargeComplete),
            5 => Some(Self::Charging),
            6 => Some(Self::Error),
            _ => None,
        }
    }

    /// Whether the device is currently charging (Charging or ChargeComplete).
    #[must_use]
    pub const fn is_charging(self) -> bool {
        matches!(self, Self::Charging | Self::ChargeComplete)
    }
}

// ---------------------------------------------------------------------------
// Link state (from QMetaObject LinkState enum)
// ---------------------------------------------------------------------------

/// Wireless link state between headset and dongle.
///
/// Encoded as the lower nibble of state report byte 2.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[repr(u8)]
pub enum LinkState {
    Invalid = 0,
    Active = 1,
    Pair = 2,
    Search = 3,
    Standby = 4,
    ReSync = 5,
    InitialScan = 6,
    TestMode = 7,
    PairCancel = 8,
    ActiveSearch = 9,
    // 10 is unused
    ActivePair = 11,
    SearchViaSynchro = 12,
    PairViaSynchro = 13,
}

impl LinkState {
    #[must_use]
    pub const fn from_nibble(val: u8) -> Option<Self> {
        match val & 0x0F {
            0 => Some(Self::Invalid),
            1 => Some(Self::Active),
            2 => Some(Self::Pair),
            3 => Some(Self::Search),
            4 => Some(Self::Standby),
            5 => Some(Self::ReSync),
            6 => Some(Self::InitialScan),
            7 => Some(Self::TestMode),
            8 => Some(Self::PairCancel),
            9 => Some(Self::ActiveSearch),
            11 => Some(Self::ActivePair),
            12 => Some(Self::SearchViaSynchro),
            13 => Some(Self::PairViaSynchro),
            _ => None,
        }
    }

    /// Whether the link is actively connected.
    #[must_use]
    pub const fn is_connected(self) -> bool {
        matches!(self, Self::Active | Self::ActiveSearch | Self::ActivePair)
    }
}

// ---------------------------------------------------------------------------
// Operating mode (from QMetaObject OperatingMode enum)
// ---------------------------------------------------------------------------

/// Device operating mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[repr(u8)]
pub enum OperatingMode {
    /// Hardware mode (normal operation).
    Hardware = 0,
    /// Software mode (software-controlled).
    Software = 1,
}

impl OperatingMode {
    #[must_use]
    pub const fn from_byte(b: u8) -> Option<Self> {
        match b {
            0 => Some(Self::Hardware),
            1 => Some(Self::Software),
            _ => None,
        }
    }
}

// ---------------------------------------------------------------------------
// Power down state
// ---------------------------------------------------------------------------

/// Power down state for SetPowerState report (0xCC).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum PowerDownState {
    Invalid = 0,
    Reset = 1,
    Shutdown = 2,
}

// ---------------------------------------------------------------------------
// ValueId enum for SetValue report (0xCA)
// ---------------------------------------------------------------------------

/// Parameter selector for the SetValue report (report ID 0xCA).
///
/// Byte 0 of the SetValue payload selects which parameter to set.
/// Bytes 1–3 contain the value.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum ValueId {
    Invalid = 0,
    /// Active EQ preset index.
    EqIndex = 1,
    /// Surround sound on/off state.
    SurroundState = 2,
    /// Microphone mute state.
    MicState = 3,
    /// Audio indications (beeps/tones) state.
    AudioIndicationsState = 4,
    /// Sidetone mute state.
    SidetoneState = 5,
    /// Surround indicator LED state.
    SurroundIndicatorState = 6,
}

impl ValueId {
    #[must_use]
    pub const fn from_byte(b: u8) -> Option<Self> {
        match b {
            0 => Some(Self::Invalid),
            1 => Some(Self::EqIndex),
            2 => Some(Self::SurroundState),
            3 => Some(Self::MicState),
            4 => Some(Self::AudioIndicationsState),
            5 => Some(Self::SidetoneState),
            6 => Some(Self::SurroundIndicatorState),
            _ => None,
        }
    }
}
