//! Platform-agnostic async transport for Corsair HID devices.
//!
//! The [`Transport`] trait provides low-level report I/O that protocol layers
//! ([`corsair_proto::legacy`] and [`corsair_proto::bragi`]) build upon.
//!
//! ## Architecture
//!
//! Corsair devices use two HID collections on different vendor-specific usage pages:
//! - **0xFFC5** — output reports (commands to device)
//! - **0xFF00** — feature reports
//!
//! Input reports (device→host) arrive asynchronously on the 0xFFC5 collection.
//! The transport broadcasts all of them; protocol-level filtering happens above.
//!
//! ## Backends
//!
//! - **`native`** (default feature): [`HidapiTransport`] + [`HidapiEnumerator`] via `hidapi` + `tokio`.

pub mod error;
pub mod paced;

#[cfg(feature = "native")]
pub mod native;

use std::pin::Pin;

use corsair_proto::Report;

pub use error::TransportError;
pub use paced::PacedTransport;

/// Which HID report mechanism to use when sending.
///
/// Corsair devices expose two vendor-specific HID collections. The report kind
/// determines which collection and IOKit report type to target.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HidReportKind {
    /// HID output report on usage page 0xFFC5.
    Output,
    /// HID feature report on usage page 0xFF00.
    Feature,
}

/// Information about a discovered Corsair HID device.
#[derive(Debug, Clone)]
pub struct CorsairDeviceInfo {
    /// USB vendor ID.
    pub vid: u16,
    /// USB product ID.
    pub pid: u16,
    /// Product name from the HID descriptor.
    pub name: Option<String>,
    /// OS-specific device path (used by `open()`).
    pub path: String,
    /// HID usage page this interface was found on.
    pub usage_page: u16,
    /// HID usage ID.
    pub usage: u16,
}

/// Low-level async HID transport for Corsair devices.
///
/// Handles raw report I/O. Protocol semantics (Legacy fire-and-forget vs
/// Bragi request/response) are built on top by the device layer.
#[async_trait::async_trait(?Send)]
pub trait Transport {
    /// Send a report to the device.
    ///
    /// `kind` selects the HID report type:
    /// - [`HidReportKind::Output`] → `SetReport(Output)` on usage page 0xFFC5
    /// - [`HidReportKind::Feature`] → `SetReport(Feature)` on usage page 0xFF00
    async fn send(&self, report: &Report, kind: HidReportKind) -> Result<(), TransportError>;

    /// Read a feature report from the device (HID `GetReport`).
    async fn get_feature_report(&self, report_id: u8) -> Result<Report, TransportError>;

    /// Subscribe to asynchronous input reports from the device.
    ///
    /// Returns a stream of **all** input reports. The caller filters by report ID.
    fn input_reports(&self) -> Pin<Box<dyn futures_core::Stream<Item = Report> + '_>>;
}

/// Enumerate and open Corsair HID devices.
#[async_trait::async_trait(?Send)]
pub trait Enumerator {
    /// List connected Corsair devices.
    ///
    /// Returns entries for each HID collection matching Corsair VID (0x1B1C)
    /// and vendor-specific usage pages.
    async fn enumerate(&self) -> Result<Vec<CorsairDeviceInfo>, TransportError>;

    /// Open a transport to a specific device.
    async fn open(&self, info: &CorsairDeviceInfo) -> Result<Box<dyn Transport>, TransportError>;
}
