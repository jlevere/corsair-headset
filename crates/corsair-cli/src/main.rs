use std::time::Duration;

use corsair_proto::legacy::lighting;
use corsair_proto::legacy::lighting::LedZone;
use corsair_proto::legacy::power;

fn main() -> anyhow::Result<()> {
    let args: Vec<String> = std::env::args().collect();
    let cmd = args.get(1).map(|s| s.as_str()).unwrap_or("help");

    let api = hidapi::HidApi::new()?;
    #[cfg(target_os = "macos")]
    api.set_open_exclusive(true);

    let iface = api
        .device_list()
        .find(|d| d.vendor_id() == 0x1B1C && d.usage_page() == 0xFFC5)
        .ok_or_else(|| anyhow::anyhow!("No Corsair headset found"))?;

    let device = api.open_path(iface.path())?;
    device.set_blocking_mode(false)?;
    let _ = device.write(&[0xC8, 0x01]); // Software mode
    std::thread::sleep(Duration::from_millis(50));

    match cmd {
        "led" => {
            let color = args.get(2).map(|s| s.as_str()).unwrap_or("ff0000");
            let (r, g, b) = parse_hex_color(color);
            println!("Setting LEDs to #{color} (R={r} G={g} B={b})");

            // Set color on both logo zones
            for zone in [LedZone::LeftLogo, LedZone::RightLogo] {
                let report = lighting::encode_set_color(zone, r, g, b);
                let wire = report.wire_bytes();
                println!("  {:?}: {} bytes, id=0x{:02X}", zone, wire.len(), wire[0]);
                match device.write(&wire) {
                    Ok(n) => println!("    write OK ({n}b)"),
                    Err(e) => println!("    write err: {e}"),
                }
                std::thread::sleep(Duration::from_millis(50));
            }
        }
        "led-bright" => {
            let level: u8 = args.get(2).and_then(|s| s.parse().ok()).unwrap_or(50);
            println!("Setting brightness to {level}%");
            for zone in [LedZone::LeftLogo, LedZone::RightLogo] {
                let report = lighting::encode_set_brightness(zone, level);
                match device.write(&report.wire_bytes()) {
                    Ok(n) => println!("  {:?}: OK ({n}b)", zone),
                    Err(e) => println!("  {:?}: {e}", zone),
                }
                std::thread::sleep(Duration::from_millis(50));
            }
        }
        "led-off" => {
            println!("Clearing all LEDs");
            let reports = lighting::encode_clear_pwm();
            for report in &reports {
                let _ = device.write(&report.wire_bytes());
                std::thread::sleep(Duration::from_millis(50));
            }
            println!("Done.");
        }
        "led-engine" => {
            let action = args.get(2).map(|s| s.as_str()).unwrap_or("start");
            match action {
                "start" => {
                    println!("Starting TI engines");
                    let r = lighting::encode_start_ti_engines();
                    let _ = device.write(&r.wire_bytes());
                }
                "stop" => {
                    println!("Stopping TI engines");
                    let r = lighting::encode_stop_ti_engines();
                    let _ = device.write(&r.wire_bytes());
                }
                _ => println!("Usage: corsair led-engine start|stop"),
            }
        }
        "shutdown" => {
            println!("Sending auto-shutdown trigger (beep + power down)...");
            let report = power::encode_auto_shutdown_trigger();
            match device.send_feature_report(&report.wire_bytes()) {
                Ok(()) => println!("Sent. Headset should beep and power off."),
                Err(e) => println!("Error: {e}"),
            }
        }
        "sleep" => {
            println!("Sending shutdown command...");
            let report = power::encode_shutdown();
            match device.write(&report.wire_bytes()) {
                Ok(n) => println!("Sent ({n}b). Headset should power off."),
                Err(e) => println!("Error: {e}"),
            }
        }
        "status" => {
            let _ = device.write(&[0xC9, 0x64]);
            if let Some(r) = read(&device, 0x64) {
                let p = &r[1..];
                let bat = p[1] & 0x7F;
                let mic = if (p[1] & 0x80) != 0 { "DOWN" } else { "UP" };
                println!("Battery: {bat}%  Mic: {mic}  Link: {}  State: {}", p[2] & 0x0F, p[3] & 0x07);
            }
        }
        _ => {
            println!("Commands:");
            println!("  corsair status");
            println!("  corsair led <hex>        — set logo color (e.g. ff0000, 00ff00, 0000ff)");
            println!("  corsair led-bright <0-100> — set LED brightness");
            println!("  corsair led-off          — turn off all LEDs");
            println!("  corsair led-engine start|stop");
            println!("  corsair shutdown         — beep + power off (auto-shutdown trigger)");
            println!("  corsair sleep            — immediate power off (no beep)");
        }
    }
    Ok(())
}

fn parse_hex_color(hex: &str) -> (u8, u8, u8) {
    let hex = hex.trim_start_matches('#');
    if hex.len() >= 6 {
        let r = u8::from_str_radix(&hex[0..2], 16).unwrap_or(0);
        let g = u8::from_str_radix(&hex[2..4], 16).unwrap_or(0);
        let b = u8::from_str_radix(&hex[4..6], 16).unwrap_or(0);
        (r, g, b)
    } else {
        (255, 0, 0) // default red
    }
}

fn read(device: &hidapi::HidDevice, id: u8) -> Option<Vec<u8>> {
    let mut buf = [0u8; 65];
    let start = std::time::Instant::now();
    while start.elapsed() < Duration::from_millis(500) {
        if let Ok(n) = device.read_timeout(&mut buf, 100)
            && n >= 1
            && buf[0] == id
        {
            return Some(buf[..n].to_vec());
        }
    }
    None
}
