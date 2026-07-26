use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager, State};
use ts_rs::TS;

use crate::clipboard::types::{ClipItem, ClipQuery};
use crate::clipboard::{self, MonitorState};
use crate::db::{Database, queries};
use crate::error::{Error, Result};
use crate::screenshot;
use crate::settings::{AppSettings, SettingsState};

// Generated TypeScript lands in `packages/ipc-types/src/generated/`. The destination
// is set once via `TS_RS_EXPORT_DIR` in `.cargo/config.toml`, so each type below only
// names its own file. Note that `///` doc comments are copied into the generated
// output — write them for the frontend reader, not for Rust internals.

/// Build and platform identity, shown in Settings and attached to bug reports.
#[derive(Debug, Serialize, Deserialize, TS)]
#[ts(export, export_to = "AppInfo.ts")]
#[serde(rename_all = "camelCase")]
pub struct AppInfo {
    pub name: String,
    pub version: String,
    pub tauri_version: String,
    pub os: String,
}

/// Counters for the settings screen.
#[derive(Debug, Serialize, Deserialize, TS)]
#[ts(export, export_to = "HistoryStats.ts")]
#[serde(rename_all = "camelCase")]
pub struct HistoryStats {
    // i64/u64 default to `bigint` in ts-rs, but Tauri's IPC delivers a plain JSON
    // number. These values (unix millis, byte counts) are far below 2^53, so the
    // annotation makes the generated type match what actually arrives.
    #[ts(type = "number")]
    pub total_clips: i64,
    pub monitoring: bool,
}

#[tauri::command]
pub fn get_app_info(app: AppHandle) -> AppInfo {
    let package = app.package_info();
    AppInfo {
        name: package.name.clone(),
        version: package.version.to_string(),
        tauri_version: tauri::VERSION.to_string(),
        os: std::env::consts::OS.to_string(),
    }
}

// ---------------------------------------------------------------- history

#[tauri::command]
pub fn list_clips(db: State<'_, Database>, query: ClipQuery) -> Result<Vec<ClipItem>> {
    db.with(|conn| queries::list_clips(conn, &query))
}

#[tauri::command]
pub fn get_clip_content(db: State<'_, Database>, id: String) -> Result<Option<String>> {
    db.with(|conn| queries::clip_content(conn, &id))
}

/// Puts a stored clip back on the system clipboard.
///
/// The monitor is told the hash beforehand so this write does not immediately come
/// back around as a "new" clip.
#[tauri::command]
pub fn copy_clip(
    db: State<'_, Database>,
    monitor: State<'_, MonitorState>,
    id: String,
) -> Result<()> {
    // An image clip has a file on disk; anything else round-trips as text.
    let image_path = db.with(|conn| queries::clip_image_path(conn, &id))?;

    let hash = match image_path {
        Some(path) => {
            log::info!("copying image clip {id} back to the clipboard");
            clipboard::write_image(&path)?
        }
        None => {
            let content = db
                .with(|conn| queries::clip_content(conn, &id))?
                .ok_or_else(|| {
                    Error::Other(format!("clip {id} has neither text nor an image on disk"))
                })?;
            log::info!("copying {} chars back to the clipboard", content.len());
            clipboard::write_text(&content)?
        }
    };

    monitor.note_self_write(hash);
    db.with(|conn| queries::touch(conn, &id))?;
    Ok(())
}

#[tauri::command]
pub fn set_clip_favorite(db: State<'_, Database>, id: String, value: bool) -> Result<()> {
    db.with(|conn| queries::set_favorite(conn, &id, value))
}

#[tauri::command]
pub fn set_clip_pinned(db: State<'_, Database>, id: String, value: bool) -> Result<()> {
    db.with(|conn| queries::set_pinned(conn, &id, value))
}

#[tauri::command]
pub fn delete_clip(db: State<'_, Database>, id: String) -> Result<()> {
    let orphan = db.with(|conn| queries::delete_clip(conn, &id))?;
    if let Some(path) = orphan {
        clipboard::remove_files(&[path]);
    }
    Ok(())
}

/// Deletes every clip except favorites and pins.
#[tauri::command]
pub fn clear_history(db: State<'_, Database>) -> Result<()> {
    let orphans = db.with(queries::clear_history)?;
    clipboard::remove_files(&orphans);
    Ok(())
}

#[tauri::command]
pub fn history_stats(
    db: State<'_, Database>,
    monitor: State<'_, MonitorState>,
) -> Result<HistoryStats> {
    Ok(HistoryStats {
        total_clips: db.with(queries::count_clips)?,
        monitoring: monitor.is_enabled(),
    })
}

// ---------------------------------------------------------------- settings

#[tauri::command]
pub fn get_settings(state: State<'_, SettingsState>) -> AppSettings {
    state.read().clone()
}

#[tauri::command]
pub fn save_settings(
    app: AppHandle,
    db: State<'_, Database>,
    state: State<'_, SettingsState>,
    monitor: State<'_, MonitorState>,
    settings: AppSettings,
) -> Result<AppSettings> {
    let settings = settings.sanitized();

    db.with(|conn| crate::settings::save(conn, &settings))?;
    monitor.set_enabled(settings.monitoring_enabled);
    apply_autostart(&app, settings.launch_at_login);
    state.replace(settings.clone());

    // A tighter retention or item cap should take effect immediately, not at the
    // next copy — otherwise "clear old items" appears to do nothing.
    let orphans = db.with(|conn| queries::purge(conn, &settings))?;
    clipboard::remove_files(&orphans);

    Ok(settings)
}

#[tauri::command]
pub fn set_monitoring(monitor: State<'_, MonitorState>, enabled: bool) {
    monitor.set_enabled(enabled);
}

// ---------------------------------------------------------------- smart paste

#[tauri::command]
pub fn toggle_pins_widget(app: AppHandle) {
    crate::window::toggle_pins(&app);
}

#[tauri::command]
pub fn close_pins_widget(app: AppHandle) -> Result<()> {
    if let Some(window) = app.get_webview_window(crate::window::PINS) {
        window.hide()?;
    }
    Ok(())
}

/// Copies a pinned clip and, if the user has opted in, pastes it into the app behind.
///
/// Returns whether a keystroke was actually sent, so the widget can tell the user
/// "copied" versus "pasted" honestly instead of claiming something it did not do.
#[tauri::command]
pub async fn paste_clip(
    app: AppHandle,
    db: State<'_, Database>,
    monitor: State<'_, MonitorState>,
    settings: State<'_, SettingsState>,
    id: String,
) -> Result<bool> {
    copy_clip(db, monitor, id)?;

    if !settings.read().auto_paste {
        return Ok(false);
    }

    // The keystroke has to land in the app the user was working in, so the widget
    // must lose focus first. Hiding and waiting is the only portable way to get
    // there — without the pause the paste races the window manager.
    if let Some(window) = app.get_webview_window(crate::window::PINS) {
        window.hide()?;
    }
    tokio::time::sleep(crate::input::FOCUS_SETTLE).await;

    crate::input::send_paste()?;

    if let Some(window) = app.get_webview_window(crate::window::PINS) {
        window.show()?;
    }
    Ok(true)
}

// ---------------------------------------------------------------- screenshots

/// A capture that has been taken and recorded.
#[derive(Debug, Serialize, Deserialize, TS)]
#[ts(export, export_to = "CaptureResult.ts")]
#[serde(rename_all = "camelCase")]
pub struct CaptureResult {
    pub id: String,
    /// Absolute path on disk. Render it through Tauri's asset protocol.
    pub path: String,
    pub width: u32,
    pub height: u32,
}

#[tauri::command]
pub fn list_monitors() -> Result<Vec<screenshot::MonitorInfo>> {
    screenshot::capture::list_monitors()
}

#[tauri::command]
pub fn list_capturable_windows() -> Result<Vec<screenshot::WindowInfo>> {
    screenshot::capture::list_windows()
}

/// Captures a display, a window, or a rectangle, and records it.
///
/// One command rather than three: the three modes differ only in which pixels they
/// select, and everything after that — save, record, hand the path back — is shared.
#[tauri::command]
pub fn capture_screen(
    app: AppHandle,
    db: State<'_, Database>,
    mode: screenshot::CaptureMode,
    monitor_id: Option<u32>,
    window_id: Option<u32>,
    region: Option<screenshot::CaptureRegion>,
) -> Result<CaptureResult> {
    let dir = app
        .path()
        .app_data_dir()
        .map_err(|_| Error::NoAppDataDir)?
        .join("screenshots");

    let capture = match mode {
        screenshot::CaptureMode::Fullscreen => screenshot::capture_monitor(&dir, monitor_id)?,
        screenshot::CaptureMode::Window => {
            let id =
                window_id.ok_or_else(|| Error::Other("window capture needs a window id".into()))?;
            screenshot::capture_window(&dir, id)?
        }
        screenshot::CaptureMode::Region => {
            let region =
                region.ok_or_else(|| Error::Other("region capture needs a rectangle".into()))?;
            screenshot::capture_region(&dir, region)?
        }
    };

    let path = capture.path.to_string_lossy().into_owned();
    let id = db.with(|conn| {
        queries::insert_screenshot(
            conn,
            &path,
            capture.mode.as_str(),
            capture.width,
            capture.height,
        )
    })?;

    log::info!(
        "captured {}x{} ({})",
        capture.width,
        capture.height,
        capture.mode.as_str()
    );

    Ok(CaptureResult {
        id,
        path,
        width: capture.width,
        height: capture.height,
    })
}

// ---------------------------------------------------------------- window

/// Hides the panel. Called when the user presses Escape.
#[tauri::command]
pub fn hide_panel(app: AppHandle) {
    crate::window::hide(&app);
}

#[tauri::command]
pub fn open_data_dir(app: AppHandle) -> Result<()> {
    let dir = app.path().app_data_dir().map_err(|_| Error::NoAppDataDir)?;
    tauri_plugin_opener::open_path(dir.to_string_lossy().to_string(), None::<&str>)
        .map_err(|e| Error::Other(format!("could not open the data folder: {e}")))
}

/// Registers or removes the login item to match the setting.
///
/// Persisting the preference is not enough — the OS keeps its own list, so the
/// toggle has to actually reach the autostart plugin. Failures are logged rather
/// than surfaced: the rest of the save succeeded, and a login item that could not be
/// written is not worth discarding the user's other changes over.
#[cfg(desktop)]
fn apply_autostart(app: &AppHandle, enabled: bool) {
    use tauri_plugin_autostart::ManagerExt;

    let manager = app.autolaunch();
    let result = if enabled {
        manager.enable()
    } else {
        manager.disable()
    };

    if let Err(e) = result {
        log::warn!("could not update the launch-at-login setting: {e}");
    }
}

#[cfg(not(desktop))]
fn apply_autostart(_app: &AppHandle, _enabled: bool) {}

/// Unix epoch milliseconds. The single time source for every persisted timestamp.
pub fn now_millis() -> i64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn millis_are_plausible() {
        // Sanity: after 2020-01-01 and before 2100.
        let now = now_millis();
        assert!(now > 1_577_836_800_000, "clock looks wrong: {now}");
        assert!(now < 4_102_444_800_000, "clock looks wrong: {now}");
    }
}
