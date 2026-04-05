//! Headphone template icon for macOS menu bar.
//!
//! A template image is monochrome — macOS automatically tints it to match
//! the current appearance (white on dark menu bar, black on light).
//!
//! The icon is an 18x18 pixel headphone silhouette stored as a const bitmap.

use tray_icon::Icon;

const W: u32 = 18;
const H: u32 = 18;

/// Headphone icon as an 18x18 1-bit bitmap (1 = filled, 0 = transparent).
/// Drawn to be recognizable at menu bar size.
#[rustfmt::skip]
const HEADPHONE_BITMAP: [[u8; 18]; 18] = [
    [0,0,0,0,0,0,1,1,1,1,1,1,0,0,0,0,0,0],
    [0,0,0,0,0,1,1,0,0,0,0,1,1,0,0,0,0,0],
    [0,0,0,0,1,1,0,0,0,0,0,0,1,1,0,0,0,0],
    [0,0,0,1,1,0,0,0,0,0,0,0,0,1,1,0,0,0],
    [0,0,0,1,0,0,0,0,0,0,0,0,0,0,1,0,0,0],
    [0,0,1,1,0,0,0,0,0,0,0,0,0,0,1,1,0,0],
    [0,0,1,0,0,0,0,0,0,0,0,0,0,0,0,1,0,0],
    [0,0,1,0,0,0,0,0,0,0,0,0,0,0,0,1,0,0],
    [0,0,1,0,0,0,0,0,0,0,0,0,0,0,0,1,0,0],
    [0,0,1,0,0,0,0,0,0,0,0,0,0,0,0,1,0,0],
    [0,1,1,1,0,0,0,0,0,0,0,0,0,0,1,1,1,0],
    [0,1,1,1,1,0,0,0,0,0,0,0,0,1,1,1,1,0],
    [0,1,1,1,1,0,0,0,0,0,0,0,0,1,1,1,1,0],
    [0,1,1,1,1,0,0,0,0,0,0,0,0,1,1,1,1,0],
    [0,1,1,1,1,0,0,0,0,0,0,0,0,1,1,1,1,0],
    [0,1,1,1,1,0,0,0,0,0,0,0,0,1,1,1,1,0],
    [0,0,1,1,1,0,0,0,0,0,0,0,0,1,1,1,0,0],
    [0,0,0,1,0,0,0,0,0,0,0,0,0,0,1,0,0,0],
];

/// Create the headphone template icon.
///
/// Returns a monochrome RGBA icon where filled pixels are black (0,0,0,255)
/// and empty pixels are fully transparent. When set as a template image on
/// macOS, the system tints the black pixels to match the menu bar theme.
pub fn headphone_icon() -> anyhow::Result<Icon> {
    let mut rgba = vec![0u8; (W * H * 4) as usize];

    for y in 0..H as usize {
        for x in 0..W as usize {
            let offset = (y * W as usize + x) * 4;
            if HEADPHONE_BITMAP[y][x] == 1 {
                // Black pixel, full opacity — macOS will tint this.
                rgba[offset] = 0;     // R
                rgba[offset + 1] = 0; // G
                rgba[offset + 2] = 0; // B
                rgba[offset + 3] = 255; // A
            }
            // else: leave as (0,0,0,0) = transparent
        }
    }

    Icon::from_rgba(rgba, W, H).map_err(|e| anyhow::anyhow!("icon error: {e}"))
}
