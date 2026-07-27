use std::path::{Path, PathBuf};

use arboard::Clipboard;
use image::{ImageEncoder, RgbaImage};

use super::platform;
use super::types::{ClipType, NewClip};
use crate::error::{Error, Result};
use crate::security::{self, Rejection};
use crate::settings::AppSettings;

/// Longest edge of a list thumbnail, in pixels.
const THUMB_MAX_EDGE: u32 = 128;
/// JPEG quality for thumbnails — 70 is visually fine at this size and stays tiny.
const THUMB_QUALITY: u8 = 70;
/// Characters kept for the list preview.
const PREVIEW_CHARS: usize = 200;

/// What a single read of the clipboard produced.
pub enum Capture {
    Clip(Box<NewClip>),
    Skipped(Rejection),
    /// Clipboard held nothing we understand (e.g. a proprietary app format).
    Unsupported,
}

/// Reads the current clipboard and prepares it for storage.
///
/// Called only after the platform change counter moved, never on a timer.
pub fn capture(clips_dir: &Path, settings: &AppSettings) -> Result<Capture> {
    let os_concealed = platform::is_concealed();
    let source_app = platform::frontmost_app();

    let mut clipboard =
        Clipboard::new().map_err(|e| Error::Other(format!("clipboard unavailable: {e}")))?;

    // Text first: it is the overwhelmingly common case, and many apps put a text
    // fallback alongside richer formats.
    if let Ok(text) = clipboard.get_text()
        && !text.is_empty()
    {
        if let Err(rejection) =
            security::screen_text(&text, source_app.as_deref(), os_concealed, settings)
        {
            return Ok(Capture::Skipped(rejection));
        }
        return Ok(Capture::Clip(Box::new(text_clip(text, source_app))));
    }

    if let Ok(image) = clipboard.get_image() {
        if let Err(rejection) =
            security::screen_image(source_app.as_deref(), os_concealed, settings)
        {
            return Ok(Capture::Skipped(rejection));
        }
        return Ok(Capture::Clip(Box::new(image_clip(
            image, clips_dir, source_app,
        )?)));
    }

    Ok(Capture::Unsupported)
}

fn text_clip(text: String, source_app: Option<String>) -> NewClip {
    // The pasteboard flags tell us the payload is *also* available as HTML or RTF.
    // We store the plain-text rendering either way — it is what gets pasted — but
    // the type drives the icon and the type filter in the UI.
    let clip_type = if platform::has_html() {
        ClipType::Html
    } else if platform::has_rtf() {
        ClipType::Rtf
    } else {
        ClipType::Text
    };

    NewClip {
        clip_type,
        preview: preview_of(&text),
        content_hash: blake3::hash(text.as_bytes()).to_hex().to_string(),
        size_bytes: text.len() as i64,
        content: Some(text),
        image_path: None,
        thumb: None,
        source_app,
    }
}

fn image_clip(
    image: arboard::ImageData<'_>,
    clips_dir: &Path,
    source_app: Option<String>,
) -> Result<NewClip> {
    let width = image.width as u32;
    let height = image.height as u32;

    let buffer = RgbaImage::from_raw(width, height, image.bytes.into_owned()).ok_or_else(|| {
        Error::Other("clipboard image dimensions did not match its buffer".into())
    })?;

    // Hash the pixels, not the encoded file: PNG encoding is not byte-stable, so
    // hashing the output would defeat dedup for identical screenshots.
    let content_hash = blake3::hash(buffer.as_raw()).to_hex().to_string();

    std::fs::create_dir_all(clips_dir)?;
    let path: PathBuf = clips_dir.join(format!("{}.png", uuid::Uuid::new_v4()));
    buffer
        .save(&path)
        .map_err(|e| Error::Other(format!("could not save clipboard image: {e}")))?;

    let size_bytes = std::fs::metadata(&path)
        .map(|m| m.len() as i64)
        .unwrap_or(0);

    Ok(NewClip {
        clip_type: ClipType::Image,
        content: None,
        preview: format!("Image · {width}×{height}"),
        image_path: Some(path.to_string_lossy().into_owned()),
        thumb: thumbnail(&buffer),
        content_hash,
        size_bytes,
        source_app,
    })
}

/// Downscaled JPEG for the list row. `None` if encoding fails — a missing thumbnail
/// degrades to a placeholder, which is not worth failing the whole capture over.
pub fn thumbnail(source: &RgbaImage) -> Option<Vec<u8>> {
    let (w, h) = source.dimensions();
    if w == 0 || h == 0 {
        return None;
    }

    let scale = f64::from(THUMB_MAX_EDGE) / f64::from(w.max(h));
    let (tw, th) = if scale >= 1.0 {
        (w, h)
    } else {
        (
            ((f64::from(w) * scale).round() as u32).max(1),
            ((f64::from(h) * scale).round() as u32).max(1),
        )
    };

    let resized = image::imageops::resize(source, tw, th, image::imageops::FilterType::Triangle);
    // JPEG has no alpha; flatten onto white so transparent screenshots don't go black.
    let rgb = image::DynamicImage::ImageRgba8(resized).to_rgb8();

    let mut out = Vec::new();
    let encoder = image::codecs::jpeg::JpegEncoder::new_with_quality(&mut out, THUMB_QUALITY);
    encoder
        .write_image(rgb.as_raw(), tw, th, image::ExtendedColorType::Rgb8)
        .ok()?;

    Some(out)
}

/// First `PREVIEW_CHARS` characters, whitespace-collapsed, for the list row.
///
/// Char-based rather than byte-based so a multi-byte character never gets sliced in
/// half, and collapsed so a copied code block does not render as a tall blank row.
fn preview_of(text: &str) -> String {
    let collapsed = text.split_whitespace().collect::<Vec<_>>().join(" ");
    if collapsed.chars().count() <= PREVIEW_CHARS {
        collapsed
    } else {
        let truncated: String = collapsed.chars().take(PREVIEW_CHARS).collect();
        format!("{truncated}…")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn preview_collapses_whitespace() {
        assert_eq!(preview_of("  hello \n\n  world \t "), "hello world");
    }

    #[test]
    fn preview_truncates_long_text_with_an_ellipsis() {
        let long = "a".repeat(500);
        let preview = preview_of(&long);
        assert_eq!(preview.chars().count(), PREVIEW_CHARS + 1);
        assert!(preview.ends_with('…'));
    }

    #[test]
    fn preview_never_splits_a_multibyte_character() {
        // 300 emoji: byte-slicing at 200 would land mid-character and panic.
        let text = "🎉".repeat(300);
        let preview = preview_of(&text);
        assert!(preview.starts_with('🎉'));
        assert_eq!(preview.chars().count(), PREVIEW_CHARS + 1);
    }

    #[test]
    fn thumbnail_scales_the_long_edge_down() {
        let image = RgbaImage::from_pixel(400, 200, image::Rgba([10, 120, 200, 255]));
        let bytes = thumbnail(&image).expect("thumbnail encodes");
        assert!(!bytes.is_empty());
        assert_eq!(&bytes[..2], &[0xFF, 0xD8], "JPEG SOI marker");
    }

    #[test]
    fn thumbnail_does_not_upscale_a_small_image() {
        let image = RgbaImage::from_pixel(16, 16, image::Rgba([0, 0, 0, 255]));
        assert!(thumbnail(&image).is_some());
    }

    #[test]
    fn identical_pixels_hash_identically() {
        let a = RgbaImage::from_pixel(8, 8, image::Rgba([1, 2, 3, 4]));
        let b = RgbaImage::from_pixel(8, 8, image::Rgba([1, 2, 3, 4]));
        assert_eq!(blake3::hash(a.as_raw()), blake3::hash(b.as_raw()));
    }
}
