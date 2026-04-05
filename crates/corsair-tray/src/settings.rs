//! Persistent settings stored in ~/Library/Application Support/Corsair Headset/

use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Settings {
    pub sidetone: bool,
    #[serde(default = "default_eq")]
    pub eq_preset: String,
    pub led_color: Option<String>,
    #[serde(default)]
    pub auto_sleep_minutes: u64,
}

fn default_eq() -> String { "pure-direct".into() }

impl Default for Settings {
    fn default() -> Self {
        Self {
            sidetone: true,
            eq_preset: default_eq(),
            led_color: None,
            auto_sleep_minutes: 0,
        }
    }
}

// EQ name <-> index mapping
const EQ_MAP: &[(&str, u8)] = &[
    ("pure-direct", 0),
    ("bass-boost", 1),
    ("clear-chat", 2),
    ("fps", 3),
    ("movie", 4),
];

// LED name <-> RGB mapping
const LED_MAP: &[(&str, [u8; 3])] = &[
    ("white", [255, 255, 255]),
    ("red", [255, 0, 0]),
    ("green", [0, 255, 0]),
    ("blue", [0, 0, 255]),
    ("cyan", [0, 255, 255]),
    ("purple", [255, 0, 255]),
    ("orange", [255, 165, 0]),
    ("yellow", [255, 255, 0]),
];

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

    pub fn eq_index(&self) -> u8 {
        EQ_MAP.iter()
            .find(|(name, _)| *name == self.eq_preset)
            .map(|(_, idx)| *idx)
            .unwrap_or(0)
    }

    pub fn set_eq_index(&mut self, idx: u8) {
        self.eq_preset = EQ_MAP.iter()
            .find(|(_, i)| *i == idx)
            .map(|(name, _)| (*name).into())
            .unwrap_or_else(|| "pure-direct".into());
    }

    pub fn led_rgb(&self) -> Option<[u8; 3]> {
        self.led_color.as_ref().and_then(|name| {
            LED_MAP.iter()
                .find(|(n, _)| *n == name.as_str())
                .map(|(_, rgb)| *rgb)
        })
    }

    pub fn set_led_rgb(&mut self, r: u8, g: u8, b: u8) {
        self.led_color = LED_MAP.iter()
            .find(|(_, rgb)| *rgb == [r, g, b])
            .map(|(name, _)| (*name).into());
    }
}
