//! CAPE (CX20805) command protocol.
//!
//! The CAPE chip uses 60-byte command payloads sent via HID report ID 0x01
//! in 62-byte reports (2-byte header + 60-byte payload).
//!
//! ## CAPE_FW_FULL_COMMAND layout (60 bytes)
//!
//! ```text
//! Offset  Size  Field
//! 0x00    4     command_header:
//!               bits[15:0]  = num_32b_words (negative in responses)
//!               bits[30:16] = command_id
//!               bit[31]     = direction (0=request, 1=response)
//! 0x04    4     app_module_id (includes magic 0xB32D23)
//! 0x08    52    data payload (up to 13 x u32 words)
//! ```
//!
//! ## HID report format
//!
//! ```text
//! Byte 0:    0x01 (HID Report ID)
//! Byte 1:    0x00 (padding)
//! Bytes 2–61: CAPE command payload (60 bytes)
//! ```

use crate::error::DecodeError;
use crate::report::Report;

/// Size of a CAPE command payload.
pub const CAPE_PAYLOAD_SIZE: usize = 60;

/// Size of the full HID report for CAPE commands.
pub const CAPE_REPORT_SIZE: usize = 62;

/// CAPE report ID.
pub const CAPE_REPORT_ID: u8 = 0x01;

/// A CAPE firmware command.
#[derive(Debug, Clone)]
pub struct CapeCommand {
    /// Number of 32-bit data words.
    pub num_words: u16,
    /// Command identifier.
    pub command_id: u16,
    /// Module identifier (includes CAPE magic bytes).
    pub module_id: u32,
    /// Data payload (up to 52 bytes / 13 dwords).
    pub data: [u8; 52],
}

impl CapeCommand {
    /// Create a new CAPE command.
    #[must_use]
    pub const fn new(command_id: u16, module_id: u32, num_words: u16) -> Self {
        Self {
            num_words,
            command_id,
            module_id,
            data: [0u8; 52],
        }
    }

    /// Build the 4-byte command header for a request.
    #[must_use]
    pub const fn request_header(&self) -> u32 {
        (self.num_words as u32) | ((self.command_id as u32) << 16)
    }

    /// Encode this command into a HID report.
    #[must_use]
    pub fn encode(&self) -> Report {
        let mut payload = [0u8; CAPE_REPORT_SIZE];
        // payload[0] is padding (0x00) — report ID is separate
        let header = self.request_header().to_le_bytes();
        let module = self.module_id.to_le_bytes();
        payload[0..4].copy_from_slice(&header);
        payload[4..8].copy_from_slice(&module);
        let data_len = ((self.num_words as usize) * 4).min(52);
        payload[8..8 + data_len].copy_from_slice(&self.data[..data_len]);
        Report::with_payload(CAPE_REPORT_ID, &payload[..CAPE_PAYLOAD_SIZE])
            .unwrap_or_else(|| Report::new(CAPE_REPORT_ID))
    }

    /// Decode a CAPE response from a HID input report.
    ///
    /// Validates report ID, payload length, and response direction bit (bit 31).
    pub fn decode_response(report: &Report) -> Result<Self, DecodeError> {
        if report.id() != CAPE_REPORT_ID {
            return Err(DecodeError::WrongReportId {
                expected: CAPE_REPORT_ID,
                actual: report.id(),
            });
        }
        if report.len() < CAPE_PAYLOAD_SIZE {
            return Err(DecodeError::TooShort {
                need: CAPE_PAYLOAD_SIZE,
                got: report.len(),
            });
        }
        let p = report.payload();
        let header = u32::from_le_bytes([p[0], p[1], p[2], p[3]]);
        // Bit 31 set = response direction.
        if header & 0x8000_0000 == 0 {
            return Err(DecodeError::InvalidValue {
                field: "cape_direction",
                value: 0,
            });
        }
        let num_words = (header & 0xFFFF) as u16;
        let command_id = ((header >> 16) & 0x7FFF) as u16;
        let module_id = u32::from_le_bytes([p[4], p[5], p[6], p[7]]);
        let mut data = [0u8; 52];
        data.copy_from_slice(&p[8..60]);
        Ok(Self {
            num_words,
            command_id,
            module_id,
            data,
        })
    }
}

/// Encode a CAPE "get firmware version" command.
#[must_use]
pub fn encode_get_firmware_version() -> Report {
    let cmd = CapeCommand::new(
        0x0006, // command_id
        0x01B32D23, // module_id (magic + module 1)
        4, // num_words
    );
    cmd.encode()
}

/// Encode a CAPE "read DRAM dword" command.
#[must_use]
pub fn encode_read_dram(mem_type: u8, address: u32) -> Report {
    let mut cmd = CapeCommand::new(
        0x0006,
        0x01B32D23,
        4,
    );
    cmd.data[0..4].copy_from_slice(&(mem_type as u32).to_le_bytes());
    cmd.data[4..8].copy_from_slice(&address.to_le_bytes());
    cmd.encode()
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn encode_firmware_version_cmd() {
        let report = encode_get_firmware_version();
        assert_eq!(report.id(), 0x01);
        let p = report.payload();
        // header: num_words=4, command_id=6
        let header = u32::from_le_bytes([p[0], p[1], p[2], p[3]]);
        assert_eq!(header & 0xFFFF, 4); // num_words
        assert_eq!((header >> 16) & 0x7FFF, 6); // command_id
        // module_id
        let module = u32::from_le_bytes([p[4], p[5], p[6], p[7]]);
        assert_eq!(module, 0x01B32D23);
    }

    #[test]
    fn decode_cape_response() {
        let mut data = [0u8; 64];
        // Build a response header: direction=1, command_id=6, num_words=4
        let header: u32 = 0x8000_0000 | (6 << 16) | 4;
        data[0..4].copy_from_slice(&header.to_le_bytes());
        data[4..8].copy_from_slice(&0x01B32D23u32.to_le_bytes());
        // Version data at offset 8
        data[8] = 0x03; // major
        data[12] = 0x0A; // minor

        let report = Report::with_payload(0x01, &data[..60]).unwrap();
        let resp = CapeCommand::decode_response(&report).unwrap();
        assert_eq!(resp.command_id, 6);
        assert_eq!(resp.num_words, 4);
        assert_eq!(resp.data[0], 0x03);
        assert_eq!(resp.data[4], 0x0A);
    }
}
