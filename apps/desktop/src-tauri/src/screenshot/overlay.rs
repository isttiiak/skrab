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
    /// The frozen screen as a `data:` URI.
    ///
    /// Inlined rather than served through Tauri's asset protocol: that path depends
    /// on the protocol being enabled *and* the file falling inside a configured
    /// scope, and when either is wrong the overlay renders a blank window with no
    /// way to tell why. A data URI always renders or fails visibly.
    pub preview: String,
    /// Absolute path to the full-resolution frame, used for the actual crop.
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
    std::thread::sleep(std::time::Duration::from_millis(120));

    let (frame, monitor) = freeze(app)?;
    app.state::<OverlayState>().set(Some(frame));

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

    // A JPEG preview keeps the IPC payload to a few hundred KB; the crop still runs
    // against the full-resolution PNG on disk.
    let preview = encode_preview(&image)
        .ok_or_else(|| Error::Other("could not prepare the capture preview".into()))?;

    let scale = monitor.scale_factor().unwrap_or(1.0);
    let (origin_x, origin_y) = (monitor.x().unwrap_or(0), monitor.y().unwrap_or(0));

    // The window is placed in logical points; the capture is in physical pixels.
    let logical_w = f64::from(image.width()) / f64::from(scale);
    let logical_h = f64::from(image.height()) / f64::from(scale);

    Ok((
        OverlayFrame {
            preview,
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

/// Closes the overlay by destroying the window.
///
/// Destroyed rather than hidden so the next capture starts from a clean slate — a
/// hidden window keeps its React state, which meant the previous selection was still
/// drawn when the overlay reopened and the first click appeared to "clear" it.
pub fn close(app: &AppHandle) {
    app.state::<OverlayState>().set(None);
    if let Some(window) = app.get_webview_window(OVERLAY)
        && let Err(e) = window.destroy()
    {
        log::error!("could not close the capture overlay: {e}");
    }
}

/// Encodes the frame as a JPEG data URI for display in the overlay.
fn encode_preview(image: &image::RgbaImage) -> Option<String> {
    use base64::Engine as _;
    use image::ImageEncoder as _;

    let rgb = image::DynamicImage::ImageRgba8(image.clone()).to_rgb8();
    let mut bytes = Vec::new();
    image::codecs::jpeg::JpegEncoder::new_with_quality(&mut bytes, 82)
        .write_image(
            rgb.as_raw(),
            rgb.width(),
            rgb.height(),
            image::ExtendedColorType::Rgb8,
        )
        .ok()?;

    Some(format!(
        "data:image/jpeg;base64,{}",
        base64::engine::general_purpose::STANDARD.encode(&bytes)
    ))
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
            // Suppressing the monitor stops the screenshot echoing back as a copy,
            // but it also meant it never reached the history at all. Insert it
            // directly so captures show up in the list like any other image clip.
            app.state::<crate::clipboard::MonitorState>()
                .note_self_write(hash.clone());
            add_to_history(app, capture, &path, hash);
            crate::notify(
                app,
                "Screenshot copied",
                &format!("{}×{} is on your clipboard", capture.width, capture.height),
            );
        }
        Err(e) => log::error!("could not copy the screenshot: {e}"),
    }
}

/// Records a capture in the clipboard history so it appears in the panel.
///
/// The `screenshots` table keeps capture metadata, but the list the user actually
/// looks at is `clip_items` — a screenshot that only landed in the former was
/// invisible everywhere in the UI.
fn add_to_history(app: &AppHandle, capture: &super::capture::Capture, path: &str, hash: String) {
    let thumb = image::open(path)
        .ok()
        .map(|img| img.to_rgba8())
        .and_then(|img| crate::clipboard::capture::thumbnail(&img));

    let size_bytes = std::fs::metadata(path).map(|m| m.len() as i64).unwrap_or(0);

    let clip = crate::clipboard::types::NewClip {
        clip_type: crate::clipboard::types::ClipType::Image,
        content: None,
        preview: format!("Screenshot · {}×{}", capture.width, capture.height),
        image_path: Some(path.to_owned()),
        thumb,
        content_hash: hash,
        size_bytes,
        source_app: Some("Skrab".to_owned()),
    };

    let db = app.state::<Database>();
    match db.with(|conn| queries::insert_clip(conn, &clip)) {
        Ok(_) => {
            use tauri::Emitter as _;
            let _ = app.emit(crate::clipboard::types::CLIP_ADDED_EVENT, ());
        }
        Err(e) => log::error!("could not add the screenshot to history: {e}"),
    }
}
