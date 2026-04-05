//! Corsair Legacy Headset Protocol.
//!
//! Used by VOID, HS60, HS70, and other older headsets. Communication is via
//! HID reports on two vendor-specific usage pages:
//!
//! - **0xFFC5**: output reports (commands to device)
//! - **0xFF00**: feature reports
//!
//! Report IDs 0x64–0x66 are device-to-host (input), 0xC8–0xFF are
//! host-to-device (output). Reports are 64 bytes, zero-padded. The report ID
//! is sent as a separate HID parameter, not embedded in the data buffer.

pub mod types;

pub mod firmware;
pub mod lighting;
pub mod mode;
pub mod pairing;
pub mod power;
pub mod sidetone;
pub mod state;
pub mod value;

pub use types::{
    BatteryState, HidReportType, LinkState, OperatingMode, PowerDownState, ReportId, ReportType,
    ValueId,
};
