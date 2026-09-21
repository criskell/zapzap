//! QR Code encoder for the WhatsApp pairing payload and nothing else: version 10 (57x57),
//! error correction L, byte mode. No allocation; the matrix is 57 bitmap rows of u64.

#![cfg_attr(not(test), no_std)]

pub const SIZE: usize = 57;
/// Largest payload that fits version 10 at level L in byte mode.
pub const MAX_BYTES: usize = 271;

const DATA_CODEWORDS: usize = 274;
const TOTAL_CODEWORDS: usize = 346;
const BLOCKS: usize = 4;
const SHORT_BLOCKS: usize = 2;
const SHORT_DATA: usize = 68;
const EC_PER_BLOCK: usize = 18;
const ALIGNMENT: [usize; 3] = [6, 28, 50];
const FORMAT_LEVEL_L: u32 = 1;

pub struct Qr {
    rows: [u64; SIZE],
}

impl Qr {
    pub fn is_dark(&self, x: usize, y: usize) -> bool {
        self.rows[y] >> x & 1 == 1
    }
}

/// Encodes `data`, picking the mask with the lowest penalty. `None` if it does not fit.
pub fn encode(data: &[u8]) -> Option<Qr> {
    let codewords = codewords(data)?;
    let mut best: Option<(u32, Matrix)> = None;
    for mask in 0..8 {
        let matrix = Matrix::build(&codewords, mask);
        let penalty = matrix.penalty();
        if best.as_ref().is_none_or(|(lowest, _)| penalty < *lowest) {
            best = Some((penalty, matrix));
        }
    }
    best.map(|(_, matrix)| Qr { rows: matrix.dark })
}

/// Same as `encode` with a fixed mask (0..8); exists so tests can compare against a reference encoder.
pub fn encode_with_mask(data: &[u8], mask: u8) -> Option<Qr> {
    let codewords = codewords(data)?;
    Some(Qr { rows: Matrix::build(&codewords, mask).dark })
}

struct Bits {
    data: [u8; DATA_CODEWORDS],
    len: usize,
}

impl Bits {
    fn push(&mut self, value: u32, count: usize) {
        for i in (0..count).rev() {
            self.data[self.len / 8] |= ((value >> i & 1) as u8) << (7 - self.len % 8);
            self.len += 1;
        }
    }
}

fn gf_multiply(x: u8, y: u8) -> u8 {
    let mut z = 0u32;
    for bit in (0..8).rev() {
        z = (z << 1) ^ ((z >> 7) * 0x11d);
        z ^= ((y as u32 >> bit) & 1) * x as u32;
    }
    z as u8
}

fn ec_divisor() -> [u8; EC_PER_BLOCK] {
    let mut divisor = [0u8; EC_PER_BLOCK];
    divisor[EC_PER_BLOCK - 1] = 1;
    let mut root = 1u8;
    for _ in 0..EC_PER_BLOCK {
        for j in 0..EC_PER_BLOCK {
            divisor[j] = gf_multiply(divisor[j], root);
            if j + 1 < EC_PER_BLOCK {
                divisor[j] ^= divisor[j + 1];
            }
        }
        root = gf_multiply(root, 2);
    }
    divisor
}

fn ec_remainder(data: &[u8], divisor: &[u8; EC_PER_BLOCK]) -> [u8; EC_PER_BLOCK] {
    let mut rem = [0u8; EC_PER_BLOCK];
    for &byte in data {
        let factor = byte ^ rem[0];
        rem.copy_within(1.., 0);
        rem[EC_PER_BLOCK - 1] = 0;
        for i in 0..EC_PER_BLOCK {
            rem[i] ^= gf_multiply(divisor[i], factor);
        }
    }
    rem
}

/// Data bits, padding, error correction and interleaving, in final transmission order.
fn codewords(payload: &[u8]) -> Option<[u8; TOTAL_CODEWORDS]> {
    if payload.len() > MAX_BYTES {
        return None;
    }
    let mut stream = Bits { data: [0; DATA_CODEWORDS], len: 0 };
    stream.push(0b0100, 4);
    stream.push(payload.len() as u32, 16);
    for &byte in payload {
        stream.push(byte as u32, 8);
    }
    let capacity = DATA_CODEWORDS * 8;
    stream.push(0, 4.min(capacity - stream.len));
    stream.push(0, (8 - stream.len % 8) % 8);
    let mut pad = [0xec, 0x11].into_iter().cycle();
    while stream.len < capacity {
        stream.push(pad.next().unwrap(), 8);
    }
    let data = stream.data;

    let divisor = ec_divisor();
    let mut ec = [[0u8; EC_PER_BLOCK]; BLOCKS];
    let mut starts = [0usize; BLOCKS + 1];
    for block in 0..BLOCKS {
        let len = SHORT_DATA + usize::from(block >= SHORT_BLOCKS);
        starts[block + 1] = starts[block] + len;
        ec[block] = ec_remainder(&data[starts[block]..starts[block + 1]], &divisor);
    }

    let mut out = [0u8; TOTAL_CODEWORDS];
    let mut at = 0;
    for i in 0..=SHORT_DATA {
        for block in 0..BLOCKS {
            if starts[block] + i < starts[block + 1] {
                out[at] = data[starts[block] + i];
                at += 1;
            }
        }
    }
    for i in 0..EC_PER_BLOCK {
        for block_ec in &ec {
            out[at] = block_ec[i];
            at += 1;
        }
    }
    Some(out)
}

struct Matrix {
    dark: [u64; SIZE],
    function: [u64; SIZE],
}

impl Matrix {
    fn get(&self, x: usize, y: usize) -> bool {
        self.dark[y] >> x & 1 == 1
    }

    fn set_function(&mut self, x: isize, y: isize, dark: bool) {
        if x < 0 || y < 0 || x >= SIZE as isize || y >= SIZE as isize {
            return;
        }
        let (x, y) = (x as usize, y as usize);
        self.function[y] |= 1 << x;
        self.dark[y] = self.dark[y] & !(1 << x) | (dark as u64) << x;
    }

    fn is_function(&self, x: usize, y: usize) -> bool {
        self.function[y] >> x & 1 == 1
    }

    fn build(codewords: &[u8; TOTAL_CODEWORDS], mask: u8) -> Self {
        let mut m = Matrix { dark: [0; SIZE], function: [0; SIZE] };
        m.draw_function_patterns();
        m.draw_codewords(codewords);
        m.apply_mask(mask);
        m.draw_format(mask);
        m
    }

    fn draw_function_patterns(&mut self) {
        for i in 0..SIZE as isize {
            self.set_function(6, i, i % 2 == 0);
            self.set_function(i, 6, i % 2 == 0);
        }
        let far = SIZE as isize - 4;
        for (cx, cy) in [(3, 3), (far, 3), (3, far)] {
            for dy in -4..=4isize {
                for dx in -4..=4isize {
                    let distance = dx.abs().max(dy.abs());
                    self.set_function(cx + dx, cy + dy, distance != 2 && distance != 4);
                }
            }
        }
        let last = ALIGNMENT.len() - 1;
        for (i, &cy) in ALIGNMENT.iter().enumerate() {
            for (j, &cx) in ALIGNMENT.iter().enumerate() {
                if (i == 0 && j == 0) || (i == 0 && j == last) || (i == last && j == 0) {
                    continue;
                }
                for dy in -2..=2isize {
                    for dx in -2..=2isize {
                        self.set_function(cx as isize + dx, cy as isize + dy, dx.abs().max(dy.abs()) != 1);
                    }
                }
            }
        }
        self.draw_format(0);
        let mut rem = 10u32;
        for _ in 0..12 {
            rem = (rem << 1) ^ ((rem >> 11) * 0x1f25);
        }
        let bits = 10 << 12 | rem;
        for i in 0..18 {
            let bit = bits >> i & 1 == 1;
            let a = SIZE as isize - 11 + i % 3;
            let b = i / 3;
            self.set_function(a, b, bit);
            self.set_function(b, a, bit);
        }
    }

    fn draw_format(&mut self, mask: u8) {
        let data = FORMAT_LEVEL_L << 3 | mask as u32;
        let mut rem = data;
        for _ in 0..10 {
            rem = (rem << 1) ^ ((rem >> 9) * 0x537);
        }
        let bits = (data << 10 | rem) ^ 0x5412;
        let bit = |i: u32| bits >> i & 1 == 1;
        let size = SIZE as isize;
        for i in 0..=5 {
            self.set_function(8, i as isize, bit(i));
        }
        self.set_function(8, 7, bit(6));
        self.set_function(8, 8, bit(7));
        self.set_function(7, 8, bit(8));
        for i in 9..15 {
            self.set_function(14 - i as isize, 8, bit(i));
        }
        for i in 0..8 {
            self.set_function(size - 1 - i as isize, 8, bit(i));
        }
        for i in 8..15 {
            self.set_function(8, size - 15 + i as isize, bit(i));
        }
        self.set_function(8, size - 8, true);
    }

    fn draw_codewords(&mut self, codewords: &[u8; TOTAL_CODEWORDS]) {
        let mut bit = 0usize;
        let mut right = SIZE - 1;
        while right >= 1 {
            if right == 6 {
                right = 5;
            }
            for vertical in 0..SIZE {
                for j in 0..2 {
                    let x = right - j;
                    let upward = (right + 1) & 2 == 0;
                    let y = if upward { SIZE - 1 - vertical } else { vertical };
                    if !self.is_function(x, y) && bit < TOTAL_CODEWORDS * 8 {
                        let value = codewords[bit / 8] >> (7 - bit % 8) & 1 == 1;
                        self.dark[y] = self.dark[y] & !(1 << x) | (value as u64) << x;
                        bit += 1;
                    }
                }
            }
            right = right.saturating_sub(2);
            if right == 0 {
                break;
            }
        }
    }

    fn apply_mask(&mut self, mask: u8) {
        for y in 0..SIZE {
            for x in 0..SIZE {
                let invert = match mask {
                    0 => (x + y) % 2 == 0,
                    1 => y % 2 == 0,
                    2 => x % 3 == 0,
                    3 => (x + y) % 3 == 0,
                    4 => (x / 3 + y / 2) % 2 == 0,
                    5 => x * y % 2 + x * y % 3 == 0,
                    6 => (x * y % 2 + x * y % 3) % 2 == 0,
                    _ => ((x + y) % 2 + x * y % 3) % 2 == 0,
                };
                if invert && !self.is_function(x, y) {
                    self.dark[y] ^= 1 << x;
                }
            }
        }
    }

    fn penalty(&self) -> u32 {
        let mut total = 0;
        for line in 0..SIZE {
            for horizontal in [true, false] {
                let at = |i: usize| if horizontal { self.get(i, line) } else { self.get(line, i) };
                let mut run = 1;
                for i in 1..SIZE {
                    if at(i) == at(i - 1) {
                        run += 1;
                        total += match run {
                            5 => 3,
                            r if r > 5 => 1,
                            _ => 0,
                        };
                    } else {
                        run = 1;
                    }
                }
                for start in 0..=SIZE - 11 {
                    let window = (0..11).fold(0u32, |acc, k| acc << 1 | at(start + k) as u32);
                    if window == 0b10111010000 || window == 0b00001011101 {
                        total += 40;
                    }
                }
            }
        }
        for y in 0..SIZE - 1 {
            for x in 0..SIZE - 1 {
                let color = self.get(x, y);
                if color == self.get(x + 1, y) && color == self.get(x, y + 1) && color == self.get(x + 1, y + 1) {
                    total += 3;
                }
            }
        }
        let dark: u32 = self.dark.iter().map(|row| row.count_ones()).sum();
        let percent = dark * 100 / (SIZE * SIZE) as u32;
        let lower = percent - percent % 5;
        let steps = (lower.abs_diff(50)).min((lower + 5).abs_diff(50)) / 5;
        total + steps * 10
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn data_modules_match_codeword_count() {
        let mut m = Matrix { dark: [0; SIZE], function: [0; SIZE] };
        m.draw_function_patterns();
        let free: usize = (0..SIZE).flat_map(|y| (0..SIZE).map(move |x| (x, y))).filter(|&(x, y)| !m.is_function(x, y)).count();
        assert_eq!(free, TOTAL_CODEWORDS * 8);
    }

    fn fingerprint(qr: &Qr) -> u64 {
        qr.rows.iter().flat_map(|row| row.to_le_bytes()).fold(0xcbf29ce484222325, |h, b| (h ^ b as u64).wrapping_mul(0x100000001b3))
    }

    /// Hashes computed with the `qrcode` Python package (version 10, level L, byte mode, fixed mask).
    #[test]
    fn matches_reference_encoder() {
        let payload: Vec<u8> = b"zapzap-qr-regression-".iter().cycle().take(239).copied().collect();
        assert_eq!(fingerprint(&encode_with_mask(&payload, 0).unwrap()), 0xe7c1d733a2097ea3);
        assert_eq!(fingerprint(&encode_with_mask(&payload, 5).unwrap()), 0xfaaf5d36bc760b89);
    }

    #[test]
    fn rejects_payloads_that_do_not_fit() {
        assert!(encode(&[b'a'; MAX_BYTES]).is_some());
        assert!(encode(&[b'a'; MAX_BYTES + 1]).is_none());
    }
}
