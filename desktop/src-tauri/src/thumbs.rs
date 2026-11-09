//! Small previews of photos you're about to send: shown in Send to, and sent with the request so
//! the other device sees what's coming (`wire::Thumb`). JPEG, PNG and WebP only; anything else,
//! and any photo that can't be read, just keeps its icon. Video frames come from Android only:
//! the desktop has no video decoder.

use std::io::Cursor;
use std::path::Path;

use image::{ImageReader, Limits, codecs::jpeg::JpegEncoder};
use nhako_core::wire::{THUMB_BYTES_MAX, THUMBS_MAX, Thumb};

/// The long side of a preview, in pixels.
const SIDE: u32 = 160;
/// Photos bigger than this aren't read for a preview: decoding one would take too long.
const FILE_MAX: u64 = 60 * 1024 * 1024;

/// Previews for up to [`THUMBS_MAX`] of `files` (path and mime), by index into `files`.
pub fn make(files: &[(&Path, &str, u64)]) -> Vec<(usize, Thumb)> {
    files
        .iter()
        .enumerate()
        .filter(|(_, (_, mime, size))| matches!(*mime, "image/jpeg" | "image/png" | "image/webp") && *size <= FILE_MAX)
        .filter_map(|(i, (path, _, _))| one(path).map(|t| (i, t)))
        .take(THUMBS_MAX)
        .collect()
}

/// One preview, or `None` if the photo can't be read or won't shrink under the size limit.
pub fn one(path: &Path) -> Option<Thumb> {
    let mut reader = ImageReader::open(path).ok()?.with_guessed_format().ok()?;
    let mut limits = Limits::default();
    // A 100 megapixel photo at most: a bigger one is likely a trick, and slow anyway.
    limits.max_image_width = Some(12_000);
    limits.max_image_height = Some(12_000);
    limits.max_alloc = Some(512 * 1024 * 1024);
    reader.limits(limits);
    let small = reader.decode().ok()?.thumbnail(SIDE, SIDE).to_rgb8();
    let (w, h) = small.dimensions();
    for quality in [72, 55, 40] {
        let mut jpeg = Vec::new();
        JpegEncoder::new_with_quality(Cursor::new(&mut jpeg), quality).encode_image(&small).ok()?;
        if jpeg.len() <= THUMB_BYTES_MAX {
            let t = Thumb {
                file: 0,
                w: w as u16,
                h: h as u16,
                jpeg,
            };
            return t.looks_valid().then_some(t);
        }
    }
    None
}

/// The preview as the page shows it.
pub fn data_url(t: &Thumb) -> String {
    format!("data:image/jpeg;base64,{}", data_encoding::BASE64.encode(&t.jpeg))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_photo_becomes_a_small_jpeg_and_junk_keeps_its_icon() {
        let dir = std::env::temp_dir().join(format!("nhako-thumbs-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let photo = dir.join("big.png");
        image::RgbImage::from_fn(1200, 800, |x, y| image::Rgb([(x % 256) as u8, (y % 256) as u8, 90]))
            .save(&photo)
            .unwrap();
        let junk = dir.join("fake.jpg");
        std::fs::write(&junk, b"not a photo").unwrap();

        let t = one(&photo).unwrap();
        assert_eq!((t.w, t.h), (160, 107));
        assert!(t.jpeg.len() <= THUMB_BYTES_MAX && t.looks_valid());
        assert!(one(&junk).is_none());

        let made = make(&[(&junk, "image/jpeg", 11), (&photo, "image/png", 1), (&photo, "video/mp4", 1)]);
        assert_eq!(made.iter().map(|(i, _)| *i).collect::<Vec<_>>(), vec![1]);
        assert!(data_url(&t).starts_with("data:image/jpeg;base64,/9j/"));
        std::fs::remove_dir_all(dir).unwrap();
    }
}
