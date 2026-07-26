//! Platform-specific clipboard introspection.
//!
//! Everything here answers a question *about* the clipboard without reading its
//! payload. Reading the payload is `arboard`'s job in the layer above; keeping the
//! two separate is what lets the monitor poll cheaply and only pay for a read when
//! something actually changed.

#[cfg(target_os = "macos")]
mod macos;
#[cfg(target_os = "macos")]
pub use macos::*;

#[cfg(target_os = "windows")]
mod windows;
#[cfg(target_os = "windows")]
pub use windows::*;

// Linux is Phase 6. The fallback keeps the crate compiling (and CI green) on any
// other target: `sequence()` never changes, so the monitor simply never fires
// rather than silently recording with broken semantics.
#[cfg(not(any(target_os = "macos", target_os = "windows")))]
mod fallback {
    pub fn sequence() -> i64 {
        0
    }
    pub fn is_concealed() -> bool {
        // Fail closed: without the platform markers we cannot tell a password from
        // ordinary text, and recording a secret is worse than recording nothing.
        true
    }
    pub fn has_html() -> bool {
        false
    }
    pub fn has_rtf() -> bool {
        false
    }
    pub fn frontmost_app() -> Option<String> {
        None
    }
}
#[cfg(not(any(target_os = "macos", target_os = "windows")))]
pub use fallback::*;
