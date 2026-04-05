//! Transport error types.

/// Errors from the HID transport layer.
#[derive(Debug, thiserror::Error)]
pub enum TransportError {
    /// The requested device was not found.
    #[error("device not found")]
    DeviceNotFound,

    /// An I/O error occurred communicating with the device.
    #[error("i/o error: {0}")]
    Io(String),

    /// Timed out waiting for a report from the device.
    #[error("timeout waiting for report")]
    Timeout,

    /// The device was disconnected.
    #[error("device disconnected")]
    Disconnected,
}
