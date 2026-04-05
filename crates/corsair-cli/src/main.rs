use std::time::Duration;

fn main() -> anyhow::Result<()> {
    let args: Vec<String> = std::env::args().collect();
    let cmd = args.get(1).map(|s| s.as_str()).unwrap_or("status");

    let api = hidapi::HidApi::new()?;
    #[cfg(target_os = "macos")]
    api.set_open_exclusive(true);

    let iface = api
        .device_list()
        .find(|d| d.vendor_id() == 0x1B1C && d.usage_page() == 0xFFC5)
        .ok_or_else(|| anyhow::anyhow!("No Corsair headset found"))?;

    let device = api.open_path(iface.path())?;
    device.set_blocking_mode(false)?;
    let _ = device.write(&[0xC8, 0x01]);
    std::thread::sleep(Duration::from_millis(50));

    match cmd {
        "status" => {
            // Dump everything we can read
            for &(rid, name) in &[
                (0x64u8, "State"),
                (0x65, "Mode"),
                (0x66, "Firmware"),
                (0x90, "Report 0x90"),
                (0xC4, "Report 0xC4"),
                (0xE2, "Report 0xE2"),
            ] {
                let _ = device.write(&[0xC9, rid]);
                std::thread::sleep(Duration::from_millis(100));
                if let Some(r) = read(&device, rid) {
                    let p = &r[1..];
                    print!("{name} (0x{rid:02X}, {}B): ", p.len());
                    for b in p.iter().take(30) { print!("{b:02X} "); }
                    if p.len() > 30 { print!("..."); }
                    println!();
                } else {
                    println!("{name} (0x{rid:02X}): no response");
                }
            }

            // Feature report
            let mut fbuf = [0u8; 65];
            fbuf[0] = 0xFF;
            if let Ok(n) = device.get_feature_report(&mut fbuf) {
                print!("Feature 0xFF ({}B): ", n - 1);
                for b in fbuf[1..n].iter().take(20) { print!("{b:02X} "); }
                println!("...");
            }

            // Also try reading 0xC4 as feature report (it's both input AND output)
            let mut c4buf = [0u8; 65];
            c4buf[0] = 0xC4;
            if let Ok(n) = device.get_feature_report(&mut c4buf) {
                print!("Feature 0xC4 ({}B): ", n - 1);
                for b in c4buf[1..n].iter().take(30) { print!("{b:02X} "); }
                println!("...");
            }
        }
        "watch" => {
            println!("Rapid polling — move mic boom, press buttons. Ctrl+C to stop.\n");
            let mut last = Vec::new();
            loop {
                let _ = device.write(&[0xC9, 0x64]);
                if let Some(r) = read(&device, 0x64) {
                    let p = r[1..].to_vec();
                    if p != last {
                        for (i, b) in p.iter().enumerate() {
                            let changed = last.get(i).is_some_and(|prev| prev != b);
                            if changed { print!("\x1b[1;31m"); }
                            print!("{b:02X}");
                            if changed { print!("\x1b[0m"); }
                            print!(" ");
                        }
                        let bat = p[1] & 0x7F;
                        let mic = (p[1] >> 7) & 1;
                        print!(" | bat={bat}% mic={mic}");
                        println!();
                        last = p;
                    }
                }
                std::thread::sleep(Duration::from_millis(500));
            }
        }
        _ => {
            println!("  corsair status  — dump all reports");
            println!("  corsair watch   — rapid poll, highlight changes");
        }
    }
    Ok(())
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
