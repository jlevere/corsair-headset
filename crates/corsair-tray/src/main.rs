use std::time::{Duration, Instant};

use muda::{
    Menu, MenuEvent, MenuItem, PredefinedMenuItem, Submenu,
};
use tao::event::Event;
use tao::event_loop::{ControlFlow, EventLoopBuilder};
use tray_icon::TrayIconBuilder;

use corsair_proto::legacy::sidetone;

// ---------------------------------------------------------------------------
// HID helpers (direct hidapi, no async needed for the tray app)
// ---------------------------------------------------------------------------

struct Headset {
    device: hidapi::HidDevice,
}

#[derive(Debug, Clone)]
struct HeadsetState {
    battery: u8,
    battery_state: &'static str,
    mic_boom_up: bool,
    link: &'static str,
    fw_tx: String,
    fw_rx: String,
}

impl Headset {
    fn open() -> anyhow::Result<Self> {
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

    fn poll_state(&self) -> Option<HeadsetState> {
        // Request state
        let _ = self.device.write(&[0xC9, 0x64]);
        let state_report = self.read_report(0x64)?;

        // Request firmware
        let _ = self.device.write(&[0xC9, 0x66]);
        let fw_report = self.read_report(0x66);

        let p = &state_report[1..];
        if p.len() < 4 {
            return None;
        }

        let battery = p[1] & 0x7F;
        let mic_boom_up = (p[1] & 0x80) == 0;
        let link = match p[2] & 0x0F {
            1 => "Active",
            3 => "Searching",
            4 => "Standby",
            _ => "Disconnected",
        };
        let battery_state = match p[3] & 0x07 {
            1 => "Ok",
            2 => "Low",
            3 => "Critical",
            4 => "Full",
            5 => "Charging",
            _ => "Unknown",
        };

        let (fw_tx, fw_rx) = if let Some(fw) = fw_report {
            let f = &fw[1..];
            if f.len() >= 4 {
                (format!("{}.{}", f[0], f[1]), format!("{}.{}", f[2], f[3]))
            } else {
                ("?".into(), "?".into())
            }
        } else {
            ("?".into(), "?".into())
        };

        Some(HeadsetState {
            battery,
            battery_state,
            mic_boom_up,
            link,
            fw_tx,
            fw_rx,
        })
    }

    fn set_sidetone(&self, percent: u8) {
        let report = sidetone::encode_set_sidetone_level(percent);
        let wire = report.wire_bytes();
        let _ = self.device.send_feature_report(&wire);
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

// ---------------------------------------------------------------------------
// Menu IDs
// ---------------------------------------------------------------------------

const SIDETONE_LEVELS: &[(u8, &str)] = &[
    (0, "Off"),
    (25, "Low (25%)"),
    (50, "Medium (50%)"),
    (75, "High (75%)"),
    (100, "Max (100%)"),
];

fn battery_icon(percent: u8) -> &'static str {
    match percent {
        0..=10 => "🪫",
        11..=30 => "🔋",
        31..=60 => "🔋",
        61..=100 => "🔋",
        _ => "🔋",
    }
}

// ---------------------------------------------------------------------------
// Main
// ---------------------------------------------------------------------------

fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter("info")
        .init();

    // Open headset
    let headset = match Headset::open() {
        Ok(h) => h,
        Err(e) => {
            eprintln!("Failed to open headset: {e}");
            eprintln!("Make sure the dongle is plugged in.");
            std::process::exit(1);
        }
    };

    // Initial state
    let state = headset.poll_state();
    let initial_title = if let Some(ref s) = state {
        format!("{} {}%", battery_icon(s.battery), s.battery)
    } else {
        "🎧 --".to_string()
    };

    // Build menu
    let menu = Menu::new();

    let battery_item = MenuItem::new(
        state
            .as_ref()
            .map(|s| format!("Battery: {}% ({})", s.battery, s.battery_state))
            .unwrap_or_else(|| "Battery: --".into()),
        false,
        None,
    );

    let mic_item = MenuItem::new(
        state
            .as_ref()
            .map(|s| {
                format!(
                    "Mic: {}",
                    if s.mic_boom_up { "Muted (up)" } else { "Live (down)" }
                )
            })
            .unwrap_or_else(|| "Mic: --".into()),
        false,
        None,
    );

    let link_item = MenuItem::new(
        state
            .as_ref()
            .map(|s| format!("Link: {}", s.link))
            .unwrap_or_else(|| "Link: --".into()),
        false,
        None,
    );

    let fw_item = MenuItem::new(
        state
            .as_ref()
            .map(|s| format!("Firmware: TX {} / RX {}", s.fw_tx, s.fw_rx))
            .unwrap_or_else(|| "Firmware: --".into()),
        false,
        None,
    );

    let sidetone_sub = Submenu::new("Sidetone", true);
    let mut sidetone_items = Vec::new();
    for &(level, label) in SIDETONE_LEVELS {
        let item = MenuItem::new(label, true, None);
        sidetone_sub.append(&item).unwrap();
        sidetone_items.push((item, level));
    }

    let quit_item = MenuItem::new("Quit", true, None);

    menu.append(&battery_item).unwrap();
    menu.append(&mic_item).unwrap();
    menu.append(&link_item).unwrap();
    menu.append(&fw_item).unwrap();
    menu.append(&PredefinedMenuItem::separator()).unwrap();
    menu.append(&sidetone_sub).unwrap();
    menu.append(&PredefinedMenuItem::separator()).unwrap();
    menu.append(&quit_item).unwrap();

    // Build tray icon (text-only on macOS, no image needed)
    let event_loop = EventLoopBuilder::new().build();

    let _tray = TrayIconBuilder::new()
        .with_menu(Box::new(menu))
        .with_title(&initial_title)
        .with_tooltip("Corsair Headset")
        .build()?;

    // Store item IDs for event handling
    let quit_id = quit_item.id().clone();
    let sidetone_ids: Vec<_> = sidetone_items
        .iter()
        .map(|(item, level)| (item.id().clone(), *level))
        .collect();

    let menu_channel = MenuEvent::receiver();

    let mut last_poll = Instant::now();
    let poll_interval = Duration::from_secs(30);

    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::WaitUntil(Instant::now() + Duration::from_millis(100));

        // Handle menu events
        if let Ok(event) = menu_channel.try_recv() {
            if event.id == quit_id {
                *control_flow = ControlFlow::Exit;
                return;
            }

            for (id, level) in &sidetone_ids {
                if &event.id == id {
                    headset.set_sidetone(*level);
                    tracing::info!("Sidetone set to {level}%");
                }
            }
        }

        // Periodic state polling
        if let Event::NewEvents(_) = event {
            if last_poll.elapsed() >= poll_interval {
                last_poll = Instant::now();
                if let Some(s) = headset.poll_state() {
                    let title = format!("{} {}%", battery_icon(s.battery), s.battery);
                    _tray.set_title(Some(&title));

                    battery_item.set_text(&format!(
                        "Battery: {}% ({})",
                        s.battery, s.battery_state
                    ));
                    mic_item.set_text(&format!(
                        "Mic: {}",
                        if s.mic_boom_up { "Muted (up)" } else { "Live (down)" }
                    ));
                    link_item.set_text(&format!("Link: {}", s.link));
                }
            }
        }
    });
}
