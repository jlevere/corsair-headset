//! Headset HID communication with automatic reconnection.

use std::time::{Duration, Instant};

use corsair_proto::legacy::lighting::{self, LedZone};
use corsair_proto::legacy::types::{ReportId, ValueId};
use corsair_proto::Report;

/// Headset handle with automatic reconnection on device loss.
pub struct Headset {
    device: Option<hidapi::HidDevice>,
    last_reconnect_attempt: Instant,
}

/// Snapshot of headset state.
#[derive(Debug, Clone)]
pub struct HeadsetState {
    pub battery: u8,
    pub battery_state: BatteryInfo,
    pub mic_boom_up: bool,
    pub link: LinkInfo,
    pub fw_tx: String,
    pub fw_rx: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BatteryInfo {
    Ok,
    Low,
    Critical,
    Full,
    Charging,
    Unknown,
}

impl BatteryInfo {
    fn from_byte(b: u8) -> Self {
        match b & 0x07 {
            1 => Self::Ok,
            2 => Self::Low,
            3 => Self::Critical,
            4 => Self::Full,
            5 => Self::Charging,
            _ => Self::Unknown,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Ok => "Ok",
            Self::Low => "Low",
            Self::Critical => "Critical",
            Self::Full => "Full",
            Self::Charging => "Charging",
            Self::Unknown => "Unknown",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LinkInfo {
    Active,
    Searching,
    Standby,
    Disconnected,
}

impl LinkInfo {
    fn from_byte(b: u8) -> Self {
        match b & 0x0F {
            1 => Self::Active,
            3 | 5 | 6 | 9 => Self::Searching,
            4 => Self::Standby,
            _ => Self::Disconnected,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Active => "Active",
            Self::Searching => "Searching",
            Self::Standby => "Standby",
            Self::Disconnected => "Disconnected",
        }
    }
}

/// Minimum time between reconnection attempts.
const RECONNECT_COOLDOWN: Duration = Duration::from_secs(5);

impl Headset {
    /// Try to open the headset. Returns a handle even if the device isn't
    /// found — it will reconnect automatically on the next operation.
    pub fn new() -> Self {
        let device = Self::try_open();
        if device.is_some() {
            tracing::info!("Headset connected");
        } else {
            tracing::warn!("Headset not found, will retry on next poll");
        }
        Self {
            device,
            last_reconnect_attempt: Instant::now(),
        }
    }

    /// Whether the HID device is currently open.
    #[allow(dead_code)]
    pub fn is_connected(&self) -> bool {
        self.device.is_some()
    }

    /// Poll the headset for its current state. Returns `None` if the device
    /// is disconnected (will attempt reconnection automatically).
    pub fn poll_state(&mut self) -> Option<HeadsetState> {
        self.ensure_connected();
        let device = self.device.as_ref()?;

        // Request state
        if device.write(&[ReportId::RequestData as u8, ReportId::State as u8]).is_err() {
            self.handle_disconnect();
            return None;
        }
        let state_data = match Self::read_report_from(device, ReportId::State as u8) {
            Some(d) => d,
            None => {
                self.handle_disconnect();
                return None;
            }
        };

        // Request firmware (non-fatal if it fails)
        let _ = device.write(&[ReportId::RequestData as u8, ReportId::FirmwareVersion as u8]);
        let fw_data = Self::read_report_from(device, ReportId::FirmwareVersion as u8);

        let p = &state_data[1..];
        if p.len() < 4 {
            return None;
        }

        let battery = p[1] & 0x7F;
        let mic_boom_up = (p[1] & 0x80) == 0;
        let link = LinkInfo::from_byte(p[2]);
        let battery_state = BatteryInfo::from_byte(p[3]);

        let (fw_tx, fw_rx) = fw_data
            .filter(|f| f.len() >= 5)
            .map(|f| (format!("{}.{}", f[1], f[2]), format!("{}.{}", f[3], f[4])))
            .unwrap_or_else(|| ("?".into(), "?".into()));

        Some(HeadsetState {
            battery,
            battery_state,
            mic_boom_up,
            link,
            fw_tx,
            fw_rx,
        })
    }

    /// Set sidetone on/off.
    ///
    /// The VOID Elite has hardware sidetone at a fixed volume level.
    /// Volume control requires CorsairAudioConfigService (not installed).
    /// We can only toggle it on/off via HID SetValue.
    pub fn set_sidetone(&mut self, enabled: bool) {
        // ValueId 5 = SidetoneState: 0 = on, 1 = muted (inverted!)
        self.send_set_value(ValueId::SidetoneState, u8::from(!enabled));
    }

    /// Set EQ preset index (0–4).
    pub fn set_eq_preset(&mut self, index: u8) {
        self.send_set_value(ValueId::EqIndex, index);
    }


    /// Set LED color on both logo zones.
    pub fn set_led_color(&mut self, r: u8, g: u8, b: u8) {
        if let Some(device) = &self.device {
            for zone in [LedZone::LeftLogo, LedZone::RightLogo] {
                let report = lighting::encode_set_color(zone, r, g, b);
                if device.write(&report.wire_bytes()).is_err() {
                    self.handle_disconnect();
                    return;
                }
            }
        }
    }

    /// Turn off all LEDs.
    pub fn set_led_off(&mut self) {
        if let Some(device) = &self.device {
            for report in &lighting::encode_clear_pwm() {
                if device.write(&report.wire_bytes()).is_err() {
                    self.handle_disconnect();
                    return;
                }
            }
        }
    }

    /// Power off the headset immediately.
    pub fn shutdown(&mut self) {
        if let Some(device) = &self.device {
            let report = corsair_proto::legacy::power::encode_shutdown();
            if device.write(&report.wire_bytes()).is_err() {
                self.handle_disconnect();
            }
        }
    }

    /// Trigger auto-shutdown (beep + power down).
    pub fn trigger_shutdown(&mut self) {
        if let Some(device) = &self.device {
            let report = corsair_proto::legacy::power::encode_auto_shutdown_trigger();
            if device.send_feature_report(&report.wire_bytes()).is_err() {
                self.handle_disconnect();
            }
        }
    }

    // --- Internal ---

    fn send_set_value(&mut self, id: ValueId, value: u8) {
        if let Some(device) = &self.device {
            let report = Report::with_payload(ReportId::SetValue as u8, &[id as u8, value]);
            if let Some(r) = report
                && device.write(&r.wire_bytes()).is_err()
            {
                self.handle_disconnect();
            }
        }
    }

    fn try_open() -> Option<hidapi::HidDevice> {
        let api = hidapi::HidApi::new().ok()?;
        #[cfg(target_os = "macos")]
        api.set_open_exclusive(true);

        let iface = api
            .device_list()
            .find(|d| d.vendor_id() == 0x1B1C && d.usage_page() == 0xFFC5)?;

        let device = api.open_path(iface.path()).ok()?;
        device.set_blocking_mode(false).ok()?;
        Some(device)
    }

    fn ensure_connected(&mut self) {
        if self.device.is_some() {
            return;
        }
        if self.last_reconnect_attempt.elapsed() < RECONNECT_COOLDOWN {
            return;
        }
        self.last_reconnect_attempt = Instant::now();
        self.device = Self::try_open();
        if self.device.is_some() {
            tracing::info!("Headset reconnected");
        }
    }

    fn handle_disconnect(&mut self) {
        if self.device.is_some() {
            tracing::warn!("Headset disconnected");
            self.device = None;
        }
    }

    fn read_report_from(device: &hidapi::HidDevice, expected_id: u8) -> Option<Vec<u8>> {
        let start = Instant::now();
        let mut buf = [0u8; 65];
        while start.elapsed() < Duration::from_millis(500) {
            match device.read_timeout(&mut buf, 100) {
                Ok(n) if n >= 1 && buf[0] == expected_id => {
                    return Some(buf[..n].to_vec());
                }
                Ok(_) => {}
                Err(_) => return None,
            }
        }
        None
    }
}
