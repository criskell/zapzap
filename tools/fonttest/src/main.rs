//! Draws a sheet of text with the same font code the ui uses, to compare against a browser.
//! `cargo run --release -p fonttest -- out.png`

#![allow(dead_code)]

#[path = "../../../crates/ui/src/canvas.rs"]
mod canvas;
#[path = "../../../crates/ui/src/font.rs"]
mod font;
#[path = "../../../crates/ui/src/pack.rs"]
mod pack;

use canvas::Canvas;
use font::{BOLD, MEDIUM};

const BG: u32 = 0x161717;
const BUBBLE: u32 = 0x242626;
const PRIMARY: u32 = 0xfafafa;
const SECONDARY: u32 = 0xa2a3a3;
const CHIP_BG: u32 = 0x103529;
const CHIP_TEXT: u32 = 0xd9fdd3;

/// (style, colour, background, text): the same set the HTML page uses, in the same order.
pub const LINES: &[(u16, u32, u32, &str)] = &[
    (22 | BOLD, PRIMARY, BG, "ZapZap"),
    (16 | BOLD, PRIMARY, BG, "Ana"),
    (16, PRIMARY, BG, "Família Trabalho Grupo da faculdade"),
    (14, SECONDARY, BG, "Te vi! Pode vir. Mandei o arquivo. Seu pedido saiu para entrega"),
    (14 | MEDIUM, CHIP_TEXT, CHIP_BG, "Tudo   Não lidas"),
    (14, SECONDARY, BG, "Pesquisar ou começar uma nova conversa"),
    (14, PRIMARY, BUBBLE, "Perfeito! Vou pedir uma mesa perto da janela."),
    (14, PRIMARY, BUBBLE, "Aliás, o cardápio de hoje tem feijoada e uma opção vegetariana."),
    (12, SECONDARY, BG, "12:37   12:53   Ontem   Sáb"),
    (15, SECONDARY, BG, "Digite uma mensagem"),
];
const ROW: i32 = 36;
const WIDTH: i32 = 560;

fn main() {
    let path = std::env::args().nth(1).unwrap_or_else(|| "fonttest.png".into());
    let height = LINES.len() as i32 * ROW + 8;
    let mut pixels = vec![BG; (WIDTH * height) as usize];
    let mut canvas = Canvas { px: &mut pixels, w: WIDTH, h: height, y0: 0, rows: height };
    for (index, &(style, color, background, text)) in LINES.iter().enumerate() {
        let top = 4 + index as i32 * ROW;
        canvas.fill_rect(0, top, WIDTH, ROW, background);
        font::draw(&mut canvas, 20, top + 24, style, color, text);
    }
    let rgb: Vec<u8> = pixels.iter().flat_map(|p| [(p >> 16) as u8, (p >> 8) as u8, *p as u8]).collect();
    let file = std::fs::File::create(&path).unwrap();
    let mut encoder = png::Encoder::new(file, WIDTH as u32, height as u32);
    encoder.set_color(png::ColorType::Rgb);
    encoder.set_depth(png::BitDepth::Eight);
    encoder.write_header().unwrap().write_image_data(&rgb).unwrap();
    println!("{path}: {WIDTH}x{height}, {} lines", LINES.len());
    // Position of the end of every prefix of one line, to compare with the browser letter by letter.
    let line = "Família Trabalho Grupo da faculdade";
    let ends: Vec<String> = line.char_indices().map(|(i, c)| format!("{:.2}", font::measure_exact(&line[..i + c.len_utf8()], 16) as f32 / 64.0)).collect();
    println!("prefix widths 16px: {}", ends.join(" "));
    // Text widths as the ui measures them, for comparing with the browser's `measureText`.
    for (style, text, browser) in [
        (14, "Te vi! Pode vir. Mandei o arquivo. Seu pedido saiu para entrega", 387.56),
        (16, "Família Trabalho Grupo da faculdade", 261.17),
        (14, "Pesquisar ou começar uma nova conversa", 264.09),
        (14, "Perfeito! Vou pedir uma mesa perto da janela.", 283.81),
    ] {
        println!("width {style}px: ui {} px, browser {browser} px ({text:.24}...)", font::measure(text, style));
    }
}
