//! Conexant (CxAudio) HID chip protocol.
//!
//! Low-level access to the Conexant audio chip on Corsair headsets.
//! The VOID RGB Elite uses the CX20805 (CAPE) variant.
//!
//! This module provides:
//! - CAPE command encode/decode for CX20805
//! - Register/EEPROM I/O for older CX20562/CX2070x chips
//!
//! ## Chip variants
//!
//! | DevType | Class | Chip | Report IDs (out/in) |
//! |---------|-------|------|---------------------|
//! | 0 | CX20562 | CAT | 0x08 / 0x09 |
//! | 1 | CX2070x | CHAN | 0x04 / 0x05 |
//! | 2 | CX2076x | CATP | 0x04 / 0x05 |
//! | 6 | CX20805 | CAPE | 0x01 / 0x01 |
//!
//! The CAPE protocol uses 62-byte HID reports with a 60-byte
//! [`CapeCommand`] payload.

pub mod cape;
pub mod types;
