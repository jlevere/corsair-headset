//! Headset HID communication (synchronous, for the tray app).

use std::time::{Duration, Instant};

use corsair_proto::legacy::sidetone;
use corsair_proto::legacy::types::{ReportId, ValueId};
use corsair_proto::Report;

/// Synchronous headset handle for the tray app.
pub struct Headset {
    device: hidapi::HidDevice,
}

/// Snapshot of the headset's current state.
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

impl Headset {
    /// Open the first Corsair headset found on the 0xFFC5 usage page.
    pub fn open() -> anyhow::Result<Self> {
        let api = hidapi::HidApi::new()?;
        #[cfg(target_os = "macos")]
        api.set_open_exclusive(true);

        let iface = api
            .device_list()
            .find(|d| d.vendor_id() == 0x1B1C && d.usage_page() == 0xFFC5)
            .ok_or_else(|| anyhow::anyhow!("No Corsair headset found"))?;

        let path = iface.path().to_owned();
        let device = api.open_path(&path)?;
        device.set_blocking_mode(false)?;
        Ok(Self { device })
    }

    /// Poll the headset for its current state.
    pub fn poll_state(&self) -> Option<HeadsetState> {
        // Request state
        self.send_request_data(ReportId::State);
        let state_data = self.read_report(ReportId::State as u8)?;

        // Request firmware
        self.send_request_data(ReportId::FirmwareVersion);
        let fw_data = self.read_report(ReportId::FirmwareVersion as u8);

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

    /// Set sidetone level (0–100%).
    pub fn set_sidetone(&self, percent: u8) {
        let report = sidetone::encode_set_sidetone_level(percent);
        let _ = self.device.send_feature_report(&report.wire_bytes());
    }

    /// Set EQ preset index (0–4).
    pub fn set_eq_preset(&self, index: u8) {
        let report = Report::with_payload(
            ReportId::SetValue as u8,
            &[ValueId::EqIndex as u8, index],
        );
        if let Some(r) = report {
            let _ = self.device.write(&r.wire_bytes());
        }
    }

    /// Set auto-shutdown timeout (0 = disabled, else minutes).
    pub fn set_auto_shutdown(&self, minutes: u16) {
        let report = corsair_proto::legacy::power::encode_set_auto_shutdown(minutes);
        let _ = self.device.send_feature_report(&report.wire_bytes());
    }

    /// Toggle mic mute via SetValue.
    pub fn set_mic_mute(&self, muted: bool) {
        let report = Report::with_payload(
            ReportId::SetValue as u8,
            &[ValueId::MicState as u8, u8::from(muted)],
        );
        if let Some(r) = report {
            let _ = self.device.write(&r.wire_bytes());
        }
    }

    fn send_request_data(&self, id: ReportId) {
        let _ = self.device.write(&[ReportId::RequestData as u8, id as u8]);
    }

    fn read_report(&self, expected_id: u8) -> Option<Vec<u8>> {
        let start = Instant::now();
        let mut buf = [0u8; 65];
        while start.elapsed() < Duration::from_millis(500) {
            match self.device.read_timeout(&mut buf, 100) {
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
