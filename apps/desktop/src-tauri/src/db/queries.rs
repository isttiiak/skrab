use base64::Engine as _;
use base64::engine::general_purpose::STANDARD as BASE64;
use rusqlite::{Connection, OptionalExtension, Row, params};

use crate::clipboard::types::{ClipItem, ClipQuery, ClipType, NewClip};
use crate::commands::now_millis;
use crate::error::{Error, Result};
use crate::settings::AppSettings;

/// Columns every `ClipItem` mapping expects, in order.
const CLIP_COLUMNS: &str = "id, clip_type, preview, thumb, image_path, size_bytes, \
     source_app, is_pinned, is_favorite, category, created_at, accessed_at";

fn map_clip(row: &Row<'_>) -> rusqlite::Result<ClipItem> {
    let type_str: String = row.get(1)?;
    let thumb: Option<Vec<u8>> = row.get(3)?;

    Ok(ClipItem {
        id: row.get(0)?,
        // A row whose type is unreadable is a bug, not a user problem — surface it as
        // text rather than dropping the clip silently.
        clip_type: ClipType::from_str(&type_str).unwrap_or(ClipType::Text),
        preview: row.get(2)?,
        thumb: thumb.map(|bytes| format!("data:image/jpeg;base64,{}", BASE64.encode(bytes))),
        image_path: row.get(4)?,
        size_bytes: row.get(5)?,
        source_app: row.get(6)?,
        is_pinned: row.get::<_, i64>(7)? != 0,
        is_favorite: row.get::<_, i64>(8)? != 0,
        category: row.get(9)?,
        created_at: row.get(10)?,
        accessed_at: row.get(11)?,
    })
}

/// Result of trying to record a newly copied item.
#[derive(Debug, PartialEq, Eq)]
pub enum InsertOutcome {
    /// A new row was created.
    Inserted(String),
    /// The same bytes were already stored; the existing row moved back to the top.
    Promoted(String),
}

/// Stores a clip, or promotes the existing one if the content is a duplicate.
///
/// Re-copying something you copied yesterday should surface it at the top of the
/// list, not create a second identical row — so a hash collision on `content_hash`
/// updates timestamps instead of failing.
pub fn insert_clip(conn: &Connection, clip: &NewClip) -> Result<InsertOutcome> {
    let now = now_millis();

    let existing: Option<String> = conn
        .query_row(
            "SELECT id FROM clip_items WHERE content_hash = ?1",
            params![clip.content_hash],
            |row| row.get(0),
        )
        .optional()?;

    if let Some(id) = existing {
        conn.execute(
            "UPDATE clip_items
                SET created_at = ?2, accessed_at = ?2, updated_at = ?2, deleted_at = NULL
              WHERE id = ?1",
            params![id, now],
        )?;
        return Ok(InsertOutcome::Promoted(id));
    }

    let id = uuid::Uuid::new_v4().to_string();
    conn.execute(
        "INSERT INTO clip_items
            (id, clip_type, content, preview, image_path, thumb, content_hash,
             size_bytes, source_app, created_at, accessed_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?10, ?10)",
        params![
            id,
            clip.clip_type.as_str(),
            clip.content,
            clip.preview,
            clip.image_path,
            clip.thumb,
            clip.content_hash,
            clip.size_bytes,
            clip.source_app,
            now,
        ],
    )?;

    Ok(InsertOutcome::Inserted(id))
}

/// Turns raw user input into a safe FTS5 MATCH expression.
///
/// FTS5 treats characters like `"`, `*`, `:`, `-` and `(` as syntax, so passing a
/// search box straight through makes the query error out on perfectly ordinary text
/// (`foo"bar`, `2024-01-01`). Quoting each token as a phrase neutralises all of it;
/// a trailing `*` on the final token keeps search feeling incremental as you type.
fn fts_query(term: &str) -> Option<String> {
    let tokens: Vec<String> = term
        .split_whitespace()
        .map(|t| t.replace('"', "\"\""))
        .filter(|t| !t.is_empty())
        .collect();

    if tokens.is_empty() {
        return None;
    }

    let last = tokens.len() - 1;
    let expr = tokens
        .iter()
        .enumerate()
        .map(|(i, t)| {
            if i == last {
                format!("\"{t}\"*")
            } else {
                format!("\"{t}\"")
            }
        })
        .collect::<Vec<_>>()
        .join(" ");

    Some(expr)
}

/// The history list. Pinned items float to the top, then newest first.
pub fn list_clips(conn: &Connection, query: &ClipQuery) -> Result<Vec<ClipItem>> {
    let limit = query.limit.unwrap_or(100).min(500) as i64;
    let offset = query.offset.unwrap_or(0) as i64;
    let search = query.search.as_deref().and_then(fts_query);

    let mut sql = String::from("SELECT ");
    sql.push_str(CLIP_COLUMNS);
    sql.push_str(" FROM clip_items c");

    let mut conditions = vec!["c.deleted_at IS NULL".to_owned()];
    let mut binds: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();

    if let Some(expr) = &search {
        sql.push_str(" JOIN clip_items_fts f ON f.rowid = c.rowid");
        conditions.push("clip_items_fts MATCH ?".to_owned());
        binds.push(Box::new(expr.clone()));
    }
    if let Some(clip_type) = query.clip_type {
        conditions.push("c.clip_type = ?".to_owned());
        binds.push(Box::new(clip_type.as_str().to_owned()));
    }
    if query.favorites_only.unwrap_or(false) {
        conditions.push("c.is_favorite = 1".to_owned());
    }
    if query.pinned_only.unwrap_or(false) {
        conditions.push("c.is_pinned = 1".to_owned());
    }

    sql.push_str(" WHERE ");
    sql.push_str(&conditions.join(" AND "));
    sql.push_str(" ORDER BY c.is_pinned DESC, c.created_at DESC LIMIT ? OFFSET ?");
    binds.push(Box::new(limit));
    binds.push(Box::new(offset));

    // `SELECT id, …` in CLIP_COLUMNS is unqualified; alias resolution picks `c`.
    let sql = sql.replacen("SELECT id,", "SELECT c.id,", 1);

    let mut stmt = conn.prepare(&sql)?;
    let params: Vec<&dyn rusqlite::ToSql> = binds.iter().map(|b| b.as_ref()).collect();
    let rows = stmt.query_map(params.as_slice(), map_clip)?;

    rows.collect::<rusqlite::Result<Vec<_>>>()
        .map_err(Error::from)
}

/// Full text of a clip. Separate from `list_clips` so the list never ships payloads.
pub fn clip_content(conn: &Connection, id: &str) -> Result<Option<String>> {
    conn.query_row(
        "SELECT content FROM clip_items WHERE id = ?1 AND deleted_at IS NULL",
        params![id],
        |row| row.get(0),
    )
    .optional()
    .map(Option::flatten)
    .map_err(Error::from)
}

pub fn clip_image_path(conn: &Connection, id: &str) -> Result<Option<String>> {
    conn.query_row(
        "SELECT image_path FROM clip_items WHERE id = ?1",
        params![id],
        |row| row.get(0),
    )
    .optional()
    .map(Option::flatten)
    .map_err(Error::from)
}

pub fn touch(conn: &Connection, id: &str) -> Result<()> {
    let now = now_millis();
    conn.execute(
        "UPDATE clip_items SET accessed_at = ?2, updated_at = ?2 WHERE id = ?1",
        params![id, now],
    )?;
    Ok(())
}

pub fn set_favorite(conn: &Connection, id: &str, value: bool) -> Result<()> {
    set_flag(conn, id, "is_favorite", value)
}

pub fn set_pinned(conn: &Connection, id: &str, value: bool) -> Result<()> {
    set_flag(conn, id, "is_pinned", value)
}

fn set_flag(conn: &Connection, id: &str, column: &'static str, value: bool) -> Result<()> {
    // `column` is a compile-time constant from the two callers above, never user input.
    let sql = format!("UPDATE clip_items SET {column} = ?2, updated_at = ?3 WHERE id = ?1");
    conn.execute(&sql, params![id, i64::from(value), now_millis()])?;
    Ok(())
}

/// Soft-deletes a clip and reports the image file to unlink, if any.
///
/// Deletion is a tombstone rather than a `DELETE` so that a future sync can
/// propagate the removal instead of resurrecting the row from another device.
pub fn delete_clip(conn: &Connection, id: &str) -> Result<Option<String>> {
    let image_path = clip_image_path(conn, id)?;
    let now = now_millis();

    // Clear the payload as well: a tombstone that still holds the copied text would
    // keep the content on disk after the user asked for it to be gone.
    conn.execute(
        "UPDATE clip_items
            SET deleted_at = ?2, updated_at = ?2, content = NULL, preview = '',
                thumb = NULL, image_path = NULL
          WHERE id = ?1",
        params![id, now],
    )?;

    Ok(image_path)
}

/// Deletes everything except favorites and pins. Returns the image files to unlink.
pub fn clear_history(conn: &Connection) -> Result<Vec<String>> {
    let mut stmt = conn.prepare(
        "SELECT image_path FROM clip_items
          WHERE deleted_at IS NULL AND is_favorite = 0 AND is_pinned = 0
            AND image_path IS NOT NULL",
    )?;
    let paths: Vec<String> = stmt
        .query_map([], |row| row.get(0))?
        .collect::<rusqlite::Result<Vec<_>>>()?;

    let now = now_millis();
    conn.execute(
        "UPDATE clip_items
            SET deleted_at = ?1, updated_at = ?1, content = NULL, preview = '',
                thumb = NULL, image_path = NULL
          WHERE deleted_at IS NULL AND is_favorite = 0 AND is_pinned = 0",
        params![now],
    )?;

    Ok(paths)
}

/// Enforces the retention policy. Returns image files that should be unlinked.
///
/// Favorites and pins are never purged — that is the contract those flags exist for.
pub fn purge(conn: &Connection, settings: &AppSettings) -> Result<Vec<String>> {
    let mut doomed: Vec<String> = Vec::new();
    let now = now_millis();

    // Age-based: retention_days == 0 means keep forever.
    if settings.retention_days > 0 {
        let cutoff = now - (settings.retention_days as i64 * 86_400_000);
        let mut stmt = conn.prepare(
            "SELECT id, image_path FROM clip_items
              WHERE deleted_at IS NULL AND is_favorite = 0 AND is_pinned = 0
                AND created_at < ?1",
        )?;
        let rows: Vec<(String, Option<String>)> = stmt
            .query_map(params![cutoff], |r| Ok((r.get(0)?, r.get(1)?)))?
            .collect::<rusqlite::Result<Vec<_>>>()?;

        for (id, path) in rows {
            if let Some(p) = delete_clip(conn, &id)?.or(path) {
                doomed.push(p);
            }
        }
    }

    // Count-based: keep the newest `max_items`, plus every favorite and pin.
    let mut stmt = conn.prepare(
        "SELECT id FROM clip_items
          WHERE deleted_at IS NULL AND is_favorite = 0 AND is_pinned = 0
          ORDER BY created_at DESC
          LIMIT -1 OFFSET ?1",
    )?;
    let overflow: Vec<String> = stmt
        .query_map(params![settings.max_items as i64], |r| r.get(0))?
        .collect::<rusqlite::Result<Vec<_>>>()?;

    for id in overflow {
        if let Some(p) = delete_clip(conn, &id)? {
            doomed.push(p);
        }
    }

    Ok(doomed)
}

// ---------------------------------------------------------------- screenshots

/// Records a capture. Returns the new row's id.
pub fn insert_screenshot(
    conn: &Connection,
    path: &str,
    mode: &str,
    width: u32,
    height: u32,
) -> Result<String> {
    let id = uuid::Uuid::new_v4().to_string();
    let now = now_millis();

    conn.execute(
        "INSERT INTO screenshots
            (id, image_path, capture_mode, width, height, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?6)",
        params![id, path, mode, width as i64, height as i64, now],
    )?;

    Ok(id)
}

pub fn count_clips(conn: &Connection) -> Result<i64> {
    conn.query_row(
        "SELECT count(*) FROM clip_items WHERE deleted_at IS NULL",
        [],
        |row| row.get(0),
    )
    .map_err(Error::from)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::clipboard::types::ClipType;

    fn db() -> Connection {
        let mut conn = Connection::open_in_memory().unwrap();
        crate::db::migrations::run(&mut conn).unwrap();
        conn
    }

    fn text_clip(body: &str) -> NewClip {
        NewClip {
            clip_type: ClipType::Text,
            content: Some(body.to_owned()),
            preview: body.chars().take(200).collect(),
            image_path: None,
            thumb: None,
            content_hash: blake3::hash(body.as_bytes()).to_hex().to_string(),
            size_bytes: body.len() as i64,
            source_app: Some("TestApp".to_owned()),
        }
    }

    #[test]
    fn inserting_then_listing_returns_the_clip() {
        let conn = db();
        insert_clip(&conn, &text_clip("hello world")).unwrap();

        let items = list_clips(&conn, &ClipQuery::default()).unwrap();
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].preview, "hello world");
        assert_eq!(items[0].source_app.as_deref(), Some("TestApp"));
    }

    #[test]
    fn duplicate_content_promotes_instead_of_duplicating() {
        let conn = db();
        let first = insert_clip(&conn, &text_clip("same")).unwrap();
        let second = insert_clip(&conn, &text_clip("same")).unwrap();

        assert!(matches!(first, InsertOutcome::Inserted(_)));
        assert!(matches!(second, InsertOutcome::Promoted(_)));
        assert_eq!(count_clips(&conn).unwrap(), 1);
    }

    #[test]
    fn pinned_items_sort_above_newer_ones() {
        let conn = db();
        let InsertOutcome::Inserted(old) = insert_clip(&conn, &text_clip("old")).unwrap() else {
            panic!("expected insert");
        };
        insert_clip(&conn, &text_clip("new")).unwrap();
        set_pinned(&conn, &old, true).unwrap();

        let items = list_clips(&conn, &ClipQuery::default()).unwrap();
        assert_eq!(items[0].preview, "old", "pinned clip should lead the list");
    }

    #[test]
    fn search_matches_on_a_word_prefix() {
        let conn = db();
        insert_clip(&conn, &text_clip("the quick brown fox")).unwrap();
        insert_clip(&conn, &text_clip("completely unrelated")).unwrap();

        let q = ClipQuery {
            search: Some("qui".to_owned()),
            ..Default::default()
        };
        let items = list_clips(&conn, &q).unwrap();
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].preview, "the quick brown fox");
    }

    #[test]
    fn search_survives_fts_syntax_characters() {
        // Raw FTS5 would throw a syntax error on every one of these.
        let conn = db();
        insert_clip(&conn, &text_clip("release 2024-01-01 shipped")).unwrap();

        for term in ["2024-01-01", "foo\"bar", "a:b", "(paren", "*", "AND OR NOT"] {
            let q = ClipQuery {
                search: Some(term.to_owned()),
                ..Default::default()
            };
            list_clips(&conn, &q)
                .unwrap_or_else(|e| panic!("search for {term:?} should not error: {e}"));
        }
    }

    #[test]
    fn type_filter_excludes_other_kinds() {
        let conn = db();
        insert_clip(&conn, &text_clip("some text")).unwrap();
        let mut img = text_clip("an image");
        img.clip_type = ClipType::Image;
        img.content_hash = "image-hash".to_owned();
        insert_clip(&conn, &img).unwrap();

        let q = ClipQuery {
            clip_type: Some(ClipType::Image),
            ..Default::default()
        };
        assert_eq!(list_clips(&conn, &q).unwrap().len(), 1);
    }

    #[test]
    fn deleting_clears_the_payload_not_just_the_row() {
        let conn = db();
        let InsertOutcome::Inserted(id) = insert_clip(&conn, &text_clip("secret-ish")).unwrap()
        else {
            panic!("expected insert");
        };

        delete_clip(&conn, &id).unwrap();

        assert!(list_clips(&conn, &ClipQuery::default()).unwrap().is_empty());
        let leftover: Option<String> = conn
            .query_row(
                "SELECT content FROM clip_items WHERE id = ?1",
                params![id],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(leftover, None, "tombstone must not retain the content");
    }

    #[test]
    fn purge_respects_max_items_but_keeps_favorites() {
        let conn = db();
        let mut ids = Vec::new();
        for i in 0..10 {
            let clip = text_clip(&format!("clip number {i}"));
            if let InsertOutcome::Inserted(id) = insert_clip(&conn, &clip).unwrap() {
                ids.push(id);
            }
        }
        // Favorite the oldest, which is exactly what the count cap would evict.
        set_favorite(&conn, &ids[0], true).unwrap();

        let settings = AppSettings {
            max_items: 3,
            retention_days: 0,
            ..Default::default()
        };
        purge(&conn, &settings).unwrap();

        let remaining = list_clips(&conn, &ClipQuery::default()).unwrap();
        assert_eq!(remaining.len(), 4, "3 newest + 1 favorite");
        assert!(
            remaining.iter().any(|c| c.id == ids[0]),
            "favorite survives"
        );
    }

    #[test]
    fn purge_drops_items_past_the_retention_window() {
        let conn = db();
        let InsertOutcome::Inserted(id) = insert_clip(&conn, &text_clip("ancient")).unwrap() else {
            panic!("expected insert");
        };
        let long_ago = now_millis() - (60 * 86_400_000);
        conn.execute(
            "UPDATE clip_items SET created_at = ?2 WHERE id = ?1",
            params![id, long_ago],
        )
        .unwrap();

        let settings = AppSettings {
            retention_days: 30,
            ..Default::default()
        };
        purge(&conn, &settings).unwrap();

        assert_eq!(count_clips(&conn).unwrap(), 0);
    }

    #[test]
    fn clear_history_keeps_favorites_and_pins() {
        let conn = db();
        let InsertOutcome::Inserted(keep) = insert_clip(&conn, &text_clip("keep me")).unwrap()
        else {
            panic!("expected insert");
        };
        insert_clip(&conn, &text_clip("throw away")).unwrap();
        set_favorite(&conn, &keep, true).unwrap();

        clear_history(&conn).unwrap();

        let remaining = list_clips(&conn, &ClipQuery::default()).unwrap();
        assert_eq!(remaining.len(), 1);
        assert_eq!(remaining[0].id, keep);
    }
}
