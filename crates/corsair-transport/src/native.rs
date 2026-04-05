//! Native hidapi + tokio transport backend.
//!
//! [`HidapiEnumerator`] discovers Corsair HID devices, and [`HidapiTransport`]
//! provides async report I/O with a background reader task that broadcasts
//! all incoming input reports.

use std::pin::Pin;
use std::sync::Arc;
use std::time::Duration;

use corsair_proto::types::CORSAIR_VID;
use corsair_proto::Report;
use futures_core::Stream;
use tokio::sync::broadcast;
use tokio::sync::Mutex;
use tokio_stream::StreamExt as _;
use tokio_stream::wrappers::BroadcastStream;

use crate::error::TransportError;
use crate::{CorsairDeviceInfo, Enumerator, HidReportKind, Transport};

/// Corsair vendor-specific usage pages.
const USAGE_PAGE_OUTPUT: u16 = 0xFFC5;
const USAGE_PAGE_FEATURE: u16 = 0xFF00;

/// Timeout for HID read polling in milliseconds.
const READ_TIMEOUT_MS: i32 = 10;

/// Buffer size for input report broadcasts.
const BROADCAST_CAPACITY: usize = 64;

// ---------------------------------------------------------------------------
// Enumerator
// ---------------------------------------------------------------------------

/// Discovers Corsair HID devices via hidapi.
pub struct HidapiEnumerator {
    api: hidapi::HidApi,
}

impl HidapiEnumerator {
    /// Create a new enumerator. Initializes the hidapi library.
    ///
    /// On macOS, opens devices in exclusive mode (`kIOHIDOptionsTypeSeizeDevice`)
    /// which is required for output report writes. This takes the device from
    /// the macOS Event System.
    pub fn new() -> Result<Self, TransportError> {
        let api = hidapi::HidApi::new().map_err(|e| TransportError::Io(e.to_string()))?;
        // macOS: exclusive mode required for output report writes.
        // Without this, IOHIDDeviceSetReport fails with 0xE0005000.
        #[cfg(target_os = "macos")]
        api.set_open_exclusive(true);
        Ok(Self { api })
    }

    /// Refresh the device list (re-enumerates USB).
    pub fn refresh(&mut self) -> Result<(), TransportError> {
        self.api
            .refresh_devices()
            .map_err(|e| TransportError::Io(e.to_string()))
    }
}

#[async_trait::async_trait(?Send)]
impl Enumerator for HidapiEnumerator {
    async fn enumerate(&self) -> Result<Vec<CorsairDeviceInfo>, TransportError> {
        let mut devices = Vec::new();

        for dev in self.api.device_list() {
            if dev.vendor_id() != CORSAIR_VID.0 {
                continue;
            }

            let usage_page = dev.usage_page();
            if usage_page != USAGE_PAGE_OUTPUT && usage_page != USAGE_PAGE_FEATURE {
                continue;
            }

            devices.push(CorsairDeviceInfo {
                vid: dev.vendor_id(),
                pid: dev.product_id(),
                name: dev.product_string().map(String::from),
                path: dev.path().to_string_lossy().into_owned(),
                usage_page,
                usage: dev.usage(),
            });
        }

        Ok(devices)
    }

    async fn open(&self, info: &CorsairDeviceInfo) -> Result<Box<dyn Transport>, TransportError> {
        let device = self
            .api
            .open_path(&std::ffi::CString::new(info.path.as_bytes()).map_err(|e| {
                TransportError::Io(format!("invalid device path: {e}"))
            })?)
            .map_err(|e| TransportError::Io(e.to_string()))?;

        device
            .set_blocking_mode(false)
            .map_err(|e| TransportError::Io(e.to_string()))?;

        let transport = HidapiTransport::spawn(device);
        Ok(Box::new(transport))
    }
}

// ---------------------------------------------------------------------------
// Transport
// ---------------------------------------------------------------------------

/// Native hidapi transport with a background reader task.
///
/// Input reports are read by a background tokio task and broadcast to all
/// subscribers. Sends are done directly on the HID device handle.
pub struct HidapiTransport {
    device: Arc<Mutex<hidapi::HidDevice>>,
    input_tx: broadcast::Sender<Report>,
    _reader_handle: tokio::task::JoinHandle<()>,
}

impl HidapiTransport {
    /// Create a transport and spawn the background reader.
    fn spawn(device: hidapi::HidDevice) -> Self {
        let device = Arc::new(Mutex::new(device));
        let (input_tx, _) = broadcast::channel(BROADCAST_CAPACITY);

        let reader_device = Arc::clone(&device);
        let reader_tx = input_tx.clone();
        let handle = tokio::task::spawn_local(Self::reader_loop(reader_device, reader_tx));

        Self {
            device,
            input_tx,
            _reader_handle: handle,
        }
    }

    /// Background loop: read HID input reports and broadcast them.
    async fn reader_loop(
        device: Arc<Mutex<hidapi::HidDevice>>,
        input_tx: broadcast::Sender<Report>,
    ) {
        let mut buf = [0u8; 65]; // report ID + 64 bytes
        loop {
            let result = {
                let dev = device.lock().await;
                dev.read_timeout(&mut buf, READ_TIMEOUT_MS)
            };

            match result {
                Ok(0) => {
                    // No data available, yield briefly.
                    tokio::time::sleep(Duration::from_millis(1)).await;
                }
                Ok(n) if n >= 1 => {
                    if let Some(report) = Report::from_input(&buf[..n]) {
                        // Ignore send errors (no subscribers).
                        let _ = input_tx.send(report);
                    }
                }
                Ok(_) => {}
                Err(e) => {
                    tracing::warn!("HID read error: {e}");
                    tokio::time::sleep(Duration::from_millis(50)).await;
                }
            }
        }
    }
}

#[async_trait::async_trait(?Send)]
impl Transport for HidapiTransport {
    async fn send(&self, report: &Report, kind: HidReportKind) -> Result<(), TransportError> {
        let dev = self.device.lock().await;

        // wire_bytes() returns [report_id, payload[0..len]] — exactly the
        // size macOS IOKit expects (must match the HID descriptor).
        let buf = report.wire_bytes();

        match kind {
            HidReportKind::Output => {
                dev.write(&buf)
                    .map_err(|e| TransportError::Io(e.to_string()))?;
            }
            HidReportKind::Feature => {
                dev.send_feature_report(&buf)
                    .map_err(|e| TransportError::Io(e.to_string()))?;
            }
        }

        Ok(())
    }

    async fn get_feature_report(&self, report_id: u8) -> Result<Report, TransportError> {
        let dev = self.device.lock().await;

        // Buffer: [report_id, ...space for max payload].
        // hidapi fills this in-place and returns the actual size.
        let mut buf = [0u8; 1 + corsair_proto::report::MAX_REPORT_SIZE];
        buf[0] = report_id;

        let n = dev
            .get_feature_report(&mut buf)
            .map_err(|e| TransportError::Io(e.to_string()))?;

        Report::from_input(&buf[..n]).ok_or_else(|| {
            TransportError::Io("failed to parse feature report response".into())
        })
    }

    fn input_reports(&self) -> Pin<Box<dyn Stream<Item = Report> + '_>> {
        let stream = BroadcastStream::new(self.input_tx.subscribe())
            .filter_map(|r: Result<Report, _>| r.ok());
        Box::pin(stream)
    }
}
