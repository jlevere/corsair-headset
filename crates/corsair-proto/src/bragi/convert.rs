//! Value conversions between host percentages and Bragi wire values.
//!
//! Bragi represents levels (sidetone, microphone, battery) as `u16` values
//! on a 0..=1000 scale, where 1000 = 100%. The host-facing API uses `u8`
//! percentages (0..=100).

/// Convert a host percentage (0..=100) to a Bragi level value (0..=1000).
///
/// The wire encoding is simply `percent * 10`.
#[must_use]
pub const fn to_bragi_level(percent: u8) -> u16 {
    percent as u16 * 10
}

/// Convert a Bragi level value (0..=1000) back to a host percentage (0..=100).
///
/// Integer division truncates; 1005 becomes 100, 9 becomes 0.
#[must_use]
pub const fn from_bragi_level(value: u16) -> u8 {
    (value / 10) as u8
}

/// Convert a sidetone percentage to the Bragi wire value.
///
/// Alias for [`to_bragi_level`] -- sidetone uses the standard 0..=1000 scale.
#[must_use]
pub const fn to_bragi_sidetone_level(percent: u8) -> u16 {
    to_bragi_level(percent)
}

/// Convert a Bragi sidetone wire value back to a percentage.
///
/// Alias for [`from_bragi_level`].
#[must_use]
pub const fn from_bragi_sidetone_level(value: u16) -> u8 {
    from_bragi_level(value)
}

/// Convert a microphone percentage to the Bragi wire value.
///
/// Alias for [`to_bragi_level`] -- microphone uses the standard 0..=1000 scale.
#[must_use]
pub const fn to_bragi_microphone_level(percent: u8) -> u16 {
    to_bragi_level(percent)
}

/// Convert a Bragi microphone wire value back to a percentage.
///
/// Alias for [`from_bragi_level`].
#[must_use]
pub const fn from_bragi_microphone_level(value: u16) -> u8 {
    from_bragi_level(value)
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    // -- Generic level --

    #[test]
    fn to_bragi_level_boundaries() {
        assert_eq!(to_bragi_level(0), 0);
        assert_eq!(to_bragi_level(50), 500);
        assert_eq!(to_bragi_level(100), 1000);
    }

    #[test]
    fn from_bragi_level_boundaries() {
        assert_eq!(from_bragi_level(0), 0);
        assert_eq!(from_bragi_level(500), 50);
        assert_eq!(from_bragi_level(1000), 100);
    }

    #[test]
    fn from_bragi_level_truncates() {
        assert_eq!(from_bragi_level(9), 0);
        assert_eq!(from_bragi_level(15), 1);
        assert_eq!(from_bragi_level(999), 99);
    }

    #[test]
    fn roundtrip_generic() {
        for pct in 0..=100u8 {
            assert_eq!(from_bragi_level(to_bragi_level(pct)), pct);
        }
    }

    // -- Sidetone --

    #[test]
    fn sidetone_roundtrip() {
        for pct in 0..=100u8 {
            assert_eq!(from_bragi_sidetone_level(to_bragi_sidetone_level(pct)), pct);
        }
    }

    #[test]
    fn sidetone_specific_values() {
        assert_eq!(to_bragi_sidetone_level(0), 0);
        assert_eq!(to_bragi_sidetone_level(75), 750);
        assert_eq!(from_bragi_sidetone_level(750), 75);
    }

    // -- Microphone --

    #[test]
    fn microphone_roundtrip() {
        for pct in 0..=100u8 {
            assert_eq!(from_bragi_microphone_level(to_bragi_microphone_level(pct)), pct);
        }
    }

    #[test]
    fn microphone_specific_values() {
        assert_eq!(to_bragi_microphone_level(0), 0);
        assert_eq!(to_bragi_microphone_level(100), 1000);
        assert_eq!(from_bragi_microphone_level(430), 43);
    }
}
