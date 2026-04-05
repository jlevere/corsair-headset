//! Headphone template icons for macOS menu bar.
//!
//! Two variants: **solid** (connected) and **outline** (disconnected/standby).
//! Both are template images — macOS auto-tints them for light/dark mode.

use tray_icon::Icon;

const W: u32 = 18;
const H: u32 = 18;

/// Solid headphone icon — used when headset is actively connected.
#[rustfmt::skip]
const SOLID: [[u8; 18]; 18] = [
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

/// Outline headphone icon — used when headset is disconnected or standby.
/// Same shape as solid but with hollow ear pads (just the border).
#[rustfmt::skip]
const OUTLINE: [[u8; 18]; 18] = [
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
    [0,1,0,0,1,0,0,0,0,0,0,0,0,1,0,0,1,0],
    [0,1,0,0,1,0,0,0,0,0,0,0,0,1,0,0,1,0],
    [0,1,0,0,1,0,0,0,0,0,0,0,0,1,0,0,1,0],
    [0,1,0,0,1,0,0,0,0,0,0,0,0,1,0,0,1,0],
    [0,1,1,1,1,0,0,0,0,0,0,0,0,1,1,1,1,0],
    [0,0,1,1,1,0,0,0,0,0,0,0,0,1,1,1,0,0],
    [0,0,0,1,0,0,0,0,0,0,0,0,0,0,1,0,0,0],
];

fn bitmap_to_icon(bitmap: &[[u8; 18]; 18]) -> anyhow::Result<Icon> {
    let mut rgba = vec![0u8; (W * H * 4) as usize];
    for (y, row) in bitmap.iter().enumerate() {
        for (x, &pixel) in row.iter().enumerate() {
            if pixel == 1 {
                let off = (y * W as usize + x) * 4;
                rgba[off] = 0;       // R
                rgba[off + 1] = 0;   // G
                rgba[off + 2] = 0;   // B
                rgba[off + 3] = 255; // A
            }
        }
    }
    Icon::from_rgba(rgba, W, H).map_err(|e| anyhow::anyhow!("icon error: {e}"))
}

/// Create the solid (connected) headphone icon.
pub fn solid_icon() -> anyhow::Result<Icon> {
    bitmap_to_icon(&SOLID)
}

/// Create the outline (disconnected) headphone icon.
pub fn outline_icon() -> anyhow::Result<Icon> {
    bitmap_to_icon(&OUTLINE)
}
