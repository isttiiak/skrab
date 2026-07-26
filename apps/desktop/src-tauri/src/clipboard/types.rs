use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// What kind of payload a clip holds.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export, export_to = "ClipType.ts")]
#[serde(rename_all = "lowercase")]
pub enum ClipType {
    Text,
    Image,
    Html,
    Rtf,
    File,
}

impl ClipType {
    /// The string stored in `clip_items.clip_type` (matches the CHECK constraint).
    pub fn as_str(self) -> &'static str {
        match self {
            ClipType::Text => "text",
            ClipType::Image => "image",
            ClipType::Html => "html",
            ClipType::Rtf => "rtf",
            ClipType::File => "file",
        }
    }

    pub fn from_str(value: &str) -> Option<Self> {
        match value {
            "text" => Some(ClipType::Text),
            "image" => Some(ClipType::Image),
            "html" => Some(ClipType::Html),
            "rtf" => Some(ClipType::Rtf),
            "file" => Some(ClipType::File),
            _ => None,
        }
    }
}

/// A clipboard entry as the history list sees it.
///
/// Deliberately does **not** carry the full payload: a 10MB copied document would
/// otherwise be serialized across the IPC bridge for every row on every render.
/// Call `get_clip_content` when the full value is actually needed.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "ClipItem.ts")]
#[serde(rename_all = "camelCase")]
pub struct ClipItem {
    pub id: String,
    pub clip_type: ClipType,
    /// First ~200 characters, for the list row.
    pub preview: String,
    /// Data URI of a small JPEG thumbnail. Only present for image clips.
    pub thumb: Option<String>,
    /// Absolute path to the full-size image on disk. Only present for image clips.
    pub image_path: Option<String>,
    // i64/u64 default to `bigint` in ts-rs, but Tauri's IPC delivers a plain JSON
    // number. These values (unix millis, byte counts) are far below 2^53, so the
    // annotation makes the generated type match what actually arrives.
    #[ts(type = "number")]
    pub size_bytes: i64,
    pub source_app: Option<String>,
    pub is_pinned: bool,
    pub is_favorite: bool,
    pub category: Option<String>,
    #[ts(type = "number")]
    pub created_at: i64,
    #[ts(type = "number")]
    pub accessed_at: i64,
}

/// Filters for the history query.
#[derive(Debug, Clone, Default, Serialize, Deserialize, TS)]
#[ts(export, export_to = "ClipQuery.ts")]
#[serde(rename_all = "camelCase")]
pub struct ClipQuery {
    /// Full-text search term. Empty or absent means "no text filter".
    pub search: Option<String>,
    /// Restrict to one kind of clip.
    pub clip_type: Option<ClipType>,
    /// Only favorites.
    pub favorites_only: Option<bool>,
    /// Only pinned items.
    pub pinned_only: Option<bool>,
    pub limit: Option<u32>,
    pub offset: Option<u32>,
}

/// A newly captured clip, before it reaches the database.
#[derive(Debug, Clone)]
pub struct NewClip {
    pub clip_type: ClipType,
    pub content: Option<String>,
    pub preview: String,
    pub image_path: Option<String>,
    pub thumb: Option<Vec<u8>>,
    pub content_hash: String,
    pub size_bytes: i64,
    pub source_app: Option<String>,
}

/// Emitted to the frontend whenever the history changes.
pub const CLIP_ADDED_EVENT: &str = "skrab://clip-added";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clip_type_round_trips_through_its_db_string() {
        for t in [
            ClipType::Text,
            ClipType::Image,
            ClipType::Html,
            ClipType::Rtf,
            ClipType::File,
        ] {
            assert_eq!(ClipType::from_str(t.as_str()), Some(t));
        }
    }

    #[test]
    fn unknown_db_value_is_rejected_rather_than_defaulted() {
        assert_eq!(ClipType::from_str("video"), None);
    }
}
