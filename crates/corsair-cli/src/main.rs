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
            device.write(&[0xC9, 0x64])?;
            if let Some(r) = read(&device, 0x64) {
                let p = &r[1..];
                println!("State (0x64) raw: {:02X?}", p);
                if p.len() >= 4 {
                    let bat = p[1] & 0x7F;
                    let mic_bit = (p[1] >> 7) & 1;
                    let link_lo = p[2] & 0x0F;
                    let link_hi = (p[2] >> 4) & 0x0F;
                    let bat_st = p[3] & 0x07;
                    println!("  battery={bat}%  mic_bit={mic_bit}  link_lo={link_lo}  link_hi=0x{link_hi:X}  bat_state={bat_st}");
                }
            }
            device.write(&[0xC9, 0x66])?;
            if let Some(r) = read(&device, 0x66) {
                println!("Firmware (0x66) raw: {:02X?}", &r[1..]);
            }
        }
        "watch" => {
            println!("Rapid polling — move mic boom, press buttons, etc.");
            println!("Changed bytes shown in RED. Ctrl+C to stop.\n");
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
                        let link = p[2] & 0x0F;
                        print!(" | bat={bat}% mic={mic} link={link}");
                        println!();
                        last = p;
                    }
                }
                std::thread::sleep(Duration::from_millis(500));
            }
        }
        _ => {
            println!("  corsair status  — dump raw report data");
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
