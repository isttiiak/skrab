use parking_lot::{RwLock, RwLockReadGuard};
use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::error::Result;

/// In-memory settings, shared with the monitor thread.
///
/// The watcher consults these every tick. Reading them from SQLite each time would
/// take the database lock several times a second for values that change rarely, so
/// the authoritative copy lives here and the database is written through on save.
pub struct SettingsState(RwLock<AppSettings>);

impl SettingsState {
    pub fn new(settings: AppSettings) -> Self {
        Self(RwLock::new(settings))
    }

    pub fn read(&self) -> RwLockReadGuard<'_, AppSettings> {
        self.0.read()
    }

    pub fn replace(&self, settings: AppSettings) {
        *self.0.write() = settings;
    }
}

/// The key under which the whole settings blob lives in the `settings` table.
const SETTINGS_KEY: &str = "app_settings";

/// User preferences.
///
/// Stored as one JSON row rather than a column per setting: the shape changes often
/// during development, and `serde`'s defaults mean an older database missing a newer
/// field still loads cleanly instead of needing a migration per preference.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "AppSettings.ts")]
#[serde(rename_all = "camelCase", default)]
pub struct AppSettings {
    /// Master switch for the clipboard watcher.
    pub monitoring_enabled: bool,
    /// How often the change counter is sampled. Lower feels snappier, costs more.
    #[ts(type = "number")]
    pub poll_interval_ms: u64,
    /// Hard cap on stored clips. Favorites and pins are exempt from purging.
    pub max_items: u32,
    /// Delete non-favorite clips older than this. `0` means keep forever.
    pub retention_days: u32,
    /// Skip clips copied from these apps (matched case-insensitively, substring).
    pub blocked_apps: Vec<String>,
    /// Also skip clips that look like secrets even without an OS marker.
    pub skip_secret_patterns: bool,
    /// `system` | `light` | `dark`
    pub theme: String,
    pub launch_at_login: bool,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            monitoring_enabled: true,
            poll_interval_ms: 250,
            max_items: 1000,
            retention_days: 30,
            blocked_apps: Vec::new(),
            skip_secret_patterns: true,
            theme: "system".to_owned(),
            launch_at_login: false,
        }
    }
}

impl AppSettings {
    /// Clamps user-supplied values into ranges the app can actually honor.
    ///
    /// The frontend already constrains these, but settings also arrive from a JSON
    /// blob on disk that a user can edit — a `poll_interval_ms` of 0 would spin a
    /// core forever.
    pub fn sanitized(mut self) -> Self {
        self.poll_interval_ms = self.poll_interval_ms.clamp(100, 5_000);
        self.max_items = self.max_items.clamp(50, 100_000);
        self.retention_days = self.retention_days.min(3650);
        if !matches!(self.theme.as_str(), "system" | "light" | "dark") {
            self.theme = "system".to_owned();
        }
        self
    }
}

pub fn load(conn: &Connection) -> Result<AppSettings> {
    let raw: Option<String> = conn
        .query_row(
            "SELECT value FROM settings WHERE key = ?1",
            [SETTINGS_KEY],
            |row| row.get(0),
        )
        .ok();

    let settings = match raw {
        Some(json) => serde_json::from_str::<AppSettings>(&json).unwrap_or_else(|e| {
            log::warn!("settings JSON is unreadable ({e}); falling back to defaults");
            AppSettings::default()
        }),
        None => AppSettings::default(),
    };

    Ok(settings.sanitized())
}

pub fn save(conn: &Connection, settings: &AppSettings) -> Result<()> {
    let json = serde_json::to_string(settings)
        .map_err(|e| crate::error::Error::Other(format!("could not serialize settings: {e}")))?;

    conn.execute(
        "INSERT INTO settings(key, value) VALUES (?1, ?2)
         ON CONFLICT(key) DO UPDATE SET value = excluded.value",
        rusqlite::params![SETTINGS_KEY, json],
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sanitize_clamps_a_zero_poll_interval() {
        let s = AppSettings {
            poll_interval_ms: 0,
            ..Default::default()
        }
        .sanitized();
        assert_eq!(s.poll_interval_ms, 100);
    }

    #[test]
    fn sanitize_rejects_an_unknown_theme() {
        let s = AppSettings {
            theme: "neon".to_owned(),
            ..Default::default()
        }
        .sanitized();
        assert_eq!(s.theme, "system");
    }

    #[test]
    fn missing_fields_fall_back_to_defaults() {
        // An older database will not have newer keys; deserializing must not fail.
        let s: AppSettings = serde_json::from_str(r#"{"maxItems": 42}"#).unwrap();
        assert_eq!(s.max_items, 42);
        assert!(s.monitoring_enabled, "unset fields take the default");
    }
}
