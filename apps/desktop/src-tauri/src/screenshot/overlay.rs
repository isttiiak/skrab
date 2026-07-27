use std::path::PathBuf;

use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager, WebviewUrl, WebviewWindowBuilder};
use ts_rs::TS;

use super::types::{CaptureMode, CaptureRegion};
use crate::db::{Database, queries};
use crate::error::{Error, Result};

pub const OVERLAY: &str = "capture-overlay";

/// The still frame the region overlay is drawn on top of.
///
/// Skrab captures the screen *first* and lets the user drag on the frozen image,
/// rather than drawing a transparent window over the live desktop. Three reasons:
/// the overlay can never appear in its own capture; window transparency would need
/// Tauri's `macos-private-api`; and what you select is exactly what you saw, even if
/// the screen changes mid-drag.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "OverlayFrame.ts")]
#[serde(rename_all = "camelCase")]
pub struct OverlayFrame {
    /// Absolute path to the frozen PNG, served through the asset protocol.
    pub path: String,
    pub width: u32,
    pub height: u32,
    /// Where the captured display sits on the virtual desktop.
    pub origin_x: i32,
    pub origin_y: i32,
    pub scale: f32,
}

/// Holds the frozen frame between opening the overlay and finishing the crop.
#[derive(Default)]
pub struct OverlayState(Mutex<Option<OverlayFrame>>);

impl OverlayState {
    pub fn frame(&self) -> Option<OverlayFrame> {
        self.0.lock().clone()
    }

    fn set(&self, frame: Option<OverlayFrame>) {
        *self.0.lock() = frame;
    }
}

fn frames_dir(app: &AppHandle) -> Result<PathBuf> {
    Ok(app
        .path()
        .app_data_dir()
        .map_err(|_| Error::NoAppDataDir)?
        .join("frames"))
}

pub fn screenshots_dir(app: &AppHandle) -> Result<PathBuf> {
    Ok(app
        .path()
        .app_data_dir()
        .map_err(|_| Error::NoAppDataDir)?
        .join("screenshots"))
}

/// Freezes the screen and opens the selection overlay over it.
pub fn open(app: &AppHandle) {
    if let Err(e) = try_open(app) {
        log::error!("could not start region capture: {e}");
        crate::notify(app, "Screenshot failed", &e.to_string());
    }
}

fn try_open(app: &AppHandle) -> Result<()> {
    // Get our own windows out of the shot before freezing the screen.
    crate::window::hide(app);
    if let Some(pins) = app.get_webview_window(crate::window::PINS) {
        let _ = pins.hide();
    }
    std::thread::sleep(std::time::Duration::from_millis(120));

    let (frame, monitor) = freeze(app)?;
    app.state::<OverlayState>().set(Some(frame));

    if let Some(existing) = app.get_webview_window(OVERLAY) {
        existing.show()?;
        existing.set_focus()?;
        return Ok(());
    }

    let window = WebviewWindowBuilder::new(
        app,
        OVERLAY,
        WebviewUrl::App("index.html?view=overlay".into()),
    )
    .title("Skrab · Select a region")
    .position(monitor.0 as f64, monitor.1 as f64)
    .inner_size(monitor.2, monitor.3)
    .decorations(false)
    .always_on_top(true)
    .skip_taskbar(true)
    .resizable(false)
    .build()?;

    window.set_focus()?;
    Ok(())
}

/// Captures the primary display to a temporary frame. Returns the frame and the
/// display's logical position/size for placing the overlay window.
fn freeze(app: &AppHandle) -> Result<(OverlayFrame, (i32, i32, f64, f64))> {
    let monitors =
        xcap::Monitor::all().map_err(|e| Error::Other(format!("could not list displays: {e}")))?;
    let monitor = monitors
        .into_iter()
        .find(|m| m.is_primary().unwrap_or(false))
        .ok_or_else(|| Error::Other("no primary display found".into()))?;

    let image = monitor
        .capture_image()
        .map_err(|e| super::capture::permission_hint("could not capture the screen", e))?;

    let dir = frames_dir(app)?;
    std::fs::create_dir_all(&dir)?;
    let path = dir.join("frozen.png");
    image
        .save(&path)
        .map_err(|e| Error::Other(format!("could not save the frame: {e}")))?;

    let scale = monitor.scale_factor().unwrap_or(1.0);
    let (origin_x, origin_y) = (monitor.x().unwrap_or(0), monitor.y().unwrap_or(0));

    // The window is placed in logical points; the capture is in physical pixels.
    let logical_w = f64::from(image.width()) / f64::from(scale);
    let logical_h = f64::from(image.height()) / f64::from(scale);

    Ok((
        OverlayFrame {
            path: path.to_string_lossy().into_owned(),
            width: image.width(),
            height: image.height(),
            origin_x,
            origin_y,
            scale,
        },
        (origin_x, origin_y, logical_w, logical_h),
    ))
}

pub fn close(app: &AppHandle) {
    app.state::<OverlayState>().set(None);
    if let Some(window) = app.get_webview_window(OVERLAY)
        && let Err(e) = window.hide()
    {
        log::error!("could not hide the capture overlay: {e}");
    }
}

/// Crops the frozen frame to `region` (in physical pixels of that frame) and stores it.
pub fn finish_region(app: &AppHandle, region: CaptureRegion) -> Result<super::capture::Capture> {
    let frame = app
        .state::<OverlayState>()
        .frame()
        .ok_or_else(|| Error::Other("the capture overlay is no longer open".into()))?;

    if !region.is_usable() {
        return Err(Error::Other("Drag to select an area first.".into()));
    }

    let (x, y, w, h) = region
        .clamped_to(frame.width, frame.height)
        .ok_or_else(|| Error::Other("that selection is outside the screen".into()))?;

    let full = image::open(&frame.path)
        .map_err(|e| Error::Other(format!("could not reopen the frame: {e}")))?
        .to_rgba8();

    let cropped = image::imageops::crop_imm(&full, x, y, w, h).to_image();
    super::capture::save_image(&screenshots_dir(app)?, cropped, CaptureMode::Region)
}

/// Captures the whole primary display immediately, with no overlay.
pub fn capture_fullscreen_now(app: &AppHandle) {
    let result = (|| -> Result<super::capture::Capture> {
        crate::window::hide(app);
        std::thread::sleep(std::time::Duration::from_millis(120));
        super::capture::capture_monitor(&screenshots_dir(app)?, None)
    })();

    match result {
        Ok(capture) => {
            record_and_copy(app, &capture);
        }
        Err(e) => {
            log::error!("fullscreen capture failed: {e}");
            crate::notify(app, "Screenshot failed", &e.to_string());
        }
    }
}

/// Stores the capture and puts it straight on the clipboard.
///
/// Copying immediately is the behaviour people expect from a screenshot hotkey —
/// the overwhelmingly common next action is pasting it somewhere.
pub fn record_and_copy(app: &AppHandle, capture: &super::capture::Capture) {
    let path = capture.path.to_string_lossy().into_owned();
    let db = app.state::<Database>();

    if let Err(e) = db.with(|conn| {
        queries::insert_screenshot(
            conn,
            &path,
            capture.mode.as_str(),
            capture.width,
            capture.height,
        )
    }) {
        log::error!("could not record the screenshot: {e}");
    }

    match crate::clipboard::write_image(&path) {
        Ok(hash) => {
            app.state::<crate::clipboard::MonitorState>()
                .note_self_write(hash);
            crate::notify(
                app,
                "Screenshot copied",
                &format!("{}×{} is on your clipboard", capture.width, capture.height),
            );
        }
        Err(e) => log::error!("could not copy the screenshot: {e}"),
    }
}
