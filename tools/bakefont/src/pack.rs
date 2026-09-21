//! Lossless packing of coverage bitmaps (glyphs and icons); the runtime side is crates/ui/src/pack.rs.

/// Lossless packing of a coverage bitmap: `height` rows of `stride` bytes (`width * 3` for a glyph, `width` for an icon). Each byte
/// is predicted from its left, upper and upper-left neighbours (PNG's Paeth predictor, over the
/// sub-pixel stream, with 0 outside the glyph) and the difference is stored as tokens:
///   00nnnnnn            n + 1 zeros
///   010nnnnn h.. (n+3 signed nibbles, high nibble first, padded to a byte)
///   10nnnnnn b..        n + 1 bytes as they are
/// The runtime decodes the whole glyph into a stack buffer before drawing it.
pub fn paeth(left: u8, up: u8, up_left: u8) -> u8 {
    let (a, b, c) = (left as i32, up as i32, up_left as i32);
    let p = a + b - c;
    let (pa, pb, pc) = ((p - a).abs(), (p - b).abs(), (p - c).abs());
    if pa <= pb && pa <= pc { left } else if pb <= pc { up } else { up_left }
}

fn predictions(stride: usize, height: usize, bytes: &[u8]) -> Vec<u8> {
    let mut out = vec![0u8; bytes.len()];
    for row in 0..height {
        for k in 0..stride {
            let i = row * stride + k;
            let left = if k > 0 { bytes[i - 1] } else { 0 };
            let up = if row > 0 { bytes[i - stride] } else { 0 };
            let up_left = if row > 0 && k > 0 { bytes[i - stride - 1] } else { 0 };
            out[i] = paeth(left, up, up_left);
        }
    }
    out
}

fn small(residual: u8) -> bool {
    (residual as i8) >= -8 && (residual as i8) <= 7
}

pub fn pack(stride: usize, height: usize, coverage: &[u8]) -> Vec<u8> {
    let predicted = predictions(stride, height, coverage);
    let residual: Vec<u8> = coverage.iter().zip(&predicted).map(|(c, p)| c.wrapping_sub(*p)).collect();
    let n = residual.len();
    let mut out = Vec::new();
    let mut i = 0;
    while i < n {
        if residual[i] == 0 {
            let run = residual[i..].iter().take(64).take_while(|&&r| r == 0).count();
            out.push(run as u8 - 1);
            i += run;
            continue;
        }
        let nibbles = (i..n).take(32).take_while(|&j| small(residual[j]) && !(residual[j] == 0 && j + 1 < n && residual[j + 1] == 0)).count();
        if nibbles >= 3 {
            out.push(0x40 | (nibbles as u8 - 3));
            for pair in residual[i..i + nibbles].chunks(2) {
                out.push((pair[0] & 15) << 4 | pair.get(1).map_or(0, |r| r & 15));
            }
            i += nibbles;
            continue;
        }
        let mut run = 1;
        while i + run < n && run < 64 && residual[i + run] != 0 && !(small(residual[i + run]) && (1..3).all(|t| i + run + t < n && small(residual[i + run + t]) && residual[i + run + t] != 0)) {
            run += 1;
        }
        out.push(0x80 | (run as u8 - 1));
        out.extend(&residual[i..i + run]);
        i += run;
    }
    out
}

/// The inverse of `pack`, kept here to check every glyph round-trips.
pub fn unpack(stride: usize, height: usize, packed: &[u8]) -> Vec<u8> {
    let mut out = vec![0u8; stride * height];
    let (mut at, mut i) = (0, 0);
    let put = |out: &mut Vec<u8>, i: &mut usize, residual: u8| {
        let (row, k) = (*i / stride, *i % stride);
        let left = if k > 0 { out[*i - 1] } else { 0 };
        let up = if row > 0 { out[*i - stride] } else { 0 };
        let up_left = if row > 0 && k > 0 { out[*i - stride - 1] } else { 0 };
        out[*i] = residual.wrapping_add(paeth(left, up, up_left));
        *i += 1;
    };
    while i < out.len() {
        let token = packed[at];
        at += 1;
        match token >> 6 {
            0 => (0..=token & 63).for_each(|_| put(&mut out, &mut i, 0)),
            1 => {
                let count = (token & 31) as usize + 3;
                for k in 0..count {
                    let byte = packed[at + k / 2];
                    let nibble = if k % 2 == 0 { byte >> 4 } else { byte & 15 };
                    put(&mut out, &mut i, ((nibble << 4) as i8 >> 4) as u8);
                }
                at += (count + 1) / 2;
            }
            _ => {
                for k in 0..=(token & 63) as usize {
                    put(&mut out, &mut i, packed[at + k]);
                }
                at += (token & 63) as usize + 1;
            }
        }
    }
    assert_eq!(at, packed.len(), "packed glyph has trailing bytes");
    out
}

