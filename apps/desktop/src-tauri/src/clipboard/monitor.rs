use std::sync::atomic::{AtomicBool, AtomicI64, Ordering};
use std::time::Duration;

use parking_lot::Mutex;
use tauri::{AppHandle, Emitter, Manager};

use super::capture::{Capture, capture};
use super::platform;
use super::types::CLIP_ADDED_EVENT;
use crate::db::{Database, queries};
use crate::settings::SettingsState;

/// Shared control surface for the background clipboard watcher.
pub struct MonitorState {
    enabled: AtomicBool,
    last_sequence: AtomicI64,
    /// Hash of the last value *we* put on the clipboard, so pasting from history
    /// does not immediately re-capture itself.
    self_written_hash: Mutex<Option<String>>,
}

impl MonitorState {
    pub fn new(enabled: bool) -> Self {
        Self {
            enabled: AtomicBool::new(enabled),
            // Seed with the current value so the clip already on the clipboard at
            // launch is not captured as if it were just copied.
            last_sequence: AtomicI64::new(platform::sequence()),
            self_written_hash: Mutex::new(None),
        }
    }

    pub fn set_enabled(&self, value: bool) {
        // Re-baseline on resume, otherwise everything copied while monitoring was
        // off would land in one burst the moment it is switched back on.
        if value && !self.enabled.load(Ordering::Relaxed) {
            self.last_sequence
                .store(platform::sequence(), Ordering::Relaxed);
        }
        self.enabled.store(value, Ordering::Relaxed);
    }

    pub fn is_enabled(&self) -> bool {
        self.enabled.load(Ordering::Relaxed)
    }

    /// Records that the app itself just wrote `hash` to the clipboard.
    pub fn note_self_write(&self, hash: String) {
        *self.self_written_hash.lock() = Some(hash);
    }

    /// Returns true (and consumes the marker) if `hash` is our own write echoing back.
    fn take_if_self_written(&self, hash: &str) -> bool {
        let mut guard = self.self_written_hash.lock();
        if guard.as_deref() == Some(hash) {
            *guard = None;
            true
        } else {
            false
        }
    }
}

/// Spawns the watcher thread.
///
/// The loop samples the OS change counter — a single integer read that neither takes
/// clipboard ownership nor touches the payload — and only reads the actual contents
/// when that counter moves. Polling the *contents* on a timer is what makes other
/// clipboard managers burn battery and break paste in other apps.
pub fn spawn(app: AppHandle) {
    std::thread::Builder::new()
        .name("skrab-clipboard-monitor".into())
        .spawn(move || run(app))
        .map(|_| ())
        .unwrap_or_else(|e| log::error!("could not start the clipboard monitor: {e}"));
}

fn run(app: AppHandle) {
    log::info!("clipboard monitor started");

    loop {
        let interval = {
            let settings = app.state::<SettingsState>();
            let guard = settings.read();
            Duration::from_millis(guard.poll_interval_ms)
        };
        std::thread::sleep(interval);

        let monitor = app.state::<MonitorState>();
        if !monitor.is_enabled() {
            continue;
        }

        let sequence = platform::sequence();
        if sequence == monitor.last_sequence.load(Ordering::Relaxed) {
            continue;
        }
        monitor.last_sequence.store(sequence, Ordering::Relaxed);

        if let Err(e) = handle_change(&app) {
            log::error!("clipboard capture failed: {e}");
        }
    }
}

fn handle_change(app: &AppHandle) -> crate::error::Result<()> {
    let settings = {
        let state = app.state::<SettingsState>();
        let guard = state.read();
        guard.clone()
    };

    let clips_dir = app
        .path()
        .app_data_dir()
        .map_err(|_| crate::error::Error::NoAppDataDir)?
        .join("clips");

    match capture(&clips_dir, &settings)? {
        Capture::Skipped(reason) => {
            // Never log the payload — the whole point is that it was sensitive.
            log::debug!("skipped clip: {}", reason.as_str());
            Ok(())
        }
        Capture::Unsupported => Ok(()),
        Capture::Clip(clip) => {
            let monitor = app.state::<MonitorState>();
            if monitor.take_if_self_written(&clip.content_hash) {
                log::debug!("ignored our own clipboard write");
                return Ok(());
            }

            let db = app.state::<Database>();
            let outcome = db.with(|conn| queries::insert_clip(conn, &clip))?;
            let doomed = db.with(|conn| queries::purge(conn, &settings))?;
            remove_files(&doomed);

            log::debug!("captured clip: {outcome:?}");
            // The panel refreshes from the database rather than trusting a payload in
            // the event, so the event itself is just a nudge.
            app.emit(CLIP_ADDED_EVENT, ())
                .unwrap_or_else(|e| log::error!("could not emit clip event: {e}"));
            Ok(())
        }
    }
}

/// Best-effort cleanup of image files whose rows were purged.
pub fn remove_files(paths: &[String]) {
    for path in paths {
        if let Err(e) = std::fs::remove_file(path)
            && e.kind() != std::io::ErrorKind::NotFound
        {
            log::warn!("could not remove purged image {path}: {e}");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn self_written_marker_is_consumed_once() {
        let state = MonitorState::new(true);
        state.note_self_write("abc".to_owned());

        assert!(state.take_if_self_written("abc"), "first check matches");
        assert!(
            !state.take_if_self_written("abc"),
            "marker must not suppress a genuine second copy of the same text"
        );
    }

    #[test]
    fn a_different_hash_is_not_suppressed() {
        let state = MonitorState::new(true);
        state.note_self_write("abc".to_owned());
        assert!(!state.take_if_self_written("xyz"));
    }

    #[test]
    fn disabled_monitor_reports_disabled() {
        let state = MonitorState::new(false);
        assert!(!state.is_enabled());
        state.set_enabled(true);
        assert!(state.is_enabled());
    }
}
