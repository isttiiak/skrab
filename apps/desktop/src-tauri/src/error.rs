use serde::Serialize;

/// Every fallible path in the app funnels through this type.
///
/// It serializes to a plain string so the frontend's `invoke` rejection carries a
/// readable message, but it keeps real variants internally so Rust code can match
/// on the cause instead of parsing strings.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("database error: {0}")]
    Database(#[from] rusqlite::Error),

    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    #[error("tauri error: {0}")]
    Tauri(#[from] tauri::Error),

    #[error("could not resolve the application data directory")]
    NoAppDataDir,

    #[error(
        "the database could not be unlocked — the encryption key does not match \
         Skrab.db. This usually means the key was lost from the OS keychain. The \
         existing history cannot be recovered without it; move or delete \
         {0} to start a fresh database."
    )]
    DatabaseKeyMismatch(String),

    // getrandom 0.3 is no_std and its Error does not implement std::error::Error,
    // so this carries the message rather than the source.
    #[error("failed to generate secure random bytes: {0}")]
    Random(String),

    #[error("{0}")]
    Other(String),
}

/// Tauri commands must return something serializable. Collapse to the message.
impl Serialize for Error {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.to_string())
    }
}

pub type Result<T, E = Error> = std::result::Result<T, E>;
