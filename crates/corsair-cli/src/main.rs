use std::time::Duration;

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

    let device = api.open_path(&iface.path().to_owned())?;
    device.set_blocking_mode(false)?;

    // Always switch to software mode
    let _ = device.write(&[0xC8, 0x01]);
    std::thread::sleep(Duration::from_millis(50));

    match cmd {
        "status" => {
            device.write(&[0xC9, 0x64])?;
            if let Some(r) = read(&device, 0x64) {
                let bat = r[2] & 0x7F;
                let mic = if (r[2] & 0x80) == 0 { "UP" } else { "DOWN" };
                println!("Battery: {bat}%  Mic: {mic}  Link: {}", r[3] & 0x0F);
            }
        }

        // Try muting sidetone via SetValue (0xCA) with ValueId=5 (SidetoneState)
        "mute" => {
            println!("Muting sidetone via SetValue (ValueId=5, value=1)...");
            device.write(&[0xCA, 0x05, 0x01, 0x00, 0x00])?;
            println!("Done. Is it silent now?");
        }
        "unmute" => {
            println!("Unmuting sidetone via SetValue (ValueId=5, value=0)...");
            device.write(&[0xCA, 0x05, 0x00, 0x00, 0x00])?;
            println!("Done. Can you hear yourself?");
        }

        // Try setting sidetone level via SetValue with DIFFERENT ValueIds
        // Maybe there's an undocumented ValueId for sidetone level
        "sv" => {
            let vid: u8 = args.get(2).and_then(|s| s.parse().ok()).unwrap_or(5);
            let val: u8 = args.get(3).and_then(|s| s.parse().ok()).unwrap_or(0);
            println!("SetValue: ValueId={vid}, value={val}");
            device.write(&[0xCA, vid, val, 0x00, 0x00])?;
            println!("Done.");
        }

        // Try different byte positions in the 0xFF report for the level
        "probe" => {
            let pos: usize = args.get(2).and_then(|s| s.parse().ok()).unwrap_or(10);
            let val: u8 = args.get(3).and_then(|s| s.parse().ok()).unwrap_or(0);
            println!("0xFF feature report: level byte at position {pos} = {val}");
            let mut payload = [0u8; 63];
            payload[0] = 0x0B;
            payload[2] = 0xFF;
            payload[3] = 0x04;
            payload[4] = 0x0E;
            payload[5] = 0x01;
            payload[6] = 0x05;
            payload[7] = 0x01;
            payload[8] = 0x04;
            if pos < 63 {
                payload[pos] = val;
            }
            let mut wire = vec![0xFFu8];
            wire.extend_from_slice(&payload);
            device.send_feature_report(&wire)?;
            println!("Done.");
        }

        // Scan: try every byte position with value 0 to find which one controls level
        "scan" => {
            let val: u8 = args.get(2).and_then(|s| s.parse().ok()).unwrap_or(0);
            println!("Scanning all byte positions with value={val}...");
            println!("Listen for changes between each position.\n");
            for pos in 0..20 {
                let mut payload = [0u8; 63];
                payload[0] = 0x0B;
                payload[2] = 0xFF;
                payload[3] = 0x04;
                payload[4] = 0x0E;
                payload[5] = 0x01;
                payload[6] = 0x05;
                payload[7] = 0x01;
                payload[8] = 0x04;
                // Override one position with the test value
                payload[pos] = val;
                let mut wire = vec![0xFFu8];
                wire.extend_from_slice(&payload);
                let _ = device.send_feature_report(&wire);
                println!("  pos={pos:2}: sent byte {pos}={val} (header bytes may be overwritten)");
                std::thread::sleep(Duration::from_secs(2));
            }
        }

        _ => {
            println!("Sidetone debugger:");
            println!("  corsair status");
            println!("  corsair mute          — mute via SetValue");
            println!("  corsair unmute        — unmute via SetValue");
            println!("  corsair sv <id> <val> — send SetValue with any ValueId");
            println!("  corsair probe <pos> <val> — send 0xFF report with val at byte position");
            println!("  corsair scan <val>    — try every byte position (2s each)");
        }
    }
    Ok(())
}

fn read(device: &hidapi::HidDevice, id: u8) -> Option<Vec<u8>> {
    let mut buf = [0u8; 65];
    let start = std::time::Instant::now();
    while start.elapsed() < Duration::from_millis(500) {
        if let Ok(n) = device.read_timeout(&mut buf, 100) {
            if n >= 1 && buf[0] == id { return Some(buf[..n].to_vec()); }
        }
    }
    None
}
