use std::time::Duration;

use corsair_proto::legacy::sidetone;

fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    let args: Vec<String> = std::env::args().collect();
    let sidetone_level: Option<u8> = args.get(1).and_then(|s| s.parse().ok());

    println!("=== Corsair VOID Elite Wireless ===\n");

    let api = hidapi::HidApi::new()?;
    #[cfg(target_os = "macos")]
    api.set_open_exclusive(true);

    let iface = api
        .device_list()
        .find(|d| d.vendor_id() == 0x1B1C && d.usage_page() == 0xFFC5)
        .ok_or_else(|| anyhow::anyhow!("No Corsair 0xFFC5 interface found"))?;

    let path = iface.path().to_owned();
    let device = api.open_path(&path)?;
    device.set_blocking_mode(false)?;

    // --- Read device state ---
    println!("--- Device State ---");

    // Firmware version
    device.write(&[0xC9, 0x66])?;
    if let Some(report) = read_report(&device) {
        let p = &report[1..];
        if report[0] == 0x66 && p.len() >= 4 {
            println!("  Firmware:  TX v{}.{}  RX v{}.{}", p[0], p[1], p[2], p[3]);
        }
    }

    // State
    device.write(&[0xC9, 0x64])?;
    if let Some(report) = read_report(&device) {
        let p = &report[1..];
        if report[0] == 0x64 && p.len() >= 4 {
            let battery = p[1] & 0x7F;
            let mic_up = (p[1] & 0x80) == 0;
            let link = p[2] & 0x0F;
            let bstate = p[3] & 0x07;
            let link_name = match link {
                1 => "Active", 3 => "Search", 4 => "Standby", _ => "?",
            };
            let bat_name = match bstate {
                1 => "Ok", 2 => "Low", 4 => "Full", 5 => "Charging", _ => "?",
            };
            println!("  Battery:   {battery}% ({bat_name})");
            println!("  Link:      {link_name}");
            println!("  Mic boom:  {}", if mic_up { "UP (muted)" } else { "DOWN (live)" });
        }
    }

    // Device mode
    device.write(&[0xC9, 0x65])?;
    if let Some(report) = read_report(&device) {
        let p = &report[1..];
        if report[0] == 0x65 && !p.is_empty() {
            println!("  Mode:      {}", if p[0] == 0 { "Hardware" } else { "Software" });
        }
    }

    println!();

    // --- Sidetone ---
    if let Some(level) = sidetone_level {
        let clamped = level.min(100);
        let db = sidetone::level_to_db(clamped);
        println!("--- Setting Sidetone to {clamped}% (dB byte: {db}) ---");

        let report = sidetone::encode_set_sidetone_level(clamped);
        let wire = report.wire_bytes();

        print!("  TX: 0xFF feature ({} bytes): [", wire.len());
        for (i, b) in wire.iter().take(16).enumerate() {
            if i > 0 { print!(" "); }
            print!("{b:02X}");
        }
        println!(" ...]");

        // Send as feature report (0xFF is a feature report per HID descriptor)
        match device.send_feature_report(&wire) {
            Ok(()) => println!("  Sent OK! You should hear the sidetone change."),
            Err(e) => println!("  Feature report error: {e}"),
        }

        // Also try via output write in case feature doesn't work
        if device.send_feature_report(&wire).is_err() {
            println!("  Trying as output report...");
            match device.write(&wire) {
                Ok(n) => println!("  Output write OK ({n} bytes)"),
                Err(e) => println!("  Output write error: {e}"),
            }
        }
    } else {
        println!("Usage: corsair <sidetone_percent>");
        println!("  e.g.: corsair 50    — set sidetone to 50%");
        println!("        corsair 0     — mute sidetone");
        println!("        corsair 100   — max sidetone");
    }

    Ok(())
}

fn read_report(device: &hidapi::HidDevice) -> Option<Vec<u8>> {
    let start = std::time::Instant::now();
    let mut buf = [0u8; 65];
    while start.elapsed() < Duration::from_secs(2) {
        match device.read_timeout(&mut buf, 200) {
            Ok(0) => {}
            Ok(n) => return Some(buf[..n].to_vec()),
            Err(_) => return None,
        }
    }
    None
}
