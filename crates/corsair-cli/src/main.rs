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

    let device = api.open_path(iface.path())?;
    device.set_blocking_mode(false)?;
    let _ = device.write(&[0xC8, 0x01]);
    std::thread::sleep(Duration::from_millis(50));

    match cmd {
        "on" => {
            let raw_level: u8 = args.get(2).and_then(|s| s.parse().ok()).unwrap_or(6);
            println!("Sidetone ON (0xFF level={raw_level}, then SetValue unmute)");
            send_ff(&device, raw_level);
            std::thread::sleep(Duration::from_millis(50));
            device.write(&[0xCA, 0x05, 0x00, 0x00, 0x00])?;
        }
        "off" => {
            println!("Sidetone OFF");
            device.write(&[0xCA, 0x05, 0x01, 0x00, 0x00])?;
        }
        "level" => {
            let raw: u8 = args.get(2).and_then(|s| s.parse().ok()).unwrap_or(6);
            println!("Setting 0xFF level byte to {raw}");
            send_ff(&device, raw);
        }
        _ => {
            println!("  corsair on [level]  — activate sidetone (0xFF + SetValue)");
            println!("  corsair off         — mute sidetone");
            println!("  corsair level <0-255> — change 0xFF level byte only");
        }
    }
    Ok(())
}

fn send_ff(device: &hidapi::HidDevice, level: u8) {
    let mut payload = [0u8; 63];
    payload[0] = 0x0B;
    payload[2] = 0xFF;
    payload[3] = 0x04;
    payload[4] = 0x0E;
    payload[5] = 0x01;
    payload[6] = 0x05;
    payload[7] = 0x01;
    payload[8] = 0x04;
    payload[10] = level;
    let mut wire = vec![0xFFu8];
    wire.extend_from_slice(&payload);
    match device.send_feature_report(&wire) {
        Ok(()) => println!("  0xFF OK (level={level})"),
        Err(e) => println!("  0xFF FAIL: {e}"),
    }
}
