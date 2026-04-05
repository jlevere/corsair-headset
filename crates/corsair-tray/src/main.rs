use std::time::{Duration, Instant};

use muda::{CheckMenuItem, Menu, MenuEvent, MenuItem, PredefinedMenuItem, Submenu};
use tao::event::Event;
use tao::event_loop::{ControlFlow, EventLoopBuilder};
use tray_icon::{TrayIconBuilder, TrayIconEvent};

mod headset;
mod icon;
mod notify;

use headset::{Headset, LinkInfo};

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

/// Poll interval when headset is active.
const POLL_ACTIVE: Duration = Duration::from_secs(30);

/// Poll interval when headset is disconnected/standby.
const POLL_IDLE: Duration = Duration::from_secs(120);

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

/// Host-side inactivity timeouts (we track this, the headset doesn't).
const SLEEP_TIMEOUTS: &[(u64, &str)] = &[
    (0, "Never"),
    (15, "15 min"),
    (30, "30 min"),
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

    // Status section (read-only)
    let battery_item = MenuItem::new(format_battery(&state), false, None);
    let mic_item = MenuItem::new(format_mic(&state), false, None);
    let link_item = MenuItem::new(format_link(&state), false, None);
    let fw_item = MenuItem::new(format_firmware(&state), false, None);

    menu.append(&battery_item).unwrap();
    menu.append(&mic_item).unwrap();
    menu.append(&link_item).unwrap();
    menu.append(&fw_item).unwrap();
    menu.append(&PredefinedMenuItem::separator()).unwrap();

    // Sidetone
    let sidetone_sub = Submenu::new("Sidetone", true);
    let mut sidetone_items = Vec::new();
    for &(level, label) in SIDETONE_LEVELS {
        let item = CheckMenuItem::new(label, true, level == 0, None);
        sidetone_sub.append(&item).unwrap();
        sidetone_items.push((item, level));
    }
    menu.append(&sidetone_sub).unwrap();

    // EQ
    let eq_sub = Submenu::new("EQ Preset", true);
    let mut eq_items = Vec::new();
    for &(idx, label) in EQ_PRESETS {
        let item = CheckMenuItem::new(label, true, idx == 0, None);
        eq_sub.append(&item).unwrap();
        eq_items.push((item, idx));
    }
    menu.append(&eq_sub).unwrap();

    menu.append(&PredefinedMenuItem::separator()).unwrap();

    // Mic mute
    let mic_mute_item = CheckMenuItem::new(
        "Mic Mute",
        true,
        state.as_ref().is_some_and(|s| s.mic_boom_up),
        None,
    );
    menu.append(&mic_mute_item).unwrap();

    // Auto-sleep (host-controlled inactivity timer)
    let sleep_sub = Submenu::new("Auto Sleep", true);
    let mut sleep_items = Vec::new();
    for &(mins, label) in SLEEP_TIMEOUTS {
        let item = CheckMenuItem::new(label, true, mins == 0, None);
        sleep_sub.append(&item).unwrap();
        sleep_items.push((item, mins));
    }
    menu.append(&sleep_sub).unwrap();

    menu.append(&PredefinedMenuItem::separator()).unwrap();

    // Sleep now
    let sleep_now_item = MenuItem::new("Sleep Now", true, None);
    menu.append(&sleep_now_item).unwrap();

    menu.append(&PredefinedMenuItem::separator()).unwrap();

    let quit_item = MenuItem::new("Quit", true, None);
    menu.append(&quit_item).unwrap();

    // --- Build tray ---

    let event_loop = EventLoopBuilder::new().build();
    let icon_connected = icon::solid_icon()?;
    let icon_disconnected = icon::outline_icon()?;

    let is_connected = state
        .as_ref()
        .is_some_and(|s| s.link == LinkInfo::Active);

    let initial_title = state
        .as_ref()
        .map(|s| match s.link {
            LinkInfo::Active => format!("{}%", s.battery),
            _ => s.link.label().into(),
        })
        .unwrap_or_else(|| "--".into());

    let _tray = TrayIconBuilder::new()
        .with_icon(if is_connected {
            icon_connected.clone()
        } else {
            icon_disconnected.clone()
        })
        .with_icon_as_template(true)
        .with_menu(Box::new(menu))
        .with_title(&initial_title)
        .with_tooltip("Corsair Headset")
        .build()?;

    // --- Event IDs ---

    let quit_id = quit_item.id().clone();
    let mic_mute_id = mic_mute_item.id().clone();
    let sleep_now_id = sleep_now_item.id().clone();

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
    let tray_channel = TrayIconEvent::receiver();

    // --- State ---

    let mut last_poll = Instant::now();
    let mut poll_interval = POLL_ACTIVE;
    let mut was_connected = is_connected;
    let mut notifier = notify::BatteryNotifier::new();
    let mut current_sidetone: u8 = 0;
    let mut current_eq: u8 = 0;
    let mut sleep_timeout_mins: u64 = 0; // 0 = disabled
    let mut last_active = Instant::now();
    let mut mic_muted = false;

    // --- Refresh helper ---
    let refresh = |headset: &Headset,
                   tray: &tray_icon::TrayIcon,
                   battery_item: &MenuItem,
                   mic_item: &MenuItem,
                   link_item: &MenuItem,
                   icon_connected: &tray_icon::Icon,
                   icon_disconnected: &tray_icon::Icon,
                   was_connected: &mut bool,
                   notifier: &mut notify::BatteryNotifier,
                   mic_muted: &mut bool,
                   mic_mute_item: &CheckMenuItem|
     -> Option<Duration> {
        if let Some(s) = headset.poll_state() {
            let connected = s.link == LinkInfo::Active;

            if connected != *was_connected {
                let new_icon = if connected {
                    icon_connected
                } else {
                    icon_disconnected
                };
                let _ = tray.set_icon(Some(new_icon.clone()));
                *was_connected = connected;
            }

            let title = match s.link {
                LinkInfo::Active => format!("{}%", s.battery),
                _ => s.link.label().to_string(),
            };
            tray.set_title(Some(&title));

            battery_item.set_text(&format_battery(&Some(s.clone())));
            mic_item.set_text(&format_mic(&Some(s.clone())));
            link_item.set_text(&format_link(&Some(s.clone())));

            if s.mic_boom_up != *mic_muted {
                *mic_muted = s.mic_boom_up;
                mic_mute_item.set_checked(*mic_muted);
            }

            notifier.check(s.battery);

            Some(if connected { POLL_ACTIVE } else { POLL_IDLE })
        } else {
            tray.set_title(Some("--"));
            battery_item.set_text("Battery: --");
            link_item.set_text("Link: Disconnected");
            if *was_connected {
                let _ = tray.set_icon(Some(icon_disconnected.clone()));
                *was_connected = false;
            }
            Some(POLL_IDLE)
        }
    };

    // --- Event loop ---

    #[allow(unused_assignments)]
    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::WaitUntil(Instant::now() + Duration::from_millis(100));

        // Poll on tray icon click — instant refresh when user opens the menu
        if let Ok(TrayIconEvent::Click { .. }) = tray_channel.try_recv() {
            if let Some(interval) = refresh(
                &headset,
                &_tray,
                &battery_item,
                &mic_item,
                &link_item,
                &icon_connected,
                &icon_disconnected,
                &mut was_connected,
                &mut notifier,
                &mut mic_muted,
                &mic_mute_item,
            ) {
                poll_interval = interval;
            }
            last_poll = Instant::now();
        }

        // Handle menu events
        if let Ok(event) = menu_channel.try_recv() {
            if event.id == quit_id {
                *control_flow = ControlFlow::Exit;
                return;
            }

            if event.id == sleep_now_id {
                headset.trigger_shutdown();
                tracing::info!("Sent shutdown trigger");
            }

            if event.id == mic_mute_id {
                mic_muted = !mic_muted;
                mic_mute_item.set_checked(mic_muted);
                headset.set_mic_mute(mic_muted);
            }

            for (id, level) in &sidetone_ids {
                if event.id == *id {
                    current_sidetone = *level;
                    headset.set_sidetone(*level);
                    for (item, l) in &sidetone_items {
                        item.set_checked(*l == current_sidetone);
                    }
                }
            }

            for (id, idx) in &eq_ids {
                if event.id == *id {
                    current_eq = *idx;
                    headset.set_eq_preset(*idx);
                    for (item, i) in &eq_items {
                        item.set_checked(*i == current_eq);
                    }
                }
            }

            for (id, mins) in &sleep_ids {
                if event.id == *id {
                    sleep_timeout_mins = *mins;
                    last_active = Instant::now();
                    for (item, m) in &sleep_items {
                        item.set_checked(*m == sleep_timeout_mins);
                    }
                    tracing::info!("Auto sleep: {mins} min (0=disabled)");
                }
            }
        }

        // Periodic state refresh
        if let Event::NewEvents(_) = event {
            if last_poll.elapsed() >= poll_interval {
                last_poll = Instant::now();
                if let Some(interval) = refresh(
                    &headset,
                    &_tray,
                    &battery_item,
                    &mic_item,
                    &link_item,
                    &icon_connected,
                    &icon_disconnected,
                    &mut was_connected,
                    &mut notifier,
                    &mut mic_muted,
                    &mic_mute_item,
                ) {
                    poll_interval = interval;

                    // Reset activity timer when headset is active
                    if was_connected {
                        last_active = Instant::now();
                    }
                }
            }

            // Host-controlled auto-sleep
            if sleep_timeout_mins > 0 && was_connected {
                let timeout = Duration::from_secs(sleep_timeout_mins * 60);
                if last_active.elapsed() >= timeout {
                    tracing::info!("Inactivity timeout — triggering shutdown");
                    headset.trigger_shutdown();
                    last_active = Instant::now(); // prevent re-triggering
                }
            }
        }
    });
}

// ---------------------------------------------------------------------------
// Formatting
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
