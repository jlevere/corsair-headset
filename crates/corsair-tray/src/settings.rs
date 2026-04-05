//! Persistent settings stored in ~/Library/Application Support/Corsair Headset/

use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Settings {
    pub sidetone_on: bool,
    pub eq_preset: u8,
    pub led_color: Option<[u8; 3]>,
    pub sleep_timeout_mins: u64,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            sidetone_on: true,
            eq_preset: 0,
            led_color: None,
            sleep_timeout_mins: 0,
        }
    }
}

impl Settings {
    fn path() -> PathBuf {
        let mut p = dirs::data_dir().unwrap_or_else(|| PathBuf::from("."));
        p.push("Corsair Headset");
        p.push("settings.toml");
        p
    }

    pub fn load() -> Self {
        let path = Self::path();
        fs::read_to_string(&path)
            .ok()
            .and_then(|s| toml::from_str(&s).ok())
            .unwrap_or_default()
    }

    pub fn save(&self) {
        let path = Self::path();
        if let Some(parent) = path.parent() {
            let _ = fs::create_dir_all(parent);
        }
        if let Ok(s) = toml::to_string_pretty(self) {
            let _ = fs::write(&path, s);
        }
    }
}
