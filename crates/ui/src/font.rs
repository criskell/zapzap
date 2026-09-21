//! Reads the atlas written by tools/bakefont (pt-BR glyphs only; anything else draws as '?').
//! Glyphs are looked up in place: no parsing, no cache, no allocation while drawing. Text is drawn
//! with LCD sub-pixel anti-aliasing (RGB stripe order), like the browser's.
//!
//! Emoji are 18x18 bitmaps from a second atlas (tools/bakeemoji), drawn inline with the text.
//!
//! A text style is a `u16`: the pixel size, plus `BOLD` or `MEDIUM` for the heavier faces. Only the styles the
//! atlas was baked with exist; asking for another one is a bug and panics.

use crate::canvas::Canvas;
use crate::pack::unpack;

static ATLAS: &[u8] = include_bytes!("../../../assets/font.atlas");
static EMOJI: &[u8] = include_bytes!("../../../assets/emoji.atlas");

/// Weight 600, for names and titles.
pub const BOLD: u16 = 0x100;
/// Weight 545, for the filter chips.
pub const MEDIUM: u16 = 0x200;

const ENTRY_BYTES: usize = 10;
const FALLBACK_CHAR: usize = '?' as usize;
/// Advances are stored in 1/64 px so that the pen keeps the fraction from letter to letter.
const SUBPIXELS: i32 = 64;

const EMOJI_SIZE: usize = 18;
const EMOJI_PALETTE_BYTES: usize = 16 * 3;
const EMOJI_ENTRY_BYTES: usize = EMOJI_PALETTE_BYTES + EMOJI_SIZE * EMOJI_SIZE;
/// Room an emoji takes in a line of text: the bitmap plus a little air.
const EMOJI_ADVANCE: i32 = 21;
/// How far the bitmap's top edge sits above the baseline.
const EMOJI_RISE: i32 = 15;
/// Below this no emoji exists, so the lookup is skipped for ordinary text.
const FIRST_EMOJI: u32 = 0x2190;
/// The flag of Brazil is two regional indicators; the atlas keeps it under this key.
const BRAZIL_FLAG: u32 = 0x1F_FFFF;

struct Glyph {
    /// Position in the atlas' character table; kerning pairs are keyed by it.
    index: usize,
    advance: i32,
    left: i32,
    top: i32,
    width: usize,
    height: usize,
    /// The glyph's coverage as packed by tools/bakefont, up to the end of the atlas; `unpack` stops on its own.
    packed: &'static [u8],
}

/// Room for the largest glyph's `width * height * 3` coverage bytes (the baker prints the largest).
const UNPACKED_MAX: usize = 1536;

fn u16_at(at: usize) -> usize {
    u16::from_le_bytes([ATLAS[at], ATLAS[at + 1]]) as usize
}

fn find_char(code_points_at: usize, char_count: usize, code: usize) -> Option<usize> {
    let (mut low, mut high) = (0, char_count);
    while low < high {
        let mid = (low + high) / 2;
        match u16_at(code_points_at + mid * 2).cmp(&code) {
            core::cmp::Ordering::Equal => return Some(mid),
            core::cmp::Ordering::Less => low = mid + 1,
            core::cmp::Ordering::Greater => high = mid,
        }
    }
    None
}

fn u32_at(at: usize) -> usize {
    u32::from_le_bytes([ATLAS[at], ATLAS[at + 1], ATLAS[at + 2], ATLAS[at + 3]]) as usize
}

/// Where the per-variant glyph tables start: after the kerning offsets and tables.
fn glyph_tables_at(variant_count: usize, char_count: usize) -> usize {
    u32_at(3 + variant_count * 2 + char_count * 2 + 12)
}

fn glyph(c: char, style: u16) -> Glyph {
    let variant_count = ATLAS[0] as usize;
    let char_count = u16_at(1);
    let variants_at = 3;
    let code_points_at = variants_at + variant_count * 2;
    let tables_at = glyph_tables_at(variant_count, char_count);

    let variant = (0..variant_count)
        .find(|&i| u16_at(variants_at + i * 2) == style as usize)
        .unwrap_or_else(|| panic!("text style {style:#x} is not in the atlas"));
    let index = find_char(code_points_at, char_count, c as usize)
        .or_else(|| find_char(code_points_at, char_count, FALLBACK_CHAR))
        .expect("atlas has no fallback glyph");

    let entry = &ATLAS[tables_at + (variant * char_count + index) * ENTRY_BYTES..][..ENTRY_BYTES];
    let (width, height) = (entry[4] as usize, entry[5] as usize);
    let offset = u32::from_le_bytes([entry[6], entry[7], entry[8], entry[9]]) as usize;
    Glyph {
        index,
        advance: u16::from_le_bytes([entry[0], entry[1]]) as i32,
        left: entry[2] as i8 as i32,
        top: entry[3] as i8 as i32,
        width,
        height,
        packed: &ATLAS[offset..],
    }
}

/// Kerning between two glyphs (by atlas index) in 1/64 px. The pairs are stored per weight in font
/// units (2048 per em), sorted, and found by binary search.
fn kerning(style: u16, left: usize, right: usize) -> i32 {
    let (variant_count, char_count) = (ATLAS[0] as usize, u16_at(1));
    let face = if style & MEDIUM != 0 {
        1
    } else if style & BOLD != 0 {
        2
    } else {
        0
    };
    let table = u32_at(3 + variant_count * 2 + char_count * 2 + face * 4);
    let key = left << 8 | right;
    let (mut low, mut high) = (0, u16_at(table));
    while low < high {
        let mid = (low + high) / 2;
        let at = table + 2 + mid * 4;
        match (ATLAS[at] as usize) << 8 | ATLAS[at + 1] as usize {
            found if found == key => {
                let units = i16::from_le_bytes([ATLAS[at + 2], ATLAS[at + 3]]) as i32;
                return units * (style & 0xff) as i32 * SUBPIXELS / 2048;
            }
            found if found < key => low = mid + 1,
            _ => high = mid,
        }
    }
    0
}

fn emoji_count() -> usize {
    u16_at_emoji(0)
}

fn u16_at_emoji(at: usize) -> usize {
    u16::from_le_bytes([EMOJI[at], EMOJI[at + 1]]) as usize
}

fn emoji_key(index: usize) -> u32 {
    let at = 2 + index * 4;
    u32::from_le_bytes([EMOJI[at], EMOJI[at + 1], EMOJI[at + 2], EMOJI[at + 3]])
}

fn find_emoji(key: u32) -> Option<usize> {
    let (mut low, mut high) = (0, emoji_count());
    while low < high {
        let mid = (low + high) / 2;
        match emoji_key(mid).cmp(&key) {
            core::cmp::Ordering::Equal => return Some(mid),
            core::cmp::Ordering::Less => low = mid + 1,
            core::cmp::Ordering::Greater => high = mid,
        }
    }
    None
}

/// The atlas number of the emoji `c`, if it has one.
pub fn emoji_index(c: char) -> Option<usize> {
    find_emoji(c as u32)
}

/// How many emoji the atlas has, for the picker.
pub fn emoji_total() -> usize {
    emoji_count()
}

/// The characters that make up emoji number `index`: one, or two for the flag of Brazil.
pub fn emoji_chars(index: usize) -> (char, Option<char>) {
    match emoji_key(index) {
        BRAZIL_FLAG => ('\u{1F1E7}', Some('\u{1F1F7}')),
        key => (char::from_u32(key).unwrap_or('?'), None),
    }
}

enum Piece {
    Glyph(char),
    Emoji(usize),
    /// Variation selector, joiner or skin tone: no room of its own.
    Skip,
}

struct Pieces<'a> {
    chars: core::iter::Peekable<core::str::Chars<'a>>,
}

fn pieces(text: &str) -> Pieces<'_> {
    Pieces { chars: text.chars().peekable() }
}

impl Iterator for Pieces<'_> {
    type Item = Piece;

    fn next(&mut self) -> Option<Piece> {
        let c = self.chars.next()?;
        Some(match c as u32 {
            0xFE0F | 0x200D | 0x1F3FB..=0x1F3FF => Piece::Skip,
            0x1F1E7 if self.chars.peek() == Some(&'\u{1F1F7}') => {
                self.chars.next();
                find_emoji(BRAZIL_FLAG).map_or(Piece::Glyph('?'), Piece::Emoji)
            }
            code if code >= FIRST_EMOJI => find_emoji(code).map_or(Piece::Glyph(c), Piece::Emoji),
            _ => Piece::Glyph(c),
        })
    }
}

/// Draws emoji number `index` with its top-left corner at (`x`, `top`).
pub fn draw_emoji(canvas: &mut Canvas, x: i32, top: i32, index: usize) {
    let entry = &EMOJI[2 + emoji_count() * 4 + index * EMOJI_ENTRY_BYTES..][..EMOJI_ENTRY_BYTES];
    let first = (canvas.y0 - top).clamp(0, EMOJI_SIZE as i32) as usize;
    let last = (canvas.y0 + canvas.rows - top).clamp(0, EMOJI_SIZE as i32) as usize;
    for row in first..last {
        for col in 0..EMOJI_SIZE {
            let byte = entry[EMOJI_PALETTE_BYTES + row * EMOJI_SIZE + col];
            let alpha = (byte & 15) * 17;
            if alpha != 0 {
                let rgb = &entry[(byte >> 4) as usize * 3..][..3];
                canvas.blend(x + col as i32, top + row as i32, (rgb[0] as u32) << 16 | (rgb[1] as u32) << 8 | rgb[2] as u32, alpha);
            }
        }
    }
}

/// Width of `text` in whole pixels, rounded up so a background drawn to it always contains it.
pub fn measure(text: &str, style: u16) -> i32 {
    (measure_exact(text, style) + SUBPIXELS - 1) / SUBPIXELS
}

/// Width of `text` in 1/64 px, kerning included.
pub fn measure_exact(text: &str, style: u16) -> i32 {
    let mut subpixels = 0;
    let mut previous: Option<usize> = None;
    for piece in pieces(text) {
        match piece {
            Piece::Glyph(c) => {
                let glyph = glyph(c, style);
                subpixels += glyph.advance + previous.map_or(0, |left| kerning(style, left, glyph.index));
                previous = Some(glyph.index);
            }
            Piece::Emoji(_) => {
                subpixels += EMOJI_ADVANCE * SUBPIXELS;
                previous = None;
            }
            Piece::Skip => {}
        }
    }
    subpixels
}

/// Word-wrapped lines of `text`, as slices of the original: no allocation.
pub struct Lines<'a> {
    rest: &'a str,
    style: u16,
    max_width: i32,
}

pub fn lines(text: &str, style: u16, max_width: i32) -> Lines<'_> {
    Lines { rest: text.trim_start(), style, max_width }
}

impl<'a> Iterator for Lines<'a> {
    type Item = &'a str;

    fn next(&mut self) -> Option<&'a str> {
        if self.rest.is_empty() {
            return None;
        }
        let text = self.rest;
        let mut end = 0;
        let mut pos = 0;
        while pos < text.len() {
            let word_end = text[pos..].find(char::is_whitespace).map_or(text.len(), |i| pos + i);
            if end != 0 && measure(&text[..word_end], self.style) > self.max_width {
                break;
            }
            end = word_end;
            pos = word_end + text[word_end..].find(|c: char| !c.is_whitespace()).unwrap_or(text.len() - word_end);
        }
        self.rest = text[end..].trim_start();
        Some(&text[..end])
    }
}

pub fn draw(canvas: &mut Canvas, x: i32, baseline: i32, style: u16, color: u32, text: &str) {
    let size_px = (style & 0xff) as i32;
    if baseline - size_px - EMOJI_RISE >= canvas.y0 + canvas.rows || baseline + size_px < canvas.y0 {
        return;
    }
    let mut pen = x * SUBPIXELS;
    let mut previous: Option<usize> = None;
    for piece in pieces(text) {
        let glyph = match piece {
            Piece::Glyph(c) => glyph(c, style),
            Piece::Emoji(index) => {
                draw_emoji(canvas, (pen + SUBPIXELS / 2) / SUBPIXELS + 1, baseline - EMOJI_RISE, index);
                pen += EMOJI_ADVANCE * SUBPIXELS;
                previous = None;
                continue;
            }
            Piece::Skip => continue,
        };
        pen += previous.map_or(0, |left| kerning(style, left, glyph.index));
        previous = Some(glyph.index);
        // The pen is placed to the nearest quarter of a pixel, as Skia does for sub-pixel positioning.
        // The glyph is stored as a stream of filtered sub-pixels (a third of a pixel each), so a quarter
        // of a pixel is 0.75 of a sub-pixel: read the stream shifted, blending two neighbours.
        let quarter = (pen * 4 + SUBPIXELS / 2).div_euclid(SUBPIXELS);
        let (whole, phase) = (quarter.div_euclid(4), quarter.rem_euclid(4));
        let shift = phase * 3; // in quarters of a sub-pixel
        let (shift_whole, weight_late) = (shift / 4, shift % 4);
        let (gx, gy) = (whole + glyph.left, baseline + glyph.top);
        let first = (canvas.y0 - gy).clamp(0, glyph.height as i32) as usize;
        let last = (canvas.y0 + canvas.rows - gy).clamp(0, glyph.height as i32) as usize;
        let stream = glyph.width as i32 * 3;
        let mut unpacked = [0u8; UNPACKED_MAX];
        if glyph.width * glyph.height > 0 && first < last {
            unpack(glyph.packed, glyph.width * 3, &mut unpacked[..glyph.width * glyph.height * 3]);
        }
        for row in first..last {
            let line = &unpacked[row * glyph.width * 3..(row + 1) * glyph.width * 3];
            let sample = |index: i32| if (0..stream).contains(&index) { line[index as usize] as i32 } else { 0 };
            for col in 0..glyph.width as i32 + i32::from(phase > 0) {
                let mut coverage = [0u8; 3];
                for (channel, value) in coverage.iter_mut().enumerate() {
                    let source = col * 3 + channel as i32 - shift_whole;
                    *value = (((4 - weight_late) * sample(source) + weight_late * sample(source - 1)) / 4) as u8;
                }
                if coverage != [0, 0, 0] {
                    canvas.blend_lcd(gx + col, gy + row as i32, color, coverage);
                }
            }
        }
        pen += glyph.advance;
    }
}
