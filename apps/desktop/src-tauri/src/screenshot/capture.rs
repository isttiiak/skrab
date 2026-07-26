use std::path::{Path, PathBuf};

use image::RgbaImage;
use xcap::{Monitor, Window};

use super::types::{CaptureMode, CaptureRegion, MonitorInfo, WindowInfo};
use crate::error::{Error, Result};

/// A capture that has been written to disk, ready to be recorded in the database.
pub struct Capture {
    pub path: PathBuf,
    pub width: u32,
    pub height: u32,
    pub mode: CaptureMode,
}

/// Wraps an `xcap` failure with the macOS permission hint.
///
/// On macOS a missing Screen Recording grant surfaces as an ordinary capture error,
/// which is impossible to act on. The permission also only takes effect after a
/// relaunch, and in development every rebuild changes the binary path and resets it —
/// so the message says both things.
fn capture_error(context: &str, error: impl std::fmt::Display) -> Error {
    if cfg!(target_os = "macos") {
        Error::Other(format!(
            "{context}: {error}. On macOS this usually means Screen Recording \
             permission is missing — grant it in System Settings → Privacy & Security \
             → Screen Recording, then quit and reopen Skrab (the permission only \
             applies on a fresh launch)."
        ))
    } else {
        Error::Other(format!("{context}: {error}"))
    }
}

pub fn list_monitors() -> Result<Vec<MonitorInfo>> {
    let monitors = Monitor::all().map_err(|e| capture_error("could not list displays", e))?;

    monitors
        .into_iter()
        .map(|m| {
            Ok(MonitorInfo {
                id: m.id().unwrap_or(0),
                name: m.name().unwrap_or_else(|_| "Display".to_owned()),
                x: m.x().unwrap_or(0),
                y: m.y().unwrap_or(0),
                width: m.width().unwrap_or(0),
                height: m.height().unwrap_or(0),
                is_primary: m.is_primary().unwrap_or(false),
                scale: m.scale_factor().unwrap_or(1.0),
            })
        })
        .collect()
}

pub fn list_windows() -> Result<Vec<WindowInfo>> {
    let windows = Window::all().map_err(|e| capture_error("could not list windows", e))?;

    Ok(windows
        .into_iter()
        .filter_map(|w| {
            let title = w.title().unwrap_or_default();
            let width = w.width().unwrap_or(0);
            let height = w.height().unwrap_or(0);

            // Skip minimised windows and untitled scratch surfaces — they cannot be
            // captured usefully and would clutter the picker.
            if w.is_minimized().unwrap_or(false) || title.trim().is_empty() {
                return None;
            }
            if width == 0 || height == 0 {
                return None;
            }

            Some(WindowInfo {
                id: w.id().unwrap_or(0),
                title,
                app_name: w.app_name().unwrap_or_default(),
                width,
                height,
            })
        })
        .collect())
}

/// Captures one display. `monitor_id` of `None` means the primary display.
pub fn capture_monitor(dir: &Path, monitor_id: Option<u32>) -> Result<Capture> {
    let monitors = Monitor::all().map_err(|e| capture_error("could not list displays", e))?;

    let monitor = match monitor_id {
        Some(id) => monitors
            .into_iter()
            .find(|m| m.id().unwrap_or(0) == id)
            .ok_or_else(|| Error::Other(format!("display {id} is no longer connected")))?,
        None => monitors
            .into_iter()
            .find(|m| m.is_primary().unwrap_or(false))
            .ok_or_else(|| Error::Other("no primary display found".into()))?,
    };

    let image = monitor
        .capture_image()
        .map_err(|e| capture_error("could not capture the display", e))?;

    save(dir, image, CaptureMode::Fullscreen)
}

pub fn capture_window(dir: &Path, window_id: u32) -> Result<Capture> {
    let window = Window::all()
        .map_err(|e| capture_error("could not list windows", e))?
        .into_iter()
        .find(|w| w.id().unwrap_or(0) == window_id)
        .ok_or_else(|| Error::Other("that window has closed".into()))?;

    let image = window
        .capture_image()
        .map_err(|e| capture_error("could not capture the window", e))?;

    save(dir, image, CaptureMode::Window)
}

/// Captures a rectangle of the virtual desktop.
///
/// The region arrives in physical pixels relative to the whole desktop, so the
/// display it lands on is resolved first and the rectangle is rebased into that
/// display's own coordinates before cropping.
pub fn capture_region(dir: &Path, region: CaptureRegion) -> Result<Capture> {
    if !region.is_usable() {
        return Err(Error::Other("the selected region is empty".into()));
    }

    let monitors = Monitor::all().map_err(|e| capture_error("could not list displays", e))?;

    // Pick the display containing the region's origin, falling back to the primary.
    let monitor = monitors
        .iter()
        .find(|m| {
            let (mx, my) = (m.x().unwrap_or(0), m.y().unwrap_or(0));
            let (mw, mh) = (
                m.width().unwrap_or(0) as i32,
                m.height().unwrap_or(0) as i32,
            );
            region.x >= mx && region.x < mx + mw && region.y >= my && region.y < my + mh
        })
        .or_else(|| monitors.iter().find(|m| m.is_primary().unwrap_or(false)))
        .ok_or_else(|| Error::Other("no display contains that region".into()))?;

    let full = monitor
        .capture_image()
        .map_err(|e| capture_error("could not capture the display", e))?;

    let local = CaptureRegion {
        x: region.x - monitor.x().unwrap_or(0),
        y: region.y - monitor.y().unwrap_or(0),
        width: region.width,
        height: region.height,
    };

    let (x, y, w, h) = local
        .clamped_to(full.width(), full.height())
        .ok_or_else(|| Error::Other("the selected region is outside the display".into()))?;

    let cropped = image::imageops::crop_imm(&full, x, y, w, h).to_image();
    save(dir, cropped, CaptureMode::Region)
}

fn save(dir: &Path, image: RgbaImage, mode: CaptureMode) -> Result<Capture> {
    std::fs::create_dir_all(dir)?;
    let path = dir.join(format!("{}.png", uuid::Uuid::new_v4()));
    let (width, height) = image.dimensions();

    image
        .save(&path)
        .map_err(|e| Error::Other(format!("could not save the screenshot: {e}")))?;

    Ok(Capture {
        path,
        width,
        height,
        mode,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn save_writes_a_png_with_the_right_dimensions() {
        let dir = std::env::temp_dir().join(format!("skrab-shot-{}", uuid::Uuid::new_v4()));
        let image = RgbaImage::from_pixel(64, 32, image::Rgba([10, 120, 200, 255]));

        let capture = save(&dir, image, CaptureMode::Region).expect("saves");

        assert_eq!((capture.width, capture.height), (64, 32));
        assert_eq!(capture.mode, CaptureMode::Region);
        assert!(capture.path.exists());

        let reopened = image::open(&capture.path).expect("valid png");
        assert_eq!(reopened.width(), 64);

        std::fs::remove_dir_all(&dir).ok();
    }

    /// Not run by default: it needs a real display and, on macOS, the Screen
    /// Recording grant — neither of which a headless CI runner has. Run manually
    /// with `cargo test -- --ignored capture_the_primary_display` when verifying a
    /// new macOS release.
    #[test]
    #[ignore = "requires a display and macOS Screen Recording permission"]
    fn capture_the_primary_display() {
        // Set SKRAB_SPIKE_DIR to keep the output — the point of running this by hand
        // is usually to *look* at the image, and macOS hands back a degraded frame
        // (desktop picture, no windows) when Screen Recording permission is missing,
        // which is indistinguishable from success unless you inspect it.
        let keep = std::env::var("SKRAB_SPIKE_DIR").ok();
        let dir = match &keep {
            Some(path) => PathBuf::from(path),
            None => std::env::temp_dir().join(format!("skrab-spike-{}", uuid::Uuid::new_v4())),
        };

        let capture = capture_monitor(&dir, None).expect("primary display captures");

        assert!(capture.width > 0 && capture.height > 0);
        assert!(capture.path.exists());
        println!(
            "captured {}x{} to {}",
            capture.width,
            capture.height,
            capture.path.display()
        );

        if keep.is_none() {
            std::fs::remove_dir_all(&dir).ok();
        }
    }
}
