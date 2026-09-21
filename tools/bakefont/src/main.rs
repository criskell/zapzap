//! Bakes the UI fonts into assets/font.atlas: LCD (RGB sub-pixel) coverage bitmaps for a fixed set of
//! faces, sizes and code points, laid out so the runtime can index it without parsing.
//!
//! Only what Brazilian Portuguese needs: printable ASCII, accented letters, ordinal
//! indicators and the typographic punctuation that shows up in chat messages.
//!
//! Layout (little endian):
//!   u8 variant_count, u16 char_count, u16 variants[variant_count], u16 code_points[char_count] (sorted)
//!   u32 kerning_offsets[3] (regular, weight 545, weight 600), u32 glyph_tables_offset
//!   kerning tables: u16 pair_count, then pairs sorted by (left, right): u8 left, u8 right, i16 value
//!     (indexes into code_points; value in font units of 2048 per em, added to the left glyph's advance)
//!   per variant: char_count entries of 10 bytes, in code point order
//!     u16 advance (1/64 px), i8 left, i8 top, u8 width, u8 height, u32 coverage_offset
//!   coverage blob: per glyph, width*height pixels of 3 bytes (red, green, blue coverage) packed as
//!   described at `pack`; offsets are from the start of the file
//! A variant is `size`, `size | 0x100` (weight 600) or `size | 0x200` (weight 545).

mod pack;

use ab_glyph_rasterizer::{point, Point, Rasterizer};
use ttf_parser::gpos::{PairAdjustment, PositioningSubtable};
use pack::{pack, unpack};
use ttf_parser::{Face, GlyphId, OutlineBuilder};

const BOLD_FLAG: u16 = 0x100;
const MEDIUM_FLAG: u16 = 0x200;
/// (font file, variant flag, sizes). Roboto, the font WhatsApp Web uses, as static instances of the
/// variable font at the weights it asks for: 400 text, 545 filter chips, 600 names and titles.
/// A size with `Some(text)` is only ever drawn with fixed strings made of those characters, so
/// every other glyph keeps its advance but carries no bitmap.
type Size = (u16, Option<&'static str>);
const FACES: [(&str, u16, &[Size]); 3] = [
    (
        "assets/fonts/Roboto-400.ttf",
        0,
        &[
            (12, None),
            (14, None),
            (15, None),
            (16, None),
            (17, None),
            (22, None),
            (32, Some("Ligações de voz e vídeo Encontrar canais Compartilhe atualizações de status")),
        ],
    ),
    ("assets/fonts/Roboto-545.ttf", MEDIUM_FLAG, &[(12, None), (14, None), (16, Some("Favoritos Recentes"))]),
    ("assets/fonts/Roboto-600.ttf", BOLD_FLAG, &[(16, None), (17, None), (20, None), (22, Some("ZapZap"))]),
];
/// Light text on a dark background looks thin with linear coverage; this thickens it slightly.
const COVERAGE_GAMMA: f32 = 0.77;
/// How far from a zone (in font units) a point still counts as its overshoot and is flattened onto it.
const OVERSHOOT_UNITS: f32 = 26.0;
/// FreeType's default LCD filter (FT_LCD_FILTER_DEFAULT), in 256ths.
const FIR5: [f32; 5] = [8.0 / 256.0, 77.0 / 256.0, 86.0 / 256.0, 77.0 / 256.0, 8.0 / 256.0];
const ASCII: std::ops::RangeInclusive<u32> = 0x20..=0x7e;
const PT_BR: &str = "ÀÁÂÃÇÉÊÍÓÔÕÚÜàáâãçéêíóôõúüªº°´¨‘’“”–—…•";
const ENTRY_BYTES: usize = 10;

struct Baked {
    advance: u16,
    left: i8,
    top: i8,
    width: u8,
    height: u8,
    coverage: Vec<u8>,
}

/// Vertical alignment zones, the way FreeType's light hinting treats them: the baseline, x-height,
/// cap height and ascender land on whole pixel rows (heights round up), the descender rounds to the
/// nearest row, and the overshoot of round letters is flattened onto its zone. Horizontal shapes
/// stay unhinted, as in Chrome, so advances keep their fractions.
struct Zones {
    /// Pixels per font unit.
    scale: f32,
    /// (font unit, pixel row above the baseline), ascending.
    points: Vec<(f32, f32)>,
}

impl Zones {
    fn new(face: &Face, size: u16) -> Self {
        let scale = size as f32 / face.units_per_em() as f32;
        let extent = |c: char, top: bool| {
            let bounds = face.glyph_bounding_box(face.glyph_index(c).expect("glyph")).expect("bounds");
            (if top { bounds.y_max } else { bounds.y_min }) as f32
        };
        let (x_height, cap, ascender, descender) = (extent('x', true), extent('H', true), extent('l', true), extent('p', false));
        // The x-height rounds up (a little slack keeps 8.98 px from becoming 10), and everything else
        // is stretched by the same ratio before it is rounded to its nearest row, as FreeType does.
        let x_height_rows = (x_height * scale - 0.06).ceil();
        let stretch = x_height_rows / (x_height * scale);
        let snap = |units: f32| (units * scale * stretch).round();
        let mut points = vec![(descender, snap(descender)), (0.0, 0.0), (x_height, x_height_rows), (cap, snap(cap))];
        // An ascender less than a pixel above the cap height joins the cap height's row.
        if ascender - cap > 2.0 * OVERSHOOT_UNITS {
            let joined = (ascender - cap) * scale * stretch < 1.0;
            points.push((ascender, if joined { snap(cap) } else { snap(ascender) }));
        }
        for i in 1..points.len() {
            assert!(points[i].1 >= points[i - 1].1 && points[i].0 > points[i - 1].0, "zones collapsed at {size}px: {points:?}");
        }
        Zones { scale, points }
    }

    fn map(&self, mut units: f32) -> f32 {
        for &(zone, _) in &self.points {
            if (units - zone).abs() <= OVERSHOOT_UNITS {
                units = zone;
            }
        }
        let first = self.points[0];
        let last = *self.points.last().unwrap();
        if units <= first.0 {
            return first.1 + (units - first.0) * self.scale;
        }
        if units >= last.0 {
            return last.1 + (units - last.0) * self.scale;
        }
        let i = self.points.windows(2).position(|w| units <= w[1].0).unwrap();
        let (low, high) = (self.points[i], self.points[i + 1]);
        low.1 + (units - low.0) / (high.0 - low.0) * (high.1 - low.1)
    }
}

enum Curve {
    Line(Point, Point),
    Quad(Point, Point, Point),
    Cubic(Point, Point, Point, Point),
}

/// Collects a glyph outline in pixels (y down), with the zone alignment applied to y.
struct Outline<'a> {
    zones: &'a Zones,
    curves: Vec<Curve>,
    start: Point,
    current: Point,
}

impl Outline<'_> {
    fn at(&self, x: f32, y: f32) -> Point {
        point(x * self.zones.scale, -self.zones.map(y))
    }
}

impl OutlineBuilder for Outline<'_> {
    fn move_to(&mut self, x: f32, y: f32) {
        self.start = self.at(x, y);
        self.current = self.start;
    }
    fn line_to(&mut self, x: f32, y: f32) {
        let to = self.at(x, y);
        self.curves.push(Curve::Line(self.current, to));
        self.current = to;
    }
    fn quad_to(&mut self, x1: f32, y1: f32, x: f32, y: f32) {
        let (control, to) = (self.at(x1, y1), self.at(x, y));
        self.curves.push(Curve::Quad(self.current, control, to));
        self.current = to;
    }
    fn curve_to(&mut self, x1: f32, y1: f32, x2: f32, y2: f32, x: f32, y: f32) {
        let (c1, c2, to) = (self.at(x1, y1), self.at(x2, y2), self.at(x, y));
        self.curves.push(Curve::Cubic(self.current, c1, c2, to));
        self.current = to;
    }
    fn close(&mut self) {
        if self.current != self.start {
            self.curves.push(Curve::Line(self.current, self.start));
        }
        self.current = self.start;
    }
}

/// Kerning of one glyph pair, in font units: the x advance the GPOS pair adjustment lookups add to the
/// left glyph. The first subtable of a lookup that covers the pair decides, as the spec says.
fn pair_kerning(face: &Face, left: GlyphId, right: GlyphId) -> i32 {
    let Some(gpos) = face.tables().gpos else { return 0 };
    let mut total = 0;
    for lookup in gpos.lookups {
        for index in 0..lookup.subtables.len() {
            let Some(PositioningSubtable::Pair(adjustment)) = lookup.subtables.get::<PositioningSubtable>(index) else { continue };
            let value = match adjustment {
                PairAdjustment::Format1 { coverage, sets } => coverage
                    .get(left)
                    .and_then(|set_index| sets.get(set_index))
                    .and_then(|set| set.get(right))
                    .map(|(first, _)| first.x_advance),
                PairAdjustment::Format2 { coverage, classes, matrix } => coverage
                    .get(left)
                    .and_then(|_| matrix.get((classes.0.get(left), classes.1.get(right))))
                    .map(|(first, _)| first.x_advance),
            };
            if let Some(value) = value {
                total += value as i32;
                break;
            }
        }
    }
    total
}

/// Every non-zero kerning pair among `chars`, as (left index, right index, units).
fn kerning_table(face: &Face, chars: &[char]) -> Vec<(u8, u8, i16)> {
    let ids: Vec<Option<GlyphId>> = chars.iter().map(|&c| face.glyph_index(c)).collect();
    let mut pairs = Vec::new();
    for (left, left_id) in ids.iter().enumerate() {
        for (right, right_id) in ids.iter().enumerate() {
            let (Some(l), Some(r)) = (left_id, right_id) else { continue };
            let units = pair_kerning(face, *l, *r);
            if units != 0 {
                pairs.push((left as u8, right as u8, i16::try_from(units).expect("kerning fits i16")));
            }
        }
    }
    pairs
}

fn bake(face: &Face, zones: &Zones, c: char) -> Baked {
    let id = face.glyph_index(c).unwrap_or(GlyphId(0));
    let advance = (face.glyph_hor_advance(id).unwrap_or(0) as f32 * zones.scale * 64.0).round() as u16;
    let empty = Baked { advance, left: 0, top: 0, width: 0, height: 0, coverage: Vec::new() };
    let mut outline = Outline { zones, curves: Vec::new(), start: point(0.0, 0.0), current: point(0.0, 0.0) };
    if face.outline_glyph(id, &mut outline).is_none() || outline.curves.is_empty() {
        return empty;
    }

    let points = outline.curves.iter().flat_map(|curve| match curve {
        Curve::Line(a, b) => vec![*a, *b],
        Curve::Quad(a, b, c) => vec![*a, *b, *c],
        Curve::Cubic(a, b, c, d) => vec![*a, *b, *c, *d],
    });
    let (mut min, mut max) = (point(f32::MAX, f32::MAX), point(f32::MIN, f32::MIN));
    for p in points {
        (min.x, min.y, max.x, max.y) = (min.x.min(p.x), min.y.min(p.y), max.x.max(p.x), max.y.max(p.y));
    }
    // One pixel of padding each side, so the filter has room to spill.
    let (left, top) = (min.x.floor() - 1.0, min.y.floor());
    let (width, height) = ((max.x.ceil() + 1.0 - left) as usize, (max.y.ceil() - top) as usize);
    let shift = |p: &Point| point((p.x - left) * 3.0, p.y - top);

    // Sub-pixel coverage: the glyph rasterised at three times the horizontal resolution.
    let mut rasterizer = Rasterizer::new(width * 3, height);
    for curve in &outline.curves {
        match curve {
            Curve::Line(a, b) => rasterizer.draw_line(shift(a), shift(b)),
            Curve::Quad(a, b, c) => rasterizer.draw_quad(shift(a), shift(b), shift(c)),
            Curve::Cubic(a, b, c, d) => rasterizer.draw_cubic(shift(a), shift(b), shift(c), shift(d)),
        }
    }
    let mut sub = vec![0f32; width * 3 * height];
    rasterizer.for_each_pixel_2d(|x, y, alpha| sub[y as usize * width * 3 + x as usize] = alpha.clamp(0.0, 1.0));

    // Each channel is the sub-pixel coverage smoothed with FreeType's default LCD filter, centred on
    // its own sub-pixel (R, G, B from left to right).
    let at = |row: usize, k: isize| if k < 0 || k >= (width * 3) as isize { 0.0 } else { sub[row * width * 3 + k as usize] };
    let mut rgb = vec![0u8; width * height * 3];
    for row in 0..height {
        for pixel in 0..width {
            for channel in 0..3 {
                let k = (pixel * 3 + channel) as isize;
                let coverage: f32 = FIR5.iter().enumerate().map(|(tap, weight)| weight * at(row, k + tap as isize - 2)).sum();
                rgb[(row * width + pixel) * 3 + channel] = (coverage.powf(COVERAGE_GAMMA) * 255.0).round() as u8;
            }
        }
    }

    // Trim the empty columns the padding left behind.
    let used = |pixel: usize| (0..height).any(|row| rgb[(row * width + pixel) * 3..][..3].iter().any(|&v| v != 0));
    let (Some(first), Some(last)) = ((0..width).find(|&p| used(p)), (0..width).rev().find(|&p| used(p))) else { return empty };
    let trimmed_width = last - first + 1;
    let mut coverage = Vec::with_capacity(trimmed_width * height * 3);
    for row in 0..height {
        coverage.extend_from_slice(&rgb[(row * width + first) * 3..(row * width + last + 1) * 3]);
    }
    let (left, width) = (left + first as f32, trimmed_width);
    Baked {
        advance,
        left: left as i8,
        top: top as i8,
        width: u8::try_from(width).expect("glyph too wide"),
        height: u8::try_from(height).expect("glyph too tall"),
        coverage,
    }
}

fn main() {
    let mut chars: Vec<char> = ASCII.map(|c| char::from_u32(c).unwrap()).chain(PT_BR.chars()).collect();
    chars.sort();
    chars.dedup();
    let char_count = chars.len();

    let mut variants: Vec<u16> = Vec::new();
    let mut glyphs: Vec<Vec<Baked>> = Vec::new();
    let mut kerning: Vec<Vec<(u8, u8, i16)>> = Vec::new();
    for (path, flag, sizes) in FACES {
        let ttf = std::fs::read(path).unwrap_or_else(|e| panic!("read {path}: {e}"));
        let face = Face::parse(&ttf, 0).expect("parse font");
        kerning.push(kerning_table(&face, &chars));
        for &(size, only) in sizes {
            variants.push(size | flag);
            let zones = Zones::new(&face, size);
            if std::env::var_os("BAKEFONT_ZONES").is_some() {
                println!("{path} {size}px zones (unit -> px): {:?}", zones.points);
            }
            glyphs.push(
                chars
                    .iter()
                    .map(|&c| {
                        let mut glyph = bake(&face, &zones, c);
                        if only.is_some_and(|text| !text.contains(c)) {
                            (glyph.width, glyph.height, glyph.coverage) = (0, 0, Vec::new());
                        }
                        glyph
                    })
                    .collect(),
            );
        }
    }

    let kerning_bytes: Vec<usize> = kerning.iter().map(|table| 2 + table.len() * 4).collect();
    let header_len = 3 + variants.len() * 2 + char_count * 2 + 16;
    let tables_offset = header_len + kerning_bytes.iter().sum::<usize>();
    let blob_start = tables_offset + variants.len() * char_count * ENTRY_BYTES;

    let mut out = vec![variants.len() as u8];
    out.extend((char_count as u16).to_le_bytes());
    for variant in &variants {
        out.extend(variant.to_le_bytes());
    }
    for c in &chars {
        out.extend(u16::try_from(*c as u32).expect("code point above U+FFFF").to_le_bytes());
    }
    let mut at = header_len;
    for bytes in &kerning_bytes {
        out.extend((at as u32).to_le_bytes());
        at += bytes;
    }
    out.extend((tables_offset as u32).to_le_bytes());
    for table in &kerning {
        out.extend((table.len() as u16).to_le_bytes());
        for &(left, right, units) in table {
            out.extend([left, right]);
            out.extend(units.to_le_bytes());
        }
    }

    let mut blob: Vec<u8> = Vec::new();
    let mut largest = 0;
    for table in &glyphs {
        for g in table {
            out.extend(g.advance.to_le_bytes());
            out.extend([g.left as u8, g.top as u8, g.width, g.height]);
            out.extend(((blob_start + blob.len()) as u32).to_le_bytes());
            let packed = pack(g.width as usize * 3, g.height as usize, &g.coverage);
            assert_eq!(unpack(g.width as usize * 3, g.height as usize, &packed), g.coverage, "glyph does not round-trip");
            blob.extend(packed);
            largest = largest.max(g.coverage.len());
        }
    }
    out.extend(blob);

    std::fs::write("assets/font.atlas", &out).expect("write assets/font.atlas");
    println!("largest unpacked glyph: {largest} bytes (the ui's scratch buffer must hold it)");
    println!("assets/font.atlas: {} bytes, {} variants, {} glyphs each, kerning pairs per weight {:?}", out.len(), variants.len(), char_count, kerning.iter().map(Vec::len).collect::<Vec<_>>());
}
