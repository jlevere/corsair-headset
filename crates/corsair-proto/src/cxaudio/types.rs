//! CxAudio chip types and constants.

/// Conexant USB Vendor ID.
pub const CONEXANT_VID: u16 = 0x0572;

/// Conexant default USB Product ID.
pub const CONEXANT_PID: u16 = 0x1410;

/// CxAudio library version string.
pub const LIB_VERSION: &str = "1.0.48.0";

/// Maximum retries for CAPE SendCommand polling.
pub const CAPE_MAX_RETRIES: u32 = 999;

/// CAPE magic bytes in the module ID field.
pub const CAPE_MAGIC: u32 = 0xB32D23;

/// Chip variant / device type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[repr(u8)]
pub enum ChipType {
    /// CX20562 — legacy chip, report IDs 0x08/0x09.
    Cx20562 = 0,
    /// CX2070x — report IDs 0x04/0x05, 33-byte reports.
    Cx2070x = 1,
    /// CX2076x — same report format as CX2070x.
    Cx2076x = 2,
    /// CX20805 (CAPE) — report ID 0x01, 62-byte reports.
    Cx20805 = 6,
}

/// HID report IDs per chip variant.
impl ChipType {
    /// Output report ID for this chip.
    #[must_use]
    pub const fn output_report_id(self) -> u8 {
        match self {
            Self::Cx20562 => 0x08,
            Self::Cx2070x | Self::Cx2076x => 0x04,
            Self::Cx20805 => 0x01,
        }
    }

    /// Input report ID for this chip.
    #[must_use]
    pub const fn input_report_id(self) -> u8 {
        match self {
            Self::Cx20562 => 0x09,
            Self::Cx2070x | Self::Cx2076x => 0x05,
            Self::Cx20805 => 0x01,
        }
    }

    /// Output report size in bytes.
    #[must_use]
    pub const fn output_report_size(self) -> usize {
        match self {
            Self::Cx20562 => 3,
            Self::Cx2070x | Self::Cx2076x => 33,
            Self::Cx20805 => 62,
        }
    }

    /// Input report size in bytes.
    #[must_use]
    pub const fn input_report_size(self) -> usize {
        match self {
            Self::Cx20562 => 6,
            Self::Cx2070x | Self::Cx2076x => 37,
            Self::Cx20805 => 62,
        }
    }
}

/// Memory type for register/EEPROM operations.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[repr(u8)]
pub enum MemType {
    Eeprom = 0,
    Rom = 1,
    Dram = 2,
    Register = 3,
}

/// Memory operation type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[repr(u8)]
pub enum MemOp {
    Read = 0,
    Write = 1,
}
