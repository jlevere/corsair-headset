/// Maximum HID report payload size for Corsair devices.
///
/// The actual wire size varies per report ID (from 1 byte for RequestData
/// to 63 bytes for feature reports). This constant defines the maximum
/// internal buffer size.
pub const MAX_REPORT_SIZE: usize = 64;

/// A HID report with ID and variable-length payload.
///
/// The report ID is stored separately from the payload, matching how
/// `IOHIDDeviceSetReport` (macOS) and `hidapi` handle reports — the ID
/// is passed as a parameter, not embedded in the data buffer.
///
/// # Wire format
///
/// When sending via `hidapi::write()`, the buffer is `[report_id, payload...]`.
/// macOS IOKit requires the payload size to **exactly match** the size declared
/// in the device's HID report descriptor. Use [`wire_bytes`](Report::wire_bytes)
/// to get the correctly-sized buffer for sending.
#[derive(Clone)]
pub struct Report {
    /// HID report ID (e.g., 0xC8, 0xFF).
    id: u8,
    /// Payload buffer, zero-padded to `MAX_REPORT_SIZE`.
    buf: [u8; MAX_REPORT_SIZE],
    /// Actual meaningful payload length (bytes written, not including padding).
    len: usize,
}

impl Report {
    /// Create a new report with the given ID and an empty (zeroed) payload.
    #[must_use]
    pub const fn new(id: u8) -> Self {
        Self {
            id,
            buf: [0u8; MAX_REPORT_SIZE],
            len: 0,
        }
    }

    /// Create a report with the given ID and payload bytes.
    ///
    /// Payload is copied into the internal buffer. The `len` field tracks
    /// the meaningful payload size for [`wire_bytes`](Self::wire_bytes).
    /// Returns `None` if `payload` exceeds [`MAX_REPORT_SIZE`].
    #[must_use]
    pub fn with_payload(id: u8, payload: &[u8]) -> Option<Self> {
        if payload.len() > MAX_REPORT_SIZE {
            return None;
        }
        let mut buf = [0u8; MAX_REPORT_SIZE];
        buf[..payload.len()].copy_from_slice(payload);
        Some(Self {
            id,
            buf,
            len: payload.len(),
        })
    }

    /// Parse a report from raw HID input data where `data[0]` is the report ID.
    ///
    /// This is the format received from HID input report callbacks (hidapi
    /// `read()` on macOS prefixes the report ID).
    #[must_use]
    pub fn from_input(data: &[u8]) -> Option<Self> {
        if data.is_empty() {
            return None;
        }
        let id = data[0];
        let payload = &data[1..];
        if payload.len() > MAX_REPORT_SIZE {
            return None;
        }
        let mut buf = [0u8; MAX_REPORT_SIZE];
        buf[..payload.len()].copy_from_slice(payload);
        Some(Self {
            id,
            buf,
            len: payload.len(),
        })
    }

    /// The HID report ID.
    #[must_use]
    pub const fn id(&self) -> u8 {
        self.id
    }

    /// The payload data buffer (full internal buffer including zero-padding).
    ///
    /// For the meaningful portion only, use `&payload()[..len()]`.
    #[must_use]
    pub fn payload(&self) -> &[u8] {
        &self.buf
    }

    /// Mutable access to the payload buffer.
    pub fn payload_mut(&mut self) -> &mut [u8] {
        &mut self.buf
    }

    /// The meaningful payload length.
    #[must_use]
    pub const fn len(&self) -> usize {
        self.len
    }

    /// Whether the payload is empty.
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// The wire-format bytes for sending via `hidapi::write()`.
    ///
    /// Returns `[report_id, payload[0..len]]`. On macOS, `hidapi::write()`
    /// expects exactly this format — the report ID as the first byte,
    /// followed by the payload sized to match the HID descriptor.
    ///
    /// # Example
    ///
    /// ```
    /// # use corsair_proto::Report;
    /// let report = Report::with_payload(0xC9, &[0x64]).unwrap();
    /// assert_eq!(report.wire_bytes(), &[0xC9, 0x64]);
    /// ```
    pub fn wire_bytes(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(1 + self.len);
        out.push(self.id);
        out.extend_from_slice(&self.buf[..self.len]);
        out
    }

    /// The full internal buffer (zero-padded to [`MAX_REPORT_SIZE`]).
    ///
    /// Prefer [`wire_bytes`](Self::wire_bytes) for sending to the device.
    /// This is useful for protocol-level inspection or when the full buffer
    /// is needed (e.g., WebHID which accepts arbitrary sizes).
    #[must_use]
    pub fn as_bytes(&self) -> &[u8; MAX_REPORT_SIZE] {
        &self.buf
    }
}

impl core::fmt::Debug for Report {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "Report {{ id: 0x{:02X}, len: {}, [", self.id, self.len)?;
        let show = if self.len > 0 { self.len.min(16) } else { 4 };
        for (i, b) in self.buf[..show].iter().enumerate() {
            if i > 0 {
                write!(f, " ")?;
            }
            write!(f, "{b:02X}")?;
        }
        if self.len > 16 {
            write!(f, " ...")?;
        }
        write!(f, "] }}")
    }
}
