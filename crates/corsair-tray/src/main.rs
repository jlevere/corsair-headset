use std::time::{Duration, Instant};

use muda::{CheckMenuItem, Menu, MenuEvent, MenuItem, PredefinedMenuItem, Submenu};
use tao::event::Event;
use tao::event_loop::{ControlFlow, EventLoopBuilder};
use tray_icon::{TrayIconBuilder, TrayIconEvent};

mod headset;
mod icon;
mod notify;

use headset::{Headset, LinkInfo};

const POLL_ACTIVE: Duration = Duration::from_secs(15);
const POLL_IDLE: Duration = Duration::from_secs(120);

const EQ_PRESETS: &[(u8, &str)] = &[
    (0, "Pure Direct"),
    (1, "Bass Boost"),
    (2, "Clear Chat"),
    (3, "FPS Competition"),
    (4, "Movie Theater"),
];

const SLEEP_TIMEOUTS: &[(u64, &str)] = &[
    (0, "Never"),
    (15, "15 min"),
    (30, "30 min"),
    (60, "1 hour"),
];

fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt().with_env_filter("info").init();

    let mut headset = Headset::new();
    let state = headset.poll_state();

    let menu = Menu::new();

    // Status
    let battery_item = MenuItem::new(fmt_battery(&state), false, None);
    let mic_item = MenuItem::new(fmt_mic(&state), false, None);
    let link_item = MenuItem::new(fmt_link(&state), false, None);
    let fw_item = MenuItem::new(fmt_fw(&state), false, None);
    menu.append(&battery_item).unwrap();
    menu.append(&mic_item).unwrap();
    menu.append(&link_item).unwrap();
    menu.append(&fw_item).unwrap();
    menu.append(&PredefinedMenuItem::separator()).unwrap();

    // Sidetone toggle
    let sidetone_item = CheckMenuItem::new("Sidetone", true, true, None);
    menu.append(&sidetone_item).unwrap();

    // EQ
    let eq_sub = Submenu::new("EQ Preset", true);
    let mut eq_items = Vec::new();
    for &(idx, label) in EQ_PRESETS {
        let item = CheckMenuItem::new(label, true, idx == 0, None);
        eq_sub.append(&item).unwrap();
        eq_items.push((item, idx));
    }
    menu.append(&eq_sub).unwrap();

    // Sleep
    let sleep_sub = Submenu::new("Auto Sleep", true);
    let mut sleep_items = Vec::new();
    for &(mins, label) in SLEEP_TIMEOUTS {
        let item = CheckMenuItem::new(label, true, mins == 0, None);
        sleep_sub.append(&item).unwrap();
        sleep_items.push((item, mins));
    }
    menu.append(&sleep_sub).unwrap();

    menu.append(&PredefinedMenuItem::separator()).unwrap();
    let sleep_now_item = MenuItem::new("Sleep Now", true, None);
    menu.append(&sleep_now_item).unwrap();
    menu.append(&PredefinedMenuItem::separator()).unwrap();
    let quit_item = MenuItem::new("Quit", true, None);
    menu.append(&quit_item).unwrap();

    // Tray
    let event_loop = EventLoopBuilder::new().build();
    let icon_solid = icon::solid_icon()?;
    let icon_outline = icon::outline_icon()?;
    let connected = state.as_ref().is_some_and(|s| s.link == LinkInfo::Active);

    let initial_title = match &state {
        Some(s) if s.link == LinkInfo::Active => format!("{}%", s.battery),
        Some(s) => s.link.label().into(),
        None => String::new(),
    };

    let _tray = TrayIconBuilder::new()
        .with_icon(if connected { icon_solid.clone() } else { icon_outline.clone() })
        .with_icon_as_template(true)
        .with_menu(Box::new(menu))
        .with_title(&initial_title)
        .with_tooltip("Corsair Headset")
        .build()?;

    // IDs
    let quit_id = quit_item.id().clone();
    let sleep_now_id = sleep_now_item.id().clone();
    let sidetone_id = sidetone_item.id().clone();
    let eq_ids: Vec<_> = eq_items.iter().map(|(i, x)| (i.id().clone(), *x)).collect();
    let sleep_ids: Vec<_> = sleep_items.iter().map(|(i, m)| (i.id().clone(), *m)).collect();
    let tray_channel = TrayIconEvent::receiver();
    let menu_channel = MenuEvent::receiver();

    // State
    let mut last_poll = Instant::now();
    let mut poll_interval = POLL_ACTIVE;
    let mut was_connected = connected;
    let mut notifier = notify::BatteryNotifier::new();
    let mut sidetone_on = true;
    #[allow(unused_assignments)]
    let mut current_eq: u8 = 0;
    let mut sleep_timeout_mins: u64 = 0;
    let mut last_active = Instant::now();

    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::WaitUntil(Instant::now() + Duration::from_millis(100));

        // Drain tray events (don't refresh on click — avoids menu flicker)
        while tray_channel.try_recv().is_ok() {}

        if let Ok(ev) = menu_channel.try_recv() {
            if ev.id == quit_id {
                *control_flow = ControlFlow::Exit;
                return;
            }
            if ev.id == sleep_now_id {
                headset.trigger_shutdown();
            }
            if ev.id == sidetone_id {
                sidetone_on = !sidetone_on;
                sidetone_item.set_checked(sidetone_on);
                headset.set_sidetone(sidetone_on);
            }
            for (id, idx) in &eq_ids {
                if ev.id == *id {
                    current_eq = *idx;
                    headset.set_eq_preset(*idx);
                    for (item, i) in &eq_items { item.set_checked(*i == current_eq); }
                }
            }
            for (id, mins) in &sleep_ids {
                if ev.id == *id {
                    sleep_timeout_mins = *mins;
                    last_active = Instant::now();
                    for (item, m) in &sleep_items { item.set_checked(*m == sleep_timeout_mins); }
                }
            }
        }

        if let Event::NewEvents(_) = event {
            if last_poll.elapsed() >= poll_interval {
                last_poll = Instant::now();

                if let Some(s) = headset.poll_state() {
                    let c = s.link == LinkInfo::Active;
                    if c != was_connected {
                        let _ = _tray.set_icon(Some(
                            if c { icon_solid.clone() } else { icon_outline.clone() }
                        ));
                        was_connected = c;
                    }
                    let title = if c { format!("{}%", s.battery) } else { s.link.label().into() };
                    _tray.set_title(Some(&title));
                    battery_item.set_text(fmt_battery(&Some(s.clone())));
                    mic_item.set_text(fmt_mic(&Some(s.clone())));
                    link_item.set_text(fmt_link(&Some(s.clone())));
                    notifier.check(s.battery);
                    poll_interval = if c { POLL_ACTIVE } else { POLL_IDLE };
                    if c { last_active = Instant::now(); }
                } else {
                    _tray.set_title(Some(""));
                    battery_item.set_text("Battery: --");
                    link_item.set_text("Link: No dongle");
                    if was_connected {
                        let _ = _tray.set_icon(Some(icon_outline.clone()));
                        was_connected = false;
                    }
                    poll_interval = POLL_IDLE;
                }
            }

            if sleep_timeout_mins > 0
                && was_connected
                && last_active.elapsed() >= Duration::from_secs(sleep_timeout_mins * 60)
            {
                headset.trigger_shutdown();
                last_active = Instant::now();
            }
        }
    });
}

fn fmt_battery(s: &Option<headset::HeadsetState>) -> String {
    s.as_ref()
        .map(|s| format!("Battery: {}% ({})", s.battery, s.battery_state.label()))
        .unwrap_or_else(|| "Battery: --".into())
}

fn fmt_mic(s: &Option<headset::HeadsetState>) -> String {
    s.as_ref()
        .map(|s| if s.mic_boom_up { "Mic: Off (boom up)" } else { "Mic: On" }.into())
        .unwrap_or_else(|| "Mic: --".into())
}

fn fmt_link(s: &Option<headset::HeadsetState>) -> String {
    s.as_ref()
        .map(|s| format!("Link: {}", s.link.label()))
        .unwrap_or_else(|| "Link: --".into())
}

fn fmt_fw(s: &Option<headset::HeadsetState>) -> String {
    s.as_ref()
        .map(|s| format!("FW {}/{}", s.fw_tx, s.fw_rx))
        .unwrap_or_else(|| "FW --".into())
}
