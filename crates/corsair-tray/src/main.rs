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
    let sleep_countdown_item = MenuItem::new("", false, None);

    menu.append(&battery_item).unwrap();
    menu.append(&mic_item).unwrap();
    menu.append(&link_item).unwrap();
    menu.append(&sleep_countdown_item).unwrap();
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

    // LED colors
    let led_sub = Submenu::new("LED Color", true);
    let led_colors: &[((u8, u8, u8), &str)] = &[
        ((255, 255, 255), "White"),
        ((255, 0, 0), "Red"),
        ((0, 255, 0), "Green"),
        ((0, 0, 255), "Blue"),
        ((0, 255, 255), "Cyan"),
        ((255, 0, 255), "Purple"),
        ((255, 165, 0), "Orange"),
        ((255, 255, 0), "Yellow"),
    ];
    let mut led_items = Vec::new();
    for &(rgb, label) in led_colors {
        let item = MenuItem::new(label, true, None);
        led_sub.append(&item).unwrap();
        led_items.push((item, rgb));
    }
    led_sub.append(&PredefinedMenuItem::separator()).unwrap();
    let led_off_item = MenuItem::new("Off", true, None);
    led_sub.append(&led_off_item).unwrap();
    menu.append(&led_sub).unwrap();

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
    let power_off_item = MenuItem::new("Power Off", true, None);
    menu.append(&power_off_item).unwrap();
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
    let power_off_id = power_off_item.id().clone();
    let sidetone_id = sidetone_item.id().clone();
    let led_off_id = led_off_item.id().clone();
    let eq_ids: Vec<_> = eq_items.iter().map(|(i, x)| (i.id().clone(), *x)).collect();
    let led_ids: Vec<_> = led_items.iter().map(|(i, c)| (i.id().clone(), *c)).collect();
    let sleep_ids: Vec<_> = sleep_items.iter().map(|(i, m)| (i.id().clone(), *m)).collect();
    let tray_channel = TrayIconEvent::receiver();
    let menu_channel = MenuEvent::receiver();

    // State
    let mut last_poll = Instant::now();
    let mut poll_interval = POLL_ACTIVE;
    let mut was_connected = connected;
    let mut notifier = notify::BatteryNotifier::new();
    let mut sidetone_on = true;
    let mut active_eq: u8 = 0;
    let mut sleep_timeout_mins: u64 = 0;
    let mut last_active = Instant::now();

    #[allow(unused_assignments)]
    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::WaitUntil(Instant::now() + Duration::from_millis(100));

        // Drain tray events (don't refresh on click — avoids menu flicker)
        while tray_channel.try_recv().is_ok() {}

        if let Ok(ev) = menu_channel.try_recv() {
            if ev.id == quit_id {
                *control_flow = ControlFlow::Exit;
                return;
            }
            if ev.id == power_off_id {
                headset.shutdown();
            }
            if ev.id == led_off_id {
                headset.set_led_off();
            }
            for (id, (r, g, b)) in &led_ids {
                if ev.id == *id {
                    headset.set_led_color(*r, *g, *b);
                }
            }
            if ev.id == sidetone_id {
                sidetone_on = !sidetone_on;
                sidetone_item.set_checked(sidetone_on);
                headset.set_sidetone(sidetone_on);
            }
            for (id, idx) in &eq_ids {
                if ev.id == *id {
                    active_eq = *idx;
                    headset.set_eq_preset(active_eq);
                    for (item, i) in &eq_items { item.set_checked(*i == active_eq); }
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
                    update_sleep_countdown(
                        &sleep_countdown_item, sleep_timeout_mins, &last_active,
                    );
                    notifier.check(s.battery);
                    poll_interval = if c { POLL_ACTIVE } else { POLL_IDLE };
                    if c { last_active = Instant::now(); }
                } else {
                    _tray.set_title(Some(""));
                    battery_item.set_text("Battery: --");
                    link_item.set_text("Link: No dongle");
                    sleep_countdown_item.set_text("");
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

fn update_sleep_countdown(item: &MenuItem, timeout_mins: u64, last_active: &Instant) {
    if timeout_mins == 0 {
        item.set_text("");
        return;
    }
    let timeout = Duration::from_secs(timeout_mins * 60);
    let elapsed = last_active.elapsed();
    if elapsed >= timeout {
        item.set_text("Sleep: imminent");
    } else {
        let remaining = timeout - elapsed;
        let mins = remaining.as_secs() / 60;
        let secs = remaining.as_secs() % 60;
        item.set_text(format!("Sleep in {mins}:{secs:02}"));
    }
}

fn fmt_fw(s: &Option<headset::HeadsetState>) -> String {
    s.as_ref()
        .map(|s| format!("FW {}/{}", s.fw_tx, s.fw_rx))
        .unwrap_or_else(|| "FW --".into())
}
