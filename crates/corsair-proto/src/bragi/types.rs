//! Core types and constants for the Bragi protocol.

// ---------------------------------------------------------------------------
// Timing constants
// ---------------------------------------------------------------------------

/// Standard inter-report wait time (milliseconds).
pub const WAIT_TIME_MS: u64 = 20;

/// Extended wait time for operations that need extra settling (milliseconds).
pub const WAIT_TIME_LONG_MS: u64 = 750;

/// Maximum wait time for slow operations like firmware queries (milliseconds).
pub const WAIT_TIME_VERY_LONG_MS: u64 = 2000;

// ---------------------------------------------------------------------------
// Battery status
// ---------------------------------------------------------------------------

/// Battery status reported by the Bragi `BatteryStatus` property.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[repr(u8)]
pub enum BatteryStatus {
    /// Discharging (normal operation on battery).
    Discharging = 1,
    /// Currently charging.
    Charging = 2,
    /// Fully charged.
    FullyCharged = 3,
}

impl BatteryStatus {
    /// Decode from the raw property value.
    #[must_use]
    pub const fn from_byte(val: u8) -> Option<Self> {
        match val {
            1 => Some(Self::Discharging),
            2 => Some(Self::Charging),
            3 => Some(Self::FullyCharged),
            _ => None,
        }
    }

    /// Whether the device is connected to a power source (charging or full).
    #[must_use]
    pub const fn is_on_power(self) -> bool {
        matches!(self, Self::Charging | Self::FullyCharged)
    }
}

// ---------------------------------------------------------------------------
// Operating mode
// ---------------------------------------------------------------------------

/// Device operating mode reported by the Bragi `OperatingMode` property.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[repr(u8)]
pub enum OperatingMode {
    /// Hardware mode (standalone, no host software control).
    Hardware = 0,
    /// Software mode (software-controlled).
    Software = 1,
}

impl OperatingMode {
    /// Decode from the raw property value.
    #[must_use]
    pub const fn from_byte(val: u8) -> Option<Self> {
        match val {
            0 => Some(Self::Hardware),
            1 => Some(Self::Software),
            _ => None,
        }
    }
}

// ---------------------------------------------------------------------------
// Wireless mode
// ---------------------------------------------------------------------------

/// Wireless connection mode reported by the Bragi `WirelessMode` property.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[repr(u8)]
pub enum WirelessMode {
    /// Connected via 2.4 GHz Slipstream wireless.
    Slipstream = 0,
    /// Connected via Bluetooth.
    Bluetooth = 1,
    /// Wired USB connection (wireless disabled).
    Wired = 2,
}

impl WirelessMode {
    /// Decode from the raw property value.
    #[must_use]
    pub const fn from_byte(val: u8) -> Option<Self> {
        match val {
            0 => Some(Self::Slipstream),
            1 => Some(Self::Bluetooth),
            2 => Some(Self::Wired),
            _ => None,
        }
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn battery_status_roundtrip() {
        assert_eq!(BatteryStatus::from_byte(1), Some(BatteryStatus::Discharging));
        assert_eq!(BatteryStatus::from_byte(2), Some(BatteryStatus::Charging));
        assert_eq!(BatteryStatus::from_byte(3), Some(BatteryStatus::FullyCharged));
        assert_eq!(BatteryStatus::from_byte(0), None);
        assert_eq!(BatteryStatus::from_byte(4), None);
    }

    #[test]
    fn battery_status_is_on_power() {
        assert!(!BatteryStatus::Discharging.is_on_power());
        assert!(BatteryStatus::Charging.is_on_power());
        assert!(BatteryStatus::FullyCharged.is_on_power());
    }

    #[test]
    fn operating_mode_roundtrip() {
        assert_eq!(OperatingMode::from_byte(0), Some(OperatingMode::Hardware));
        assert_eq!(OperatingMode::from_byte(1), Some(OperatingMode::Software));
        assert_eq!(OperatingMode::from_byte(2), None);
    }

    #[test]
    fn wireless_mode_roundtrip() {
        assert_eq!(WirelessMode::from_byte(0), Some(WirelessMode::Slipstream));
        assert_eq!(WirelessMode::from_byte(1), Some(WirelessMode::Bluetooth));
        assert_eq!(WirelessMode::from_byte(2), Some(WirelessMode::Wired));
        assert_eq!(WirelessMode::from_byte(3), None);
    }
}
