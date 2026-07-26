use rusqlite::Connection;

use crate::error::Result;

/// Ordered, append-only migrations. Never edit a shipped entry — add a new one.
///
/// The index in this slice *is* the schema version, tracked in `PRAGMA user_version`.
const MIGRATIONS: &[&str] = &[
    // v1 — initial schema
    r#"
    CREATE TABLE clip_items (
        id           TEXT    PRIMARY KEY,
        clip_type    TEXT    NOT NULL CHECK (clip_type IN ('text','image','html','rtf','file')),
        content      TEXT,
        preview      TEXT    NOT NULL DEFAULT '',
        image_path   TEXT,
        thumb        BLOB,
        content_hash TEXT    NOT NULL,
        size_bytes   INTEGER NOT NULL DEFAULT 0,
        source_app   TEXT,
        is_pinned    INTEGER NOT NULL DEFAULT 0,
        is_favorite  INTEGER NOT NULL DEFAULT 0,
        is_concealed INTEGER NOT NULL DEFAULT 0,
        category     TEXT,
        created_at   INTEGER NOT NULL,
        accessed_at  INTEGER NOT NULL,
        updated_at   INTEGER NOT NULL,
        deleted_at   INTEGER
    );

    -- Dedup: the same bytes copied twice is one row, not two.
    CREATE UNIQUE INDEX idx_clip_items_hash ON clip_items(content_hash);

    -- The history panel's default query: newest first, excluding tombstones.
    CREATE INDEX idx_clip_items_created  ON clip_items(created_at DESC) WHERE deleted_at IS NULL;
    CREATE INDEX idx_clip_items_type     ON clip_items(clip_type)       WHERE deleted_at IS NULL;
    CREATE INDEX idx_clip_items_pinned   ON clip_items(is_pinned)       WHERE is_pinned = 1;
    CREATE INDEX idx_clip_items_favorite ON clip_items(is_favorite)     WHERE is_favorite = 1;

    -- Sync seam: pull everything changed since a cursor, tombstones included.
    CREATE INDEX idx_clip_items_updated  ON clip_items(updated_at);

    CREATE TABLE screenshots (
        id           TEXT    PRIMARY KEY,
        image_path   TEXT    NOT NULL,
        edited_path  TEXT,
        capture_mode TEXT    NOT NULL CHECK (capture_mode IN ('fullscreen','window','region')),
        width        INTEGER NOT NULL,
        height       INTEGER NOT NULL,
        created_at   INTEGER NOT NULL,
        updated_at   INTEGER NOT NULL,
        deleted_at   INTEGER
    );

    CREATE INDEX idx_screenshots_created ON screenshots(created_at DESC) WHERE deleted_at IS NULL;

    CREATE TABLE settings (
        key   TEXT PRIMARY KEY,
        value TEXT NOT NULL
    );
    "#,
    // v2 — full-text search over clip text, kept in step with clip_items by triggers
    r#"
    CREATE VIRTUAL TABLE clip_items_fts USING fts5(
        content,
        content = 'clip_items',
        content_rowid = 'rowid',
        tokenize = 'unicode61 remove_diacritics 2'
    );

    CREATE TRIGGER clip_items_fts_insert AFTER INSERT ON clip_items BEGIN
        INSERT INTO clip_items_fts(rowid, content) VALUES (new.rowid, new.content);
    END;

    CREATE TRIGGER clip_items_fts_delete AFTER DELETE ON clip_items BEGIN
        INSERT INTO clip_items_fts(clip_items_fts, rowid, content)
        VALUES ('delete', old.rowid, old.content);
    END;

    CREATE TRIGGER clip_items_fts_update AFTER UPDATE ON clip_items BEGIN
        INSERT INTO clip_items_fts(clip_items_fts, rowid, content)
        VALUES ('delete', old.rowid, old.content);
        INSERT INTO clip_items_fts(rowid, content) VALUES (new.rowid, new.content);
    END;
    "#,
];

/// Brings the database up to the latest schema version.
///
/// Each pending migration runs in its own transaction, so a failure leaves the
/// database at the last version that fully applied rather than half-migrated.
pub fn run(conn: &mut Connection) -> Result<()> {
    let current: usize =
        conn.query_row("PRAGMA user_version", [], |row| row.get::<_, i64>(0))? as usize;

    if current > MIGRATIONS.len() {
        log::warn!(
            "database schema v{current} is newer than this build understands (v{}); \
             refusing to downgrade",
            MIGRATIONS.len()
        );
        return Ok(());
    }

    for (index, migration) in MIGRATIONS.iter().enumerate().skip(current) {
        let version = index + 1;
        log::info!("applying database migration v{version}");

        let tx = conn.transaction()?;
        tx.execute_batch(migration)?;
        // PRAGMA can't be parameterized.
        tx.pragma_update(None, "user_version", version as i64)?;
        tx.commit()?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn migrated() -> Connection {
        let mut conn = Connection::open_in_memory().expect("in-memory db");
        run(&mut conn).expect("migrations apply cleanly");
        conn
    }

    #[test]
    fn migrations_apply_to_latest_version() {
        let conn = migrated();
        let version: i64 = conn
            .query_row("PRAGMA user_version", [], |r| r.get(0))
            .unwrap();
        assert_eq!(version as usize, MIGRATIONS.len());
    }

    #[test]
    fn migrations_are_idempotent() {
        let mut conn = migrated();
        // A second run on an already-current database must be a no-op, not an error.
        run(&mut conn).expect("re-running migrations is safe");
    }

    #[test]
    fn fts_index_follows_clip_items() {
        let conn = migrated();
        let now = 1_700_000_000_000i64;
        conn.execute(
            "INSERT INTO clip_items
               (id, clip_type, content, preview, content_hash, created_at, accessed_at, updated_at)
             VALUES (?1, 'text', ?2, ?2, ?3, ?4, ?4, ?4)",
            rusqlite::params!["id-1", "the quick brown fox", "hash-1", now],
        )
        .unwrap();

        let hits: i64 = conn
            .query_row(
                "SELECT count(*) FROM clip_items_fts WHERE clip_items_fts MATCH 'brown'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(hits, 1, "insert trigger should populate the FTS index");

        conn.execute("DELETE FROM clip_items WHERE id = 'id-1'", [])
            .unwrap();
        let hits: i64 = conn
            .query_row(
                "SELECT count(*) FROM clip_items_fts WHERE clip_items_fts MATCH 'brown'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(hits, 0, "delete trigger should clear the FTS index");
    }

    #[test]
    fn duplicate_content_hash_is_rejected() {
        let conn = migrated();
        let now = 1_700_000_000_000i64;
        let insert = |id: &str| {
            conn.execute(
                "INSERT INTO clip_items
                   (id, clip_type, content, preview, content_hash, created_at, accessed_at, updated_at)
                 VALUES (?1, 'text', 'x', 'x', 'same-hash', ?2, ?2, ?2)",
                rusqlite::params![id, now],
            )
        };

        insert("id-1").expect("first insert succeeds");
        insert("id-2").expect_err("the unique hash index must reject a duplicate");
    }
}
