//! Transaction-paced transport wrapper.
//!
//! Corsair Legacy protocol requires a 35ms delay between consecutive reports
//! within a transaction. [`PacedTransport`] wraps any [`Transport`] and
//! enforces this minimum inter-report gap.

use std::cell::Cell;
use std::pin::Pin;
use std::time::Duration;

use corsair_proto::Report;

use crate::error::TransportError;
use crate::{HidReportKind, Transport};

/// Default inter-report delay for Legacy headset protocol.
pub const LEGACY_DELAY: Duration = Duration::from_millis(35);

/// Default inter-report delay for Bragi protocol.
pub const BRAGI_DELAY: Duration = Duration::from_millis(20);

/// A transport wrapper that enforces a minimum delay between consecutive sends.
///
/// This does **not** implement [`Transport`] itself — it wraps one and exposes
/// pacing-aware send methods. The inner transport is still accessible for
/// `input_reports()` and `get_feature_report()`.
pub struct PacedTransport<T> {
    inner: T,
    delay: Duration,
    last_send: Cell<Option<std::time::Instant>>,
}

impl<T: Transport> PacedTransport<T> {
    /// Wrap a transport with the given inter-report delay.
    pub fn new(inner: T, delay: Duration) -> Self {
        Self {
            inner,
            delay,
            last_send: Cell::new(None),
        }
    }

    /// Wrap a transport with the Legacy protocol 35ms delay.
    pub fn legacy(inner: T) -> Self {
        Self::new(inner, LEGACY_DELAY)
    }

    /// Wrap a transport with the Bragi protocol 20ms delay.
    pub fn bragi(inner: T) -> Self {
        Self::new(inner, BRAGI_DELAY)
    }

    /// Send a single report, sleeping if necessary to maintain the inter-report gap.
    pub async fn send_paced(
        &self,
        report: &Report,
        kind: HidReportKind,
    ) -> Result<(), TransportError> {
        self.wait_for_gap().await;
        self.inner.send(report, kind).await?;
        self.last_send.set(Some(std::time::Instant::now()));
        Ok(())
    }

    /// Send a batch of reports with inter-report pacing.
    ///
    /// Each report is sent after waiting for the minimum delay since the
    /// previous send.
    pub async fn send_transaction(
        &self,
        reports: &[Report],
        kind: HidReportKind,
    ) -> Result<(), TransportError> {
        for report in reports {
            self.send_paced(report, kind).await?;
        }
        Ok(())
    }

    /// Access the inner transport (e.g. for `input_reports()` or `get_feature_report()`).
    pub fn inner(&self) -> &T {
        &self.inner
    }

    /// Subscribe to input reports from the underlying transport.
    pub fn input_reports(&self) -> Pin<Box<dyn futures_core::Stream<Item = Report> + '_>> {
        self.inner.input_reports()
    }

    /// Read a feature report via the underlying transport.
    pub async fn get_feature_report(&self, report_id: u8) -> Result<Report, TransportError> {
        self.inner.get_feature_report(report_id).await
    }

    /// Sleep until the minimum delay since the last send has elapsed.
    async fn wait_for_gap(&self) {
        if let Some(last) = self.last_send.get() {
            let elapsed = last.elapsed();
            if elapsed < self.delay {
                let remaining = self.delay - elapsed;
                #[cfg(feature = "native")]
                tokio::time::sleep(remaining).await;
                #[cfg(not(feature = "native"))]
                {
                    let _ = remaining; // WASM: use wasm_bindgen_futures or similar
                }
            }
        }
    }
}
