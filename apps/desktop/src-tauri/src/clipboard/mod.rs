//! Clipboard capture and monitoring.
//!
//! Layering, from the bottom up:
//! - `platform` answers questions *about* the clipboard (change counter, concealed
//!   markers, source app) without reading the payload.
//! - `capture` reads the payload via `arboard` and prepares it for storage.
//! - `monitor` owns the background thread that ties the two together.
//!
//! `arboard` is only referenced inside `capture`, so replacing it stays a one-file
//! change.

pub mod capture;
pub mod monitor;
mod platform;
pub mod types;

pub use monitor::{MonitorState, remove_files, spawn};

use arboard::Clipboard;

use crate::error::{Error, Result};

/// Writes text to the system clipboard.
///
/// Returns the content hash so the caller can tell the monitor to ignore the echo.
pub fn write_text(text: &str) -> Result<String> {
    let mut clipboard =
        Clipboard::new().map_err(|e| Error::Other(format!("clipboard unavailable: {e}")))?;
    clipboard
        .set_text(text)
        .map_err(|e| Error::Other(format!("could not write to clipboard: {e}")))?;

    Ok(blake3::hash(text.as_bytes()).to_hex().to_string())
}

/// Writes a previously captured image file back to the system clipboard.
pub fn write_image(path: &str) -> Result<String> {
    let decoded = image::open(path)
        .map_err(|e| Error::Other(format!("could not read stored image: {e}")))?
        .to_rgba8();
    let (width, height) = decoded.dimensions();
    let bytes = decoded.into_raw();
    let hash = blake3::hash(&bytes).to_hex().to_string();

    let mut clipboard =
        Clipboard::new().map_err(|e| Error::Other(format!("clipboard unavailable: {e}")))?;
    clipboard
        .set_image(arboard::ImageData {
            width: width as usize,
            height: height as usize,
            bytes: bytes.into(),
        })
        .map_err(|e| Error::Other(format!("could not write image to clipboard: {e}")))?;

    Ok(hash)
}
