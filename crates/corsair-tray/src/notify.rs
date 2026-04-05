//! Battery notifications via macOS native alerts.
//!
//! Uses `osascript` to show native macOS notifications. These are
//! dismissible, non-modal, and respect Do Not Disturb.

use std::process::Command;

/// Battery thresholds that trigger a notification.
const WARNING_THRESHOLD: u8 = 15;
const CRITICAL_THRESHOLD: u8 = 5;

/// Tracks which battery alerts have already been shown to avoid spam.
pub struct BatteryNotifier {
    warned_15: bool,
    warned_5: bool,
    last_battery: u8,
}

impl BatteryNotifier {
    pub fn new() -> Self {
        Self {
            warned_15: false,
            warned_5: false,
            last_battery: 100,
        }
    }

    /// Check battery level and show a notification if crossing a threshold.
    ///
    /// Only notifies once per threshold crossing. Resets when battery goes
    /// above the threshold again (e.g., after charging).
    pub fn check(&mut self, battery: u8) {
        // Reset warnings if battery went back up (charging)
        if battery > WARNING_THRESHOLD + 5 {
            self.warned_15 = false;
        }
        if battery > CRITICAL_THRESHOLD + 5 {
            self.warned_5 = false;
        }

        // Notify at 15%
        if battery <= WARNING_THRESHOLD
            && self.last_battery > WARNING_THRESHOLD
            && !self.warned_15
        {
            self.warned_15 = true;
            send_notification(
                "Battery Low",
                &format!("Headset battery at {battery}%. Consider charging."),
            );
        }

        // Notify at 5%
        if battery <= CRITICAL_THRESHOLD
            && self.last_battery > CRITICAL_THRESHOLD
            && !self.warned_5
        {
            self.warned_5 = true;
            send_notification(
                "Battery Critical",
                &format!("Headset battery at {battery}%! Charge soon."),
            );
        }

        self.last_battery = battery;
    }
}

fn send_notification(title: &str, message: &str) {
    // osascript always works on macOS, even for non-bundled apps.
    let script = format!(
        r#"display notification "{message}" with title "{title}""#,
    );
    let _ = Command::new("osascript")
        .arg("-e")
        .arg(&script)
        .spawn();
}
