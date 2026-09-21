//! Reads a payload from stdin and prints the matrix as lines of 0/1. With a mask argument (0-7)
//! it forces that mask, otherwise it picks the best one.

use std::io::Read;

fn main() {
    let mut payload = Vec::new();
    std::io::stdin().read_to_end(&mut payload).unwrap();
    let qr = match std::env::args().nth(1) {
        Some(mask) => zapzap_qr::encode_with_mask(&payload, mask.parse().unwrap()),
        None => zapzap_qr::encode(&payload),
    }
    .expect("payload too long");
    for y in 0..zapzap_qr::SIZE {
        let row: String = (0..zapzap_qr::SIZE).map(|x| if qr.is_dark(x, y) { '1' } else { '0' }).collect();
        println!("{row}");
    }
}
