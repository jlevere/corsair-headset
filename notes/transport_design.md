# corsair-transport Design Notes

Analysis of logi-re's transport patterns and iCUE's protocol architecture, informing the
design of the `corsair-transport` crate.

---

## Part 1: logi-re Transport Crate (`hidpp-transport`)

### Transport Trait

```rust
#[async_trait(?Send)]  // !Send — works in both tokio and WASM single-threaded runtimes
pub trait Transport {
    async fn request(&self, report: &LongReport) -> Result<LongReport, TransportError>;
    async fn send(&self, report: &LongReport) -> Result<(), TransportError>;
    fn notifications(&self) -> Pin<Box<dyn Stream<Item = LongReport> + '_>>;
}
```

Three operations:
- `request` — send a report and wait for the matching response (request/response with timeout).
- `send` — fire-and-forget, no response expected.
- `notifications` — returns a stream of unsolicited input reports that didn't match any pending request.

Key design choice: `async_trait(?Send)` (non-Send) so the same trait works in both tokio
(native) and single-threaded WASM contexts without requiring `Send` bounds.

### Enumerator Trait

```rust
#[async_trait(?Send)]
pub trait Enumerator {
    async fn enumerate(&self) -> Result<Vec<DeviceInfo>, TransportError>;
    async fn open(&self, info: &DeviceInfo) -> Result<Box<dyn Transport>, TransportError>;
}
```

Separates discovery from communication. `enumerate()` lists devices, `open()` creates a
transport. Returns `Box<dyn Transport>` for runtime polymorphism.

### DeviceInfo

```rust
pub struct DeviceInfo {
    pub vendor_id: u16,
    pub product_id: u16,
    pub name: Option<String>,
    pub path: String,       // OS-specific device path (hidapi path or WebHID handle)
}
```

Minimal discovery metadata. The `path` field is used by `open()` to reconnect to a
specific device.

### HidapiTransport (Native Backend)

Architecture: **background reader task + channel-based demux**.

```
                    ┌──────────────────┐
                    │  reader_loop()   │  tokio::spawn
                    │  (background)    │
                    │                  │
  HID device ──────│  read_timeout()  │
  (non-blocking)   │       │          │
                    │       ▼          │
                    │  match pending?  │──yes──▶ oneshot::Sender → request() caller
                    │       │          │
                    │       no         │
                    │       │          │
                    │       ▼          │
                    │  broadcast::send │──────▶ notification subscribers
                    └──────────────────┘
```

Components:
- `device: Arc<Mutex<HidDevice>>` — shared HID handle for reads (background) and writes (foreground).
- `pending_tx: mpsc::UnboundedSender<PendingRequest>` — foreground registers pending requests.
- `notification_tx: broadcast::Sender<LongReport>` — broadcasts unmatched reports.
- `_reader_handle: JoinHandle<()>` — keeps the background reader alive.

**Request matching** (HID++ specific): A `PendingRequest` matches when the response has the
same `(feature_index, function_id, sw_id)` triple. Error responses (feature_index == 0xFF)
match by the original feature_index in byte 3.

**Timeout**: 2000ms via `tokio::time::timeout`.

**macOS quirk**: hidapi on macOS strips the report ID from reads. The reader detects 19-byte
reads and prepends `REPORT_ID_LONG`.

### Error Types

```rust
pub enum TransportError {
    DeviceNotFound,
    Io(String),
    Timeout,
    Disconnected,
    Hidpp(hidpp::error::HidppError),
}
```

### Cargo.toml Pattern

Feature-gated backends:
- `native` feature (default) — pulls in `hidapi` + `tokio`.
- Web feature planned but not yet wired (WASM backend is in a separate crate).
- Strict clippy lints: `unwrap_used = "deny"`, `panic = "deny"`, etc.

---

## Part 2: logi-re Web Transport (`hidpp-web`)

### WasmDevice Architecture

```rust
pub struct WasmDevice {
    inner: Rc<RefCell<Inner>>,              // shared mutable state
    _input_callback: Closure<dyn FnMut(JsValue)>,  // prevent GC of callback
}

struct Inner {
    device: webhid::HidDevice,
    pending: Vec<Pending>,
    sw_id: SoftwareId,
    device_index: DeviceIndex,
    features: BTreeMap<FeatureId, FeatureIndex>,
    name: String,
    notification_callback: Option<js_sys::Function>,
}
```

**Why `Rc<RefCell<>>` instead of `Arc<Mutex<>>`**: WASM is single-threaded, `Send`/`Sync`
are not available for JS types. `Rc<RefCell<>>` provides interior mutability without atomic
overhead. The `Closure` must be stored to prevent JS garbage collection.

### Request/Response Matching in WASM

The pattern replaces tokio channels with JS Promises:

1. Create a `js_sys::Promise` with a captured `resolve` function.
2. Register a `Pending { feature_index, function_id, sw_id, resolve }` in `inner.pending`.
3. Send the report via `device.send_report()` (returns a Promise, awaited via `JsFuture`).
4. Await the original Promise via `JsFuture::from(promise).await`.
5. The `inputreport` event callback fires, matches the pending request, and calls `resolve`.

**Critical borrow discipline**: The `inner.borrow()` must be dropped before any `.await`
point, because the `inputreport` callback needs `borrow_mut()` and fires during the await.
The code carefully extracts the send promise before awaiting:

```rust
let send_promise = {
    let inner = self.inner.borrow();
    inner.device.send_report(REPORT_ID_LONG, &data)
};
JsFuture::from(send_promise).await?;  // borrow is dropped, callback can fire
```

### Promise-to-Rust Async Bridge

```rust
let (promise, resolve) = {
    let mut resolve_fn: Option<js_sys::Function> = None;
    let promise = js_sys::Promise::new(&mut |resolve, _reject| {
        resolve_fn = Some(resolve);
    });
    (promise, resolve_fn.unwrap())
};
// ... register pending with resolve ...
// ... send report ...
let result = JsFuture::from(promise).await?;  // blocks until resolve is called
```

The `Pending` struct stores `resolve: js_sys::Function` instead of a `oneshot::Sender`.
When the callback matches, it calls `resolve.call1()` with the report bytes as a
`Uint8Array`.

### WebHID Bindings

Custom `wasm_bindgen` extern definitions for the WebHID API (not yet in `web-sys`):
- `Hid::request_device()` / `Hid::get_devices()` — device picker / auto-reconnect.
- `HidDevice::send_report(report_id, data)` — note: report ID is separate from data.
- `HidDevice::add_event_listener("inputreport", callback)` — input report stream.
- `HidInputReportEvent::report_id()` / `data()` — parse incoming reports.

### Notification Delivery

Unmatched reports are passed to `notification_callback: Option<js_sys::Function>` which
is a JS callback set by the frontend. Reports are packed into a JS object with
`{ featureIndex, featureId, functionId, params }`.

---

## Part 3: How iCUE's Transport Differs from Logitech's

### Two Usage Pages vs One

**Logitech**: Single vendor-specific usage page (0xFF43 or 0xFF00) carries all HID++
traffic. Both input and output reports use the same HID collection.

**Corsair Legacy**: Two separate HID collections on two usage pages:
- **0xFFC5**: Output reports (host -> device). Commands sent via `IOHIDDeviceSetReport`
  with `kIOHIDReportTypeOutput`.
- **0xFF00**: Feature reports. Sent via `IOHIDDeviceSetReport` with `kIOHIDReportTypeFeature`.
  Also used for some get-report operations (`IOHIDDeviceGetReport`).

Input reports (device -> host, report IDs 0x64-0x66) arrive asynchronously via
`IOHIDDeviceRegisterInputReportCallback` and are not tied to either usage page collection
at the transport level -- they are just HID input reports.

### The Splitter/HidChannel Architecture

iCUE uses a `Splitter` that sits on top of a single physical `IOHIDDevice` and creates
multiple logical `HidChannel` instances. Each channel is identified by a tuple of:
`(vid, pid, usage_page, usage, direction, report_size, serial)`.

This is necessary because Corsair devices expose multiple HID collections and the
transport layer must route output reports to the correct collection and report type.

For our purposes this means: when opening a Corsair device, we need to be aware that
writes may target different HID report types (Output vs Feature), and we need two
"channels" conceptually -- even if we only open one `hidapi` device handle.

### Report ID Based Routing vs Feature Index

**Logitech HID++**: All reports share the same report ID (0x11 for long). Demuxing is
done by the `(feature_index, function_id, sw_id)` triple in the report header.

**Corsair Legacy**: Each report type has a **unique report ID** (0x64 state, 0x65 mode,
0x66 firmware, 0xC8 set mode, 0xCA set value, 0xFF extended, etc.). The report ID itself
determines what kind of data it is. There is no feature_index/function_id/sw_id framing.

The Extended report (0xFF) is overloaded -- sidetone, auto-shutdown, pairing, and link
state all use report ID 0xFF but with different sub-command header bytes in the payload.

### Fire-and-Forget Transactions

**Logitech HID++**: Every `request()` sends a report and waits for a matching response.
The sw_id field enables multiplexing multiple outstanding requests.

**Corsair Legacy**: Transactions are **fire-and-forget**. The host sends an output/feature
report (e.g., SetValue 0xCA) and does **not** wait for a response. There is no matching
mechanism. State changes are observed by polling or receiving the next State Report (0x64)
as an input report.

The one exception is `RequestData` (0xC9) + `FirmwareVersion` (0x66): the host sends 0xC9
and eventually receives 0x66 as an input report. But there's no request ID -- you just send
the request and wait for that specific report ID to appear.

### 35ms Inter-Report Delay

Corsair Legacy enforces a **35ms delay between consecutive reports** within a transaction.
This is defined as `TRANSACTION_DELAY_NS = 35_000_000`. When multiple reports are sent
(e.g., `encode_set_sidetone_mute` returns 3 reports), the transport must insert a 35ms
sleep between each one.

**Logitech HID++**: No inter-report delay. Send as fast as you want.

### Bragi Protocol: Property-Based, With Responses

Bragi (newer devices) is more structured:
- Property read/write operations with defined timeouts (20ms standard, 750ms long, 2000ms very long).
- The transport does have request/response semantics.
- Uses `bragi::PhysicalHidDevice` and `bragi::DeviceIoAdapter` for I/O.
- Logical subdevices (headset vs dongle) are multiplexed on the same physical device.

### Summary Table

| Aspect | Logitech HID++ | Corsair Legacy | Corsair Bragi |
|--------|----------------|----------------|---------------|
| Usage pages | 1 (0xFF43) | 2 (0xFFC5 + 0xFF00) | TBD (likely 2) |
| Report IDs | Shared (0x10/0x11) | Unique per type | TBD |
| Demux by | feature_index+fn+sw_id | report ID | property ID |
| Request/response | Yes (sw_id matching) | No (fire-and-forget) | Yes (property r/w) |
| Inter-report delay | None | 35ms | 20ms+ |
| Report size | 7 or 20 bytes | 64 bytes | 64 bytes |

---

## Part 4: Design Recommendation for corsair-transport

### Goals

1. Support both Legacy (fire-and-forget) and Bragi (request/response) protocols.
2. Work with hidapi (native) and WebHID (WASM) backends.
3. Handle the two-usage-page model (output vs feature reports).
4. Support async input report notifications.
5. Enforce 35ms inter-report transaction delay.
6. Stay `?Send` compatible for WASM.

### Core Trait Design

The fundamental difference from logi-re is that Corsair Legacy has no request/response
matching. Instead of a single `Transport::request()` method, we split into lower-level
primitives.

```rust
use corsair_proto::Report;
use std::pin::Pin;

/// Errors from the transport layer.
#[derive(Debug, thiserror::Error)]
pub enum TransportError {
    #[error("device not found")]
    DeviceNotFound,
    #[error("i/o error: {0}")]
    Io(String),
    #[error("timeout waiting for report")]
    Timeout,
    #[error("device disconnected")]
    Disconnected,
}

/// Which HID report mechanism to use for sending.
///
/// Corsair devices use two HID collections:
/// - Output reports on usage page 0xFFC5
/// - Feature reports on usage page 0xFF00
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HidReportKind {
    /// Sent via HID SetReport (Output type). Usage page 0xFFC5.
    Output,
    /// Sent via HID SetReport (Feature type). Usage page 0xFF00.
    Feature,
}

/// Low-level async HID transport for Corsair devices.
///
/// This trait handles raw report I/O. Protocol-level semantics (Legacy vs Bragi)
/// are built on top by the `corsair-device` layer.
#[async_trait::async_trait(?Send)]
pub trait Transport {
    /// Send an output report to the device.
    ///
    /// `kind` determines whether this is sent as an HID output report (0xFFC5)
    /// or a feature report (0xFF00).
    async fn send(&self, report: &Report, kind: HidReportKind) -> Result<(), TransportError>;

    /// Read a feature report from the device (GetReport).
    ///
    /// Used for feature report reads on usage page 0xFF00.
    async fn get_feature_report(&self, report_id: u8) -> Result<Report, TransportError>;

    /// Subscribe to asynchronous input reports from the device.
    ///
    /// Returns a stream of all input reports (state, mode, firmware version, etc.).
    /// The caller (device layer) is responsible for filtering by report ID.
    fn input_reports(&self) -> Pin<Box<dyn futures_core::Stream<Item = Report> + '_>>;
}
```

### Why Not `request()` at the Transport Level?

Corsair Legacy has no request/response pairing. The device layer handles two patterns:

1. **Legacy fire-and-forget**: `send()` then optionally wait on `input_reports()` for a
   specific report ID.
2. **Bragi request/response**: `send()` then wait on `input_reports()` for a matching
   property response.

Both patterns compose from `send()` + `input_reports()`. A convenience `request()` method
lives in the device layer, not the transport trait:

```rust
/// Convenience built on Transport. Lives in corsair-device, not corsair-transport.
pub struct BragiSession<T: Transport> {
    transport: T,
    tx_delay: Duration,
}

impl<T: Transport> BragiSession<T> {
    /// Send a Bragi property request and wait for the matching response.
    pub async fn request(&self, report: &Report) -> Result<Report, TransportError> {
        self.transport.send(report, HidReportKind::Output).await?;
        // Wait on input_reports() stream, filter by property ID, with timeout
        todo!()
    }
}
```

### Transaction Pacing

The 35ms inter-report delay is enforced by a `TransactionSender` wrapper, not in the
trait itself. This keeps the trait simple and lets callers opt in.

```rust
use std::time::Duration;
use tokio::time::Instant;  // or web_sys::window().performance() for WASM

/// Wraps a Transport and enforces minimum inter-report delay.
pub struct PacedTransport<T: Transport> {
    inner: T,
    delay: Duration,
    last_send: RefCell<Option<Instant>>,  // RefCell for ?Send compat
}

impl<T: Transport> PacedTransport<T> {
    pub fn new(inner: T, delay: Duration) -> Self {
        Self {
            inner,
            delay,
            last_send: RefCell::new(None),
        }
    }

    /// Send with pacing. Sleeps if needed to maintain minimum inter-report gap.
    pub async fn send_paced(
        &self,
        report: &Report,
        kind: HidReportKind,
    ) -> Result<(), TransportError> {
        if let Some(last) = *self.last_send.borrow() {
            let elapsed = last.elapsed();
            if elapsed < self.delay {
                // Platform-appropriate sleep
                sleep(self.delay - elapsed).await;
            }
        }
        self.inner.send(report, kind).await?;
        *self.last_send.borrow_mut() = Some(Instant::now());
        Ok(())
    }

    /// Send a batch of reports with inter-report pacing.
    pub async fn send_transaction(
        &self,
        reports: &[(Report, HidReportKind)],
    ) -> Result<(), TransportError> {
        for (report, kind) in reports {
            self.send_paced(report, *kind).await?;
        }
        Ok(())
    }
}
```

### Enumerator

```rust
/// Information about a discovered Corsair HID device.
#[derive(Debug, Clone)]
pub struct CorsairDeviceInfo {
    pub vid: u16,
    pub pid: u16,
    pub name: Option<String>,
    /// OS-specific device path (hidapi path string or WebHID device reference).
    pub path: String,
    /// HID usage page this handle was found on.
    pub usage_page: u16,
    /// HID usage ID.
    pub usage: u16,
}

/// Enumerate and open Corsair HID devices.
#[async_trait::async_trait(?Send)]
pub trait Enumerator {
    /// List connected Corsair devices.
    ///
    /// Returns entries for each HID collection that matches Corsair VID (0x1B1C)
    /// and vendor-specific usage pages (0xFFC5, 0xFF00).
    async fn enumerate(&self) -> Result<Vec<CorsairDeviceInfo>, TransportError>;

    /// Open a transport to a specific device.
    ///
    /// The implementation may need to open multiple HID handles (one per usage page)
    /// and compose them into a single Transport.
    async fn open(&self, info: &CorsairDeviceInfo) -> Result<Box<dyn Transport>, TransportError>;
}
```

### Native Backend (hidapi)

```rust
/// Native hidapi transport for Corsair devices.
///
/// May hold one or two HID device handles depending on the device's
/// HID descriptor layout:
/// - Output handle: for usage page 0xFFC5 output reports
/// - Feature handle: for usage page 0xFF00 feature reports
///
/// Input reports are read from whichever handle provides them (typically
/// the output-page handle).
pub struct HidapiTransport {
    /// Handle for output reports (usage page 0xFFC5).
    output_device: Arc<Mutex<HidDevice>>,
    /// Handle for feature reports (usage page 0xFF00), if separate.
    /// Some devices expose both on the same HID interface.
    feature_device: Option<Arc<Mutex<HidDevice>>>,
    /// Broadcast channel for input reports from the background reader.
    input_tx: broadcast::Sender<Report>,
    /// Background reader task handle.
    _reader_handle: tokio::task::JoinHandle<()>,
}
```

The background reader is simpler than logi-re's because there's no pending-request
matching at the transport level. It just reads and broadcasts:

```rust
impl HidapiTransport {
    async fn reader_loop(
        device: Arc<Mutex<HidDevice>>,
        input_tx: broadcast::Sender<Report>,
    ) {
        let mut buf = [0u8; 65]; // 1 byte report ID + 64 bytes payload
        loop {
            let read_result = {
                let dev = device.lock().await;
                dev.read_timeout(&mut buf, 10)
            };
            match read_result {
                Ok(0) => {
                    tokio::time::sleep(Duration::from_millis(1)).await;
                }
                Ok(n) if n >= 1 => {
                    if let Some(report) = Report::from_input(&buf[..n]) {
                        let _ = input_tx.send(report);
                    }
                }
                Ok(_) => {}
                Err(_) => {
                    tokio::time::sleep(Duration::from_millis(50)).await;
                }
            }
        }
    }
}
```

### WASM Backend (WebHID)

Following the same `Rc<RefCell<>>` pattern as logi-re:

```rust
/// WebHID transport for Corsair devices.
pub struct WebHidTransport {
    inner: Rc<RefCell<WebHidInner>>,
    _input_callback: Closure<dyn FnMut(JsValue)>,
}

struct WebHidInner {
    device: webhid::HidDevice,
    /// All received input reports, pushed by the inputreport callback.
    input_queue: VecDeque<Report>,
    /// Wakers for input_reports() stream consumers.
    input_waker: Option<Waker>,
}
```

The `send()` implementation:

```rust
#[async_trait(?Send)]
impl Transport for WebHidTransport {
    async fn send(&self, report: &Report, kind: HidReportKind) -> Result<(), TransportError> {
        let data = js_sys::Uint8Array::from(report.as_bytes());
        let promise = {
            let inner = self.inner.borrow();
            match kind {
                HidReportKind::Output => {
                    inner.device.send_report(report.id(), &data)
                }
                HidReportKind::Feature => {
                    inner.device.send_feature_report(report.id(), &data)
                }
            }
        };
        // Borrow dropped before await -- critical for callback reentrancy.
        JsFuture::from(promise).await
            .map_err(|e| TransportError::Io(format!("{e:?}")))?;
        Ok(())
    }
}
```

### Module Layout

```
crates/corsair-transport/
  Cargo.toml
  src/
    lib.rs          # Transport trait, HidReportKind, TransportError, CorsairDeviceInfo, Enumerator
    paced.rs        # PacedTransport wrapper
    native.rs       # HidapiTransport + HidapiEnumerator (cfg feature = "native")
    webhid.rs       # WebHID wasm_bindgen bindings (cfg feature = "web")
    web.rs          # WebHidTransport + WebHidEnumerator (cfg feature = "web")
```

### Cargo.toml

```toml
[package]
name = "corsair-transport"
description = "Platform-agnostic async transport trait + backends for Corsair HID devices"
version.workspace = true
edition.workspace = true
license.workspace = true

[features]
default = ["native"]
native = ["dep:hidapi", "dep:tokio"]
web = ["dep:wasm-bindgen", "dep:wasm-bindgen-futures", "dep:web-sys", "dep:js-sys"]

[dependencies]
corsair-proto = { workspace = true }
thiserror = { workspace = true }
async-trait = { workspace = true }
futures-core = { workspace = true }
tracing = { workspace = true }

# Native
hidapi = { workspace = true, optional = true }
tokio = { workspace = true, optional = true }

# Web
wasm-bindgen = { workspace = true, optional = true }
wasm-bindgen-futures = { workspace = true, optional = true }
web-sys = { workspace = true, optional = true }
js-sys = { workspace = true, optional = true }
```

### How Protocol Layers Compose on Top

```
┌─────────────────────────────────────────────────────┐
│  corsair-device                                     │
│                                                     │
│  LegacyDevice<T: Transport>                         │
│    - PacedTransport<T> (35ms delay)                 │
│    - Subscribes to input_reports(), filters by ID   │
│    - set_sidetone(), get_battery(), etc.            │
│                                                     │
│  BragiDevice<T: Transport>                          │
│    - PacedTransport<T> (20ms delay)                 │
│    - request/response matching on property ID       │
│    - read_property(), write_property(), etc.        │
│                                                     │
├─────────────────────────────────────────────────────┤
│  corsair-transport                                  │
│                                                     │
│  trait Transport { send, get_feature_report,        │
│                    input_reports }                   │
│  PacedTransport<T>                                  │
│                                                     │
│  HidapiTransport (native)                           │
│  WebHidTransport (wasm)                             │
├─────────────────────────────────────────────────────┤
│  corsair-proto                                      │
│                                                     │
│  Report, ReportId, encode/decode functions           │
│  DeviceInfo catalog, ProtocolFamily                 │
└─────────────────────────────────────────────────────┘
```

### Key Differences from logi-re Transport

| Decision | logi-re | corsair-transport | Why |
|----------|---------|-------------------|-----|
| `request()` in trait | Yes | No | Legacy has no response matching |
| Pending request matching | In transport | In device layer | Protocol-specific |
| Report type dispatch | Not needed (one usage page) | `HidReportKind` param | Two usage pages |
| Inter-report pacing | Not needed | `PacedTransport` wrapper | 35ms requirement |
| Report type | `LongReport` (20 bytes) | `Report` (64 bytes) | Corsair uses larger reports |
| Background reader | Demuxes responses + notifications | Broadcasts all input reports | Simpler; demux happens above |
| Error types | Includes `Hidpp` variant | No protocol error variant | Protocol errors in corsair-proto |
