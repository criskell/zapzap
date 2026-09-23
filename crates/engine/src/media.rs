//! What a message carries besides text (photo, video, voice, document, sticker), reduced to what the ui
//! draws: a kind, the size for the box, a short label, and the tiny preview picture WhatsApp embeds.

use whatsapp_rust::prelude::*;
use whatsapp_rust::wacore::proto_helpers::MessageExt;

/// The largest side of a preview sent to the ui, in pixels.
pub const THUMB_SIDE: usize = 48;

/// Kinds as the ui numbers them (`MEDIA` line).
pub const IMAGE: u8 = 1;
pub const VIDEO: u8 = 2;
pub const AUDIO: u8 = 3;
pub const DOCUMENT: u8 = 4;
pub const STICKER: u8 = 5;

#[derive(Clone)]
pub struct Media {
    pub kind: u8,
    pub width: u32,
    pub height: u32,
    /// A file name, or a duration such as "0:12".
    pub label: String,
    pub caption: String,
}

fn duration(seconds: u32) -> String {
    format!("{}:{:02}", seconds / 60, seconds % 60)
}

/// The media in `message`, if it is one (wrappers such as "view once" or "ephemeral" are looked through).
pub fn media_of(message: &wa::Message) -> Option<Media> {
    let base = message.get_base_message();
    if let Some(image) = base.image_message.as_option() {
        return Some(Media { kind: IMAGE, width: image.width.unwrap_or(0), height: image.height.unwrap_or(0), label: String::new(), caption: image.caption.clone().unwrap_or_default() });
    }
    if let Some(video) = base.video_message.as_option() {
        return Some(Media { kind: VIDEO, width: video.width.unwrap_or(0), height: video.height.unwrap_or(0), label: duration(video.seconds.unwrap_or(0)), caption: video.caption.clone().unwrap_or_default() });
    }
    if let Some(audio) = base.audio_message.as_option() {
        return Some(Media { kind: AUDIO, width: 0, height: 0, label: duration(audio.seconds.unwrap_or(0)), caption: String::new() });
    }
    if let Some(document) = base.document_message.as_option() {
        let name = document.file_name.clone().or_else(|| document.title.clone()).unwrap_or_else(|| "Documento".to_string());
        return Some(Media { kind: DOCUMENT, width: 0, height: 0, label: name, caption: document.caption.clone().unwrap_or_default() });
    }
    if let Some(sticker) = base.sticker_message.as_option() {
        return Some(Media { kind: STICKER, width: sticker.width.unwrap_or(0), height: sticker.height.unwrap_or(0), label: String::new(), caption: String::new() });
    }
    None
}

/// The embedded preview of a photo, video or document.
pub fn thumbnail_of(message: &wa::Message) -> Option<&[u8]> {
    let base = message.get_base_message();
    let bytes = base
        .image_message
        .as_option()
        .and_then(|m| m.jpeg_thumbnail.as_deref())
        .or_else(|| base.video_message.as_option().and_then(|m| m.jpeg_thumbnail.as_deref()))
        .or_else(|| base.document_message.as_option().and_then(|m| m.jpeg_thumbnail.as_deref()))?;
    (!bytes.is_empty()).then_some(bytes)
}

/// A preview in RGB565, at most `THUMB_SIDE` on a side: (width, height, rows of little-endian pixels).
pub fn decode_thumbnail(jpeg: &[u8]) -> Option<(usize, usize, Vec<u8>)> {
    let mut decoder = jpeg_decoder::Decoder::new(jpeg);
    let pixels = decoder.decode().ok()?;
    let info = decoder.info()?;
    let (width, height) = (info.width as usize, info.height as usize);
    let channels = match info.pixel_format {
        jpeg_decoder::PixelFormat::RGB24 => 3,
        jpeg_decoder::PixelFormat::L8 => 1,
        _ => return None,
    };
    if width == 0 || height == 0 || pixels.len() < width * height * channels {
        return None;
    }
    let scale = width.max(height).div_ceil(THUMB_SIDE).max(1);
    let (out_w, out_h) = ((width / scale).max(1), (height / scale).max(1));
    let mut out = Vec::with_capacity(out_w * out_h * 2);
    for y in 0..out_h {
        for x in 0..out_w {
            // The average of the block of source pixels this one stands for.
            let mut sum = [0u32; 3];
            let mut count = 0;
            for sy in y * scale..((y + 1) * scale).min(height) {
                for sx in x * scale..((x + 1) * scale).min(width) {
                    let at = (sy * width + sx) * channels;
                    for (part, total) in sum.iter_mut().enumerate() {
                        *total += pixels[at + if channels == 3 { part } else { 0 }] as u32;
                    }
                    count += 1;
                }
            }
            let count = count.max(1);
            let (r, g, b) = (sum[0] / count, sum[1] / count, sum[2] / count);
            let packed = ((r >> 3) << 11 | (g >> 2) << 5 | (b >> 3)) as u16;
            out.extend_from_slice(&packed.to_le_bytes());
        }
    }
    Some((out_w, out_h, out))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn durations() {
        assert_eq!(duration(0), "0:00");
        assert_eq!(duration(75), "1:15");
    }

    /// A 100 by 60 JPEG that fades from red (top) to blue (bottom).
    const GRADIENT: &[u8] = &[255, 216, 255, 224, 0, 16, 74, 70, 73, 70, 0, 1, 1, 0, 0, 0, 0, 0, 0, 0, 255, 219, 0, 67, 0, 20, 14, 15, 18, 15, 13, 20, 18, 16, 18, 23, 21, 20, 24, 30, 50, 33, 30, 28, 28, 30, 61, 44, 46, 36, 50, 73, 64, 76, 75, 71, 64, 70, 69, 80, 90, 115, 98, 80, 85, 109, 86, 69, 70, 100, 136, 101, 109, 119, 123, 129, 130, 129, 78, 96, 141, 151, 140, 125, 150, 115, 126, 129, 124, 255, 219, 0, 67, 1, 21, 23, 23, 30, 26, 30, 59, 33, 33, 59, 124, 83, 70, 83, 124, 124, 124, 124, 124, 124, 124, 124, 124, 124, 124, 124, 124, 124, 124, 124, 124, 124, 124, 124, 124, 124, 124, 124, 124, 124, 124, 124, 124, 124, 124, 124, 124, 124, 124, 124, 124, 124, 124, 124, 124, 124, 124, 124, 124, 124, 124, 124, 124, 124, 255, 192, 0, 17, 8, 0, 60, 0, 100, 3, 1, 34, 0, 2, 17, 1, 3, 17, 1, 255, 196, 0, 22, 0, 1, 1, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 2, 5, 255, 196, 0, 21, 16, 1, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 17, 255, 196, 0, 23, 1, 1, 1, 1, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 4, 5, 6, 255, 196, 0, 23, 17, 1, 1, 1, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 19, 1, 18, 255, 218, 0, 12, 3, 1, 0, 2, 17, 3, 17, 0, 63, 0, 201, 165, 77, 42, 121, 58, 154, 42, 149, 52, 164, 138, 42, 149, 52, 164, 138, 42, 149, 52, 164, 138, 42, 149, 52, 164, 138, 42, 149, 52, 164, 138, 42, 137, 161, 34, 136, 165, 69, 43, 74, 72, 40, 186, 84, 82, 146, 40, 186, 84, 82, 146, 40, 186, 84, 82, 146, 40, 186, 84, 82, 146, 40, 186, 84, 82, 146, 40, 186, 34, 132, 138, 38, 149, 20, 173, 25, 32, 162, 233, 81, 74, 72, 162, 233, 81, 74, 72, 162, 233, 81, 74, 72, 162, 233, 81, 74, 72, 162, 233, 81, 74, 72, 162, 232, 138, 18, 40, 154, 84, 141, 25, 226, 14, 245, 84, 169, 9, 225, 222, 170, 149, 33, 60, 59, 213, 82, 164, 39, 135, 122, 170, 84, 132, 240, 239, 85, 74, 144, 158, 29, 234, 168, 144, 158, 29, 235, 255, 217];

    #[test]
    fn a_preview_shrinks_to_fit_the_side() {
        let (width, height, pixels) = decode_thumbnail(GRADIENT).expect("decodes");
        assert!(width <= THUMB_SIDE && height <= THUMB_SIDE);
        assert_eq!((width, height), (33, 20));
        assert_eq!(pixels.len(), width * height * 2);
        // Red at the top, blue at the bottom.
        let pixel = |row: usize| u16::from_le_bytes([pixels[row * width * 2], pixels[row * width * 2 + 1]]);
        assert!(pixel(0) >> 11 > pixel(height - 1) >> 11);
        assert!(pixel(0) & 31 < pixel(height - 1) & 31);
    }

    #[test]
    fn something_that_is_not_a_jpeg_has_no_preview() {
        assert!(decode_thumbnail(b"not a picture").is_none());
    }

    #[test]
    fn a_text_message_has_no_media() {
        assert!(media_of(&wa::Message::text("oi")).is_none());
    }

    #[test]
    fn a_document_shows_its_file_name() {
        let message = wa::Message {
            document_message: wa::message::DocumentMessage { file_name: Some("contrato.pdf".into()), ..Default::default() }.into(),
            ..Default::default()
        };
        let media = media_of(&message).expect("document");
        assert_eq!((media.kind, media.label.as_str()), (DOCUMENT, "contrato.pdf"));
    }
}
