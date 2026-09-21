//! Bakes a small set of emoji from Noto Color Emoji (OFL) into assets/emoji.atlas.
//!
//! Layout (little endian):
//!   u16 count, u32 code_points[count] (sorted), then per emoji `ENTRY_BYTES`:
//!     16 palette colours as RGB, then SIZE*SIZE bytes, each `index << 4 | alpha` (both 4 bits).
//! The flag of Brazil, which is two code points, is stored under `BRAZIL_FLAG`.
//!
//! Run from the repository root: `cargo run --release -p bakeemoji [preview.png]`.

use std::collections::BTreeMap;

const FONT: &str = "/usr/share/fonts/truetype/noto/NotoColorEmoji.ttf";
const SIZE: usize = 18;
const PALETTE: usize = 16;
const ENTRY_BYTES: usize = PALETTE * 3 + SIZE * SIZE;
const BRAZIL_FLAG: u32 = 0x1F_FFFF;

/// The emoji people actually send in Brazilian chats. Ranges first, then singles.
const RANGES: &[(u32, u32)] = &[
    (0x1F600, 0x1F64F), // faces and gestures
    (0x1F910, 0x1F92F), // more faces
    (0x1F493, 0x1F49F), // hearts
    (0x1F44A, 0x1F450), // hands
    (0x1F446, 0x1F449),
    (0x1F332, 0x1F33C), // plants
    (0x1F345, 0x1F37F), // food and drinks
    (0x1F680, 0x1F683), // transport
];
const SINGLES: &[u32] = &[
    0x2764, 0x1F5A4, 0x1F90D, 0x1F90E, 0x1F9E1, 0x1F48B, 0x1F48C, 0x1F970, 0x1F973, 0x1F974, 0x1F975, 0x1F976, 0x1F97A, 0x1F9D0,
    0x1F937, 0x1F926, 0x1F481, 0x1F64C, 0x1F64F, 0x270C, 0x270B, 0x261D, 0x270A, 0x1F4AA, 0x1F440, 0x1F91D, 0x1F918, 0x1F919,
    0x2705, 0x274C, 0x2757, 0x2753, 0x2B50, 0x1F31F, 0x2728, 0x1F525, 0x1F4A5, 0x1F4AF, 0x1F4A2, 0x1F4A4, 0x1F4A6, 0x1F4A8,
    0x1F389, 0x1F38A, 0x1F381, 0x1F388, 0x1F3C6, 0x26A0, 0x26D4, 0x1F6AB, 0x2714, 0x27A1, 0x2B05, 0x1F519, 0x1F51D, 0x1F517,
    0x1F512, 0x1F513, 0x1F514, 0x1F4A1, 0x1F4B0, 0x1F4B5, 0x1F4B8, 0x1F4B3, 0x1F4F1, 0x1F4BB, 0x1F4F7, 0x1F4F8, 0x1F3A5,
    0x1F4DE, 0x1F4E7, 0x1F4E9, 0x1F4DD, 0x1F4C5, 0x1F4CE, 0x1F4CD, 0x1F4A9, 0x1F480, 0x1F47B, 0x1F47D,
    0x2600, 0x2601, 0x26C5, 0x26A1, 0x2744, 0x1F308, 0x1F319, 0x1F30D, 0x1F30E, 0x1F30A, 0x1F334, 0x1F333, 0x1F331, 0x1F340,
    0x1F341, 0x1F436, 0x1F431, 0x1F42F, 0x1F981, 0x1F434, 0x1F984, 0x1F437, 0x1F438, 0x1F435, 0x1F412, 0x1F414, 0x1F426,
    0x1F427, 0x1F422, 0x1F40D, 0x1F41F, 0x1F433, 0x1F42C, 0x1F98B, 0x1F41D, 0x1F41E, 0x1F951, 0x1F35E, 0x1F9C0, 0x2615,
    0x1F942, 0x1F964, 0x1F969, 0x26BD, 0x1F3C0, 0x1F3C8, 0x1F3BE, 0x1F3AE, 0x1F3B5, 0x1F3B6, 0x1F3A4, 0x1F3A7, 0x1F3B8,
    0x1F3AC, 0x1F4DA, 0x1F393, 0x1F3E0, 0x1F3E2, 0x1F697, 0x1F695, 0x1F68C, 0x1F6B2, 0x2708, 0x1F3D6, 0x1F468, 0x1F469,
    0x1F476, 0x1F474, 0x1F475, 0x1F46B, 0x1F46C, 0x1F46D, 0x1F385, 0x1F48D, 0x1F451, 0x1F460, 0x1F453, 0x1F60E,
];

type Rgba = [f32; 4];

fn decode_png(data: &[u8]) -> (usize, usize, Vec<u8>) {
    let mut decoder = png::Decoder::new(data);
    decoder.set_transformations(png::Transformations::normalize_to_color8());
    let mut reader = decoder.read_info().expect("png header");
    let mut buffer = vec![0; reader.output_buffer_size()];
    let info = reader.next_frame(&mut buffer).expect("png frame");
    let (w, h) = (info.width as usize, info.height as usize);
    let rgba = match info.color_type {
        png::ColorType::Rgba => buffer[..w * h * 4].to_vec(),
        png::ColorType::Rgb => buffer[..w * h * 3].chunks(3).flat_map(|p| [p[0], p[1], p[2], 255]).collect(),
        other => panic!("unexpected png colour type {other:?}"),
    };
    (w, h, rgba)
}

/// Area-average downscale on premultiplied colour, so transparent edges do not darken.
fn downscale(rgba: &[u8], width: usize, height: usize) -> Vec<Rgba> {
    let mut out = vec![[0.0; 4]; SIZE * SIZE];
    for dy in 0..SIZE {
        for dx in 0..SIZE {
            let (x0, x1) = (dx as f32 * width as f32 / SIZE as f32, (dx + 1) as f32 * width as f32 / SIZE as f32);
            let (y0, y1) = (dy as f32 * height as f32 / SIZE as f32, (dy + 1) as f32 * height as f32 / SIZE as f32);
            let mut sum = [0.0f32; 4];
            let mut weight = 0.0;
            for sy in y0.floor() as usize..(y1.ceil() as usize).min(height) {
                for sx in x0.floor() as usize..(x1.ceil() as usize).min(width) {
                    let overlap = (x1.min(sx as f32 + 1.0) - x0.max(sx as f32)).max(0.0) * (y1.min(sy as f32 + 1.0) - y0.max(sy as f32)).max(0.0);
                    let p = &rgba[(sy * width + sx) * 4..][..4];
                    let alpha = p[3] as f32 / 255.0;
                    sum[0] += p[0] as f32 * alpha * overlap;
                    sum[1] += p[1] as f32 * alpha * overlap;
                    sum[2] += p[2] as f32 * alpha * overlap;
                    sum[3] += alpha * overlap;
                    weight += overlap;
                }
            }
            out[dy * SIZE + dx] = if weight > 0.0 { [sum[0] / weight, sum[1] / weight, sum[2] / weight, sum[3] / weight] } else { [0.0; 4] };
        }
    }
    out
}

/// Median-cut palette of the visible pixels' straight (not premultiplied) colours.
fn palette_of(pixels: &[Rgba]) -> [[u8; 3]; PALETTE] {
    let mut colours: Vec<[f32; 3]> =
        pixels.iter().filter(|p| p[3] > 0.02).map(|p| [p[0] / p[3], p[1] / p[3], p[2] / p[3]]).collect();
    if colours.is_empty() {
        return [[0; 3]; PALETTE];
    }
    let mut buckets = vec![std::mem::take(&mut colours)];
    while buckets.len() < PALETTE {
        let (index, channel) = buckets
            .iter()
            .enumerate()
            .filter(|(_, b)| b.len() > 1)
            .flat_map(|(i, b)| (0..3).map(move |c| (i, c, b.iter().map(|p| p[c]).fold(f32::MAX, f32::min), b.iter().map(|p| p[c]).fold(f32::MIN, f32::max))))
            .max_by(|a, b| ((a.3 - a.2) * 1.0).total_cmp(&(b.3 - b.2)))
            .map(|(i, c, _, _)| (i, c))
            .unwrap_or((usize::MAX, 0));
        if index == usize::MAX {
            break;
        }
        let mut bucket = buckets.swap_remove(index);
        bucket.sort_by(|a, b| a[channel].total_cmp(&b[channel]));
        let upper = bucket.split_off(bucket.len() / 2);
        buckets.push(bucket);
        buckets.push(upper);
    }
    let mut palette = [[0u8; 3]; PALETTE];
    for (slot, bucket) in palette.iter_mut().zip(&buckets) {
        let n = bucket.len() as f32;
        for c in 0..3 {
            slot[c] = (bucket.iter().map(|p| p[c]).sum::<f32>() / n).round().clamp(0.0, 255.0) as u8;
        }
    }
    palette
}

fn encode(pixels: &[Rgba]) -> Vec<u8> {
    let palette = palette_of(pixels);
    let mut out: Vec<u8> = palette.iter().flatten().copied().collect();
    for p in pixels {
        let alpha = (p[3] * 15.0).round().clamp(0.0, 15.0) as u8;
        let colour = if p[3] > 0.0 { [p[0] / p[3], p[1] / p[3], p[2] / p[3]] } else { [0.0; 3] };
        let nearest = (0..PALETTE)
            .min_by(|&a, &b| distance(&palette[a], &colour).total_cmp(&distance(&palette[b], &colour)))
            .unwrap() as u8;
        out.push(nearest << 4 | alpha);
    }
    assert_eq!(out.len(), ENTRY_BYTES);
    out
}

fn distance(entry: &[u8; 3], colour: &[f32; 3]) -> f32 {
    (0..3).map(|c| (entry[c] as f32 - colour[c]).powi(2)).sum()
}

/// The flag is not in the cmap: the font builds it with a GSUB ligature of two regional indicators.
fn brazil_flag(face: &ttf_parser::Face) -> Option<ttf_parser::GlyphId> {
    use ttf_parser::gsub::SubstitutionSubtable;
    let (b, r) = (face.glyph_index('\u{1F1E7}')?, face.glyph_index('\u{1F1F7}')?);
    let gsub = face.tables().gsub?;
    for lookup in gsub.lookups {
        for index in 0..lookup.subtables.len() {
            let Some(SubstitutionSubtable::Ligature(ligatures)) = lookup.subtables.get::<SubstitutionSubtable>(index) else { continue };
            let Some(set_index) = ligatures.coverage.get(b) else { continue };
            let Some(set) = ligatures.ligature_sets.get(set_index) else { continue };
            for ligature in set {
                if ligature.components.len() == 1 && ligature.components.get(0) == Some(r) {
                    return Some(ligature.glyph);
                }
            }
        }
    }
    None
}

fn decode_entry(entry: &[u8]) -> Vec<[u8; 4]> {
    entry[PALETTE * 3..]
        .iter()
        .map(|byte| {
            let colour = &entry[(byte >> 4) as usize * 3..][..3];
            [colour[0], colour[1], colour[2], (byte & 15) * 17]
        })
        .collect()
}

fn main() {
    let data = std::fs::read(FONT).expect("read Noto Color Emoji");
    let face = ttf_parser::Face::parse(&data, 0).expect("parse font");

    let mut wanted: Vec<u32> = RANGES.iter().flat_map(|&(a, b)| a..=b).chain(SINGLES.iter().copied()).collect();
    wanted.sort();
    wanted.dedup();

    let mut entries: BTreeMap<u32, Vec<u8>> = BTreeMap::new();
    let mut missing = Vec::new();
    let mut bake = |key: u32, glyph: ttf_parser::GlyphId| match face.glyph_raster_image(glyph, 109) {
        Some(image) if image.format == ttf_parser::RasterImageFormat::PNG => {
            let (w, h, rgba) = decode_png(image.data);
            entries.insert(key, encode(&downscale(&rgba, w, h)));
            true
        }
        _ => false,
    };
    for cp in wanted {
        let glyph = char::from_u32(cp).and_then(|c| face.glyph_index(c));
        if !glyph.is_some_and(|glyph| bake(cp, glyph)) {
            missing.push(cp);
        }
    }
    if !brazil_flag(&face).is_some_and(|glyph| bake(BRAZIL_FLAG, glyph)) {
        missing.push(BRAZIL_FLAG);
    }

    let mut out = (entries.len() as u16).to_le_bytes().to_vec();
    for key in entries.keys() {
        out.extend(key.to_le_bytes());
    }
    for entry in entries.values() {
        out.extend(entry);
    }
    std::fs::write("assets/emoji.atlas", &out).expect("write assets/emoji.atlas");
    println!("assets/emoji.atlas: {} bytes, {} emoji ({} bytes each)", out.len(), entries.len(), ENTRY_BYTES);
    if !missing.is_empty() {
        println!("not in the font: {}", missing.iter().map(|cp| format!("{cp:X}")).collect::<Vec<_>>().join(" "));
    }

    if let Some(path) = std::env::args().nth(1) {
        let columns = 20;
        let rows = entries.len().div_ceil(columns);
        let cell = SIZE + 6;
        let (w, h) = (columns * cell, rows * cell);
        let mut sheet = vec![[24u8, 34, 41, 255]; w * h];
        for (n, entry) in entries.values().enumerate() {
            let (ox, oy) = ((n % columns) * cell + 3, (n / columns) * cell + 3);
            for (i, p) in decode_entry(entry).iter().enumerate() {
                let (x, y) = (ox + i % SIZE, oy + i / SIZE);
                let a = p[3] as u32;
                for c in 0..3 {
                    sheet[y * w + x][c] = ((p[c] as u32 * a + sheet[y * w + x][c] as u32 * (255 - a)) / 255) as u8;
                }
            }
        }
        let file = std::fs::File::create(&path).expect("preview file");
        let mut encoder = png::Encoder::new(file, w as u32, h as u32);
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        encoder.write_header().unwrap().write_image_data(&sheet.iter().flatten().copied().collect::<Vec<u8>>()).unwrap();
        println!("preview: {path}");
    }
}
