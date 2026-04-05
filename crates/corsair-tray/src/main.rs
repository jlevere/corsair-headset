use std::time::{Duration, Instant};

use muda::{CheckMenuItem, Menu, MenuEvent, MenuItem, PredefinedMenuItem, Submenu};
use tao::event::Event;
use tao::event_loop::{ControlFlow, EventLoopBuilder};
use tray_icon::TrayIconBuilder;

mod headset;
mod icon;
mod notify;

use headset::{Headset, LinkInfo};

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

/// Poll interval when headset is active and connected.
const POLL_INTERVAL_ACTIVE: Duration = Duration::from_secs(30);

/// Poll interval when headset is disconnected, standby, or searching.
/// Much slower to avoid unnecessary USB traffic.
const POLL_INTERVAL_IDLE: Duration = Duration::from_secs(120);

const SIDETONE_LEVELS: &[(u8, &str)] = &[
    (0, "Off"),
    (25, "25%"),
    (50, "50%"),
    (75, "75%"),
    (100, "100%"),
];

const EQ_PRESETS: &[(u8, &str)] = &[
    (0, "Pure Direct"),
    (1, "Bass Boost"),
    (2, "Clear Chat"),
    (3, "FPS Competition"),
    (4, "Movie Theater"),
];

/// Auto-sleep timeout options (minutes). 0 = disabled.
const SLEEP_TIMEOUTS: &[(u16, &str)] = &[
    (0, "Never"),
    (5, "5 minutes"),
    (15, "15 minutes"),
    (30, "30 minutes"),
    (60, "1 hour"),
];

// ---------------------------------------------------------------------------
// Main
// ---------------------------------------------------------------------------

fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter("info")
        .init();

    let headset = match Headset::open() {
        Ok(h) => h,
        Err(e) => {
            eprintln!("Could not open headset: {e}");
            eprintln!("Make sure the USB dongle is plugged in.");
            std::process::exit(1);
        }
    };

    let state = headset.poll_state();

    // --- Build menu ---

    let menu = Menu::new();

    // Status section (read-only info items, disabled)
    let battery_item = MenuItem::new(format_battery(&state), false, None);
    let mic_item = MenuItem::new(format_mic(&state), false, None);
    let link_item = MenuItem::new(format_link(&state), false, None);
    let fw_item = MenuItem::new(format_firmware(&state), false, None);

    menu.append(&battery_item).unwrap();
    menu.append(&mic_item).unwrap();
    menu.append(&link_item).unwrap();
    menu.append(&fw_item).unwrap();
    menu.append(&PredefinedMenuItem::separator()).unwrap();

    // Sidetone submenu with checkmarks
    let sidetone_sub = Submenu::new("Sidetone", true);
    let mut sidetone_items = Vec::new();
    for &(level, label) in SIDETONE_LEVELS {
        let item = CheckMenuItem::new(label, true, level == 0, None);
        sidetone_sub.append(&item).unwrap();
        sidetone_items.push((item, level));
    }
    menu.append(&sidetone_sub).unwrap();

    // EQ preset submenu with checkmarks
    let eq_sub = Submenu::new("EQ Preset", true);
    let mut eq_items = Vec::new();
    for &(idx, label) in EQ_PRESETS {
        let item = CheckMenuItem::new(label, true, idx == 0, None);
        eq_sub.append(&item).unwrap();
        eq_items.push((item, idx));
    }
    menu.append(&eq_sub).unwrap();

    menu.append(&PredefinedMenuItem::separator()).unwrap();

    // Mic mute toggle
    let mic_mute_item = CheckMenuItem::new(
        "Mic Mute",
        true,
        state.as_ref().is_some_and(|s| s.mic_boom_up),
        None,
    );
    menu.append(&mic_mute_item).unwrap();

    menu.append(&PredefinedMenuItem::separator()).unwrap();

    // Auto-sleep submenu
    let sleep_sub = Submenu::new("Auto Sleep", true);
    let mut sleep_items = Vec::new();
    for &(mins, label) in SLEEP_TIMEOUTS {
        let item = CheckMenuItem::new(label, true, mins == 15, None);
        sleep_sub.append(&item).unwrap();
        sleep_items.push((item, mins));
    }
    menu.append(&sleep_sub).unwrap();

    menu.append(&PredefinedMenuItem::separator()).unwrap();

    let quit_item = MenuItem::new("Quit", true, None);
    menu.append(&quit_item).unwrap();

    // --- Build tray with dual icons ---

    let event_loop = EventLoopBuilder::new().build();
    let icon_connected = icon::solid_icon()?;
    let icon_disconnected = icon::outline_icon()?;

    let is_connected = state
        .as_ref()
        .is_some_and(|s| s.link == LinkInfo::Active);

    let initial_icon = if is_connected {
        &icon_connected
    } else {
        &icon_disconnected
    };

    let initial_title = state
        .as_ref()
        .map(|s| match s.link {
            LinkInfo::Active => format!("{}%", s.battery),
            _ => s.link.label().into(),
        })
        .unwrap_or_else(|| "--".into());

    let _tray = TrayIconBuilder::new()
        .with_icon(initial_icon.clone())
        .with_icon_as_template(true)
        .with_menu(Box::new(menu))
        .with_title(&initial_title)
        .with_tooltip("Corsair Headset")
        .build()?;

    // --- Event IDs ---

    let quit_id = quit_item.id().clone();
    let mic_mute_id = mic_mute_item.id().clone();

    let sidetone_ids: Vec<_> = sidetone_items
        .iter()
        .map(|(item, level)| (item.id().clone(), *level))
        .collect();

    let eq_ids: Vec<_> = eq_items
        .iter()
        .map(|(item, idx)| (item.id().clone(), *idx))
        .collect();

    let sleep_ids: Vec<_> = sleep_items
        .iter()
        .map(|(item, mins)| (item.id().clone(), *mins))
        .collect();

    let menu_channel = MenuEvent::receiver();

    // --- State tracking ---

    let mut last_poll = Instant::now();
    let mut poll_interval = POLL_INTERVAL_ACTIVE;
    let mut was_connected = is_connected;
    let mut notifier = notify::BatteryNotifier::new();
    let mut current_sidetone: u8 = 0;
    let mut current_eq: u8 = 0;
    let mut current_sleep: u16 = 15;
    let mut mic_muted = false;

    // --- Event loop ---

    #[allow(unused_assignments)] // state vars track across loop iterations
    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::WaitUntil(Instant::now() + Duration::from_millis(100));

        // Handle menu events
        if let Ok(event) = menu_channel.try_recv() {
            // Quit
            if event.id == quit_id {
                *control_flow = ControlFlow::Exit;
                return;
            }

            // Mic mute toggle
            if event.id == mic_mute_id {
                mic_muted = !mic_muted;
                mic_mute_item.set_checked(mic_muted);
                headset.set_mic_mute(mic_muted);
                tracing::info!("Mic mute: {mic_muted}");
            }

            // Sidetone selection
            for (id, level) in &sidetone_ids {
                if event.id == *id {
                    current_sidetone = *level;
                    headset.set_sidetone(*level);
                    // Update checkmarks
                    for (item, l) in &sidetone_items {
                        item.set_checked(*l == current_sidetone);
                    }
                    tracing::info!("Sidetone: {level}%");
                }
            }

            // EQ preset selection
            for (id, idx) in &eq_ids {
                if event.id == *id {
                    current_eq = *idx;
                    headset.set_eq_preset(*idx);
                    for (item, i) in &eq_items {
                        item.set_checked(*i == current_eq);
                    }
                    tracing::info!("EQ preset: {}", EQ_PRESETS[*idx as usize].1);
                }
            }

            // Auto-sleep timeout selection
            for (id, mins) in &sleep_ids {
                if event.id == *id {
                    current_sleep = *mins;
                    headset.set_auto_shutdown(*mins);
                    for (item, m) in &sleep_items {
                        item.set_checked(*m == current_sleep);
                    }
                    tracing::info!("Auto sleep: {mins} minutes");
                }
            }
        }

        // Periodic state refresh
        if let Event::NewEvents(_) = event {
            if last_poll.elapsed() >= poll_interval {
                last_poll = Instant::now();

                if let Some(s) = headset.poll_state() {
                    let connected = s.link == LinkInfo::Active;

                    // Swap icon when connection state changes
                    if connected != was_connected {
                        let new_icon = if connected {
                            &icon_connected
                        } else {
                            &icon_disconnected
                        };
                        let _ = _tray.set_icon(Some(new_icon.clone()));
                        was_connected = connected;
                    }

                    // Update menu bar title
                    let title = match s.link {
                        LinkInfo::Active => format!("{}%", s.battery),
                        _ => s.link.label().to_string(),
                    };
                    _tray.set_title(Some(&title));

                    // Update menu items
                    battery_item.set_text(&format_battery(&Some(s.clone())));
                    mic_item.set_text(&format_mic(&Some(s.clone())));
                    link_item.set_text(&format_link(&Some(s.clone())));

                    // Sync mic mute state from hardware
                    if s.mic_boom_up != mic_muted {
                        mic_muted = s.mic_boom_up;
                        mic_mute_item.set_checked(mic_muted);
                    }

                    // Battery notifications
                    notifier.check(s.battery);

                    // Back off polling when not actively connected
                    poll_interval = match s.link {
                        LinkInfo::Active => POLL_INTERVAL_ACTIVE,
                        _ => POLL_INTERVAL_IDLE,
                    };
                } else {
                    _tray.set_title(Some("--"));
                    battery_item.set_text("Battery: --");
                    link_item.set_text("Link: Disconnected");
                    poll_interval = POLL_INTERVAL_IDLE;
                }
            }
        }
    });
}

// ---------------------------------------------------------------------------
// Formatting helpers
// ---------------------------------------------------------------------------

fn format_battery(state: &Option<headset::HeadsetState>) -> String {
    state
        .as_ref()
        .map(|s| format!("Battery: {}% ({})", s.battery, s.battery_state.label()))
        .unwrap_or_else(|| "Battery: --".into())
}

fn format_mic(state: &Option<headset::HeadsetState>) -> String {
    state
        .as_ref()
        .map(|s| {
            if s.mic_boom_up {
                "Mic: Muted (boom up)".into()
            } else {
                "Mic: Live (boom down)".into()
            }
        })
        .unwrap_or_else(|| "Mic: --".into())
}

fn format_link(state: &Option<headset::HeadsetState>) -> String {
    state
        .as_ref()
        .map(|s| format!("Link: {}", s.link.label()))
        .unwrap_or_else(|| "Link: --".into())
}

fn format_firmware(state: &Option<headset::HeadsetState>) -> String {
    state
        .as_ref()
        .map(|s| format!("Firmware: TX {} / RX {}", s.fw_tx, s.fw_rx))
        .unwrap_or_else(|| "Firmware: --".into())
}
