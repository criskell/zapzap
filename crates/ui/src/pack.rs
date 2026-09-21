//! Undoes tools/bakefont's `pack`: the token stream of a coverage bitmap back to its bytes.

/// Paeth predictor of PNG: whichever of left, up and up-left is closest to left + up - up-left.
fn paeth(left: u8, up: u8, up_left: u8) -> u8 {
    let (a, b, c) = (left as i32, up as i32, up_left as i32);
    let p = a + b - c;
    let (pa, pb, pc) = ((p - a).abs(), (p - b).abs(), (p - c).abs());
    if pa <= pb && pa <= pc {
        left
    } else if pb <= pc {
        up
    } else {
        up_left
    }
}

/// Undoes tools/bakefont's `pack`: turns the token stream into `stride * height` coverage bytes.
pub fn unpack(packed: &[u8], stride: usize, out: &mut [u8]) {
    let (mut at, mut i) = (0, 0);
    let mut put = |out: &mut [u8], residual: u8| {
        let left = if i % stride > 0 { out[i - 1] } else { 0 };
        let up = if i >= stride { out[i - stride] } else { 0 };
        let up_left = if i >= stride && i % stride > 0 { out[i - stride - 1] } else { 0 };
        out[i] = residual.wrapping_add(paeth(left, up, up_left));
        i += 1;
    };
    let total = out.len();
    let mut done = 0;
    while done < total {
        let token = packed[at];
        at += 1;
        let count = match token >> 6 {
            0 => {
                let count = (token & 63) as usize + 1;
                for _ in 0..count {
                    put(out, 0);
                }
                count
            }
            1 => {
                let count = (token & 31) as usize + 3;
                for k in 0..count {
                    let byte = packed[at + k / 2];
                    let nibble = if k % 2 == 0 { byte >> 4 } else { byte & 15 };
                    put(out, ((nibble << 4) as i8 >> 4) as u8);
                }
                at += (count + 1) / 2;
                count
            }
            _ => {
                let count = (token & 63) as usize + 1;
                for k in 0..count {
                    put(out, packed[at + k]);
                }
                at += count;
                count
            }
        };
        done += count;
    }
}

