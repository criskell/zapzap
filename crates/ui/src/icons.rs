//! The WhatsApp Web icons, baked by tools/bakeicons from the page's own SVGs as 8-bit coverage
//! masks (`assets/icons.atlas`) and drawn here in any colour.

use crate::canvas::Canvas;
use crate::icon_ids::Icon;
use crate::pack::unpack;

static ATLAS: &[u8] = include_bytes!("../../../assets/icons.atlas");
const ENTRY_BYTES: usize = 5;

/// The largest icon is 64 x 64.
const MASK_MAX: usize = 64 * 64;

/// The icon's size and its packed mask (see tools/bakefont's `pack`).
fn entry(icon: Icon) -> (usize, &'static [u8]) {
    let at = 1 + icon as usize * ENTRY_BYTES;
    let size = ATLAS[at] as usize;
    let offset = u32::from_le_bytes([ATLAS[at + 1], ATLAS[at + 2], ATLAS[at + 3], ATLAS[at + 4]]) as usize;
    (size, &ATLAS[offset..])
}

/// Draws `icon` with its top-left corner at (`x`, `y`).
pub fn draw(canvas: &mut Canvas, x: i32, y: i32, icon: Icon, color: u32) {
    let (size, packed) = entry(icon);
    let mut mask = [0u8; MASK_MAX];
    unpack(packed, size, &mut mask[..size * size]);
    let first = (canvas.y0 - y).clamp(0, size as i32) as usize;
    let last = (canvas.y0 + canvas.rows - y).clamp(0, size as i32) as usize;
    for row in first..last {
        for col in 0..size {
            let alpha = mask[row * size + col];
            if alpha != 0 {
                canvas.blend(x + col as i32, y + row as i32, color, alpha);
            }
        }
    }
}

/// Draws `icon` centred on (`cx`, `cy`).
pub fn draw_centered(canvas: &mut Canvas, cx: i32, cy: i32, icon: Icon, color: u32) {
    let half = entry(icon).0 as i32 / 2;
    draw(canvas, cx - half, cy - half, icon, color);
}
