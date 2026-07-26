mod key;
pub(crate) mod migrations;
pub mod queries;

use std::path::Path;

use parking_lot::Mutex;
use rusqlite::Connection;

use crate::error::{Error, Result};

/// SQLCipher reports a wrong key as a generic "not a database" error.
fn is_key_mismatch(error: &Error) -> bool {
    matches!(error, Error::Database(_)) && error.to_string().contains("file is not a database")
}

/// The application's single database handle, held in Tauri managed state.
///
/// SQLite serializes writes anyway; a mutex around one connection is simpler and
/// faster here than a pool, because every caller is a short command on the IPC
/// thread. If read contention ever shows up, this is the one place to change.
pub struct Database {
    conn: Mutex<Connection>,
}

impl Database {
    /// Opens (creating if needed) the encrypted database in the app data directory.
    pub fn open(app_data_dir: &Path) -> Result<Self> {
        std::fs::create_dir_all(app_data_dir)?;

        let db_path = app_data_dir.join("Skrab.db");
        let mut conn = Connection::open(&db_path)?;

        // The key must be applied before any other statement touches the file.
        let key = key::get_or_create(app_data_dir)?;
        conn.pragma_update(None, "key", &key)?;

        // The key is only proven wrong on the first statement that touches a page.
        // Translate SQLite's opaque "file is not a database" into something the user
        // can act on rather than letting it surface as a raw error.
        if let Err(e) = configure(&conn) {
            return Err(if is_key_mismatch(&e) {
                Error::DatabaseKeyMismatch(db_path.to_string_lossy().into_owned())
            } else {
                e
            });
        }

        migrations::run(&mut conn)?;

        log::info!("database ready at {}", db_path.display());
        Ok(Self {
            conn: Mutex::new(conn),
        })
    }

    /// Runs `f` with the connection locked. Keep the closure short.
    pub fn with<T>(&self, f: impl FnOnce(&Connection) -> Result<T>) -> Result<T> {
        f(&self.conn.lock())
    }
}

/// Pragmas tuned for a small, write-frequent, always-running desktop app.
fn configure(conn: &Connection) -> Result<()> {
    // WAL: readers never block the clipboard writer.
    conn.pragma_update(None, "journal_mode", "WAL")?;
    // NORMAL is the right durability trade in WAL — a crash can lose the last
    // few milliseconds of clipboard history, which is not worth an fsync per copy.
    conn.pragma_update(None, "synchronous", "NORMAL")?;
    conn.pragma_update(None, "foreign_keys", "ON")?;
    // Keep the WAL from growing without bound over a multi-day session.
    conn.pragma_update(None, "journal_size_limit", 6 * 1024 * 1024)?;
    conn.pragma_update(None, "busy_timeout", 5_000)?;
    Ok(())
}
