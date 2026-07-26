//! Simulated keystrokes for auto-paste.
//!
//! This is the one place Skrab acts *on* another application rather than just
//! reading the clipboard, and on macOS it needs Accessibility permission — a
//! meaningful trust ask. It is therefore off by default and gated behind a setting,
//! never enabled implicitly. Click-to-copy works without any of this.

use std::time::Duration;

use crate::error::{Error, Result};

/// Presses the platform paste chord in whatever app currently has focus.
///
/// The caller must have already put the payload on the clipboard and given the
/// target app time to regain focus — this only sends the keystroke.
pub fn send_paste() -> Result<()> {
    use enigo::{Direction, Enigo, Key, Keyboard, Settings};

    let mut enigo = Enigo::new(&Settings::default()).map_err(|e| {
        Error::Other(format!(
            "could not access the input system: {e}. On macOS, grant Skrab \
             Accessibility permission in System Settings → Privacy & Security → \
             Accessibility, then quit and reopen Skrab."
        ))
    })?;

    #[cfg(target_os = "macos")]
    let modifier = Key::Meta;
    #[cfg(not(target_os = "macos"))]
    let modifier = Key::Control;

    let press = |enigo: &mut Enigo, key: Key, direction: Direction| -> Result<()> {
        enigo
            .key(key, direction)
            .map_err(|e| Error::Other(format!("could not send a keystroke: {e}")))
    };

    press(&mut enigo, modifier, Direction::Press)?;
    press(&mut enigo, Key::Unicode('v'), Direction::Click)?;
    press(&mut enigo, modifier, Direction::Release)?;

    Ok(())
}

/// How long to wait after hiding our window before sending the keystroke.
///
/// The paste has to land in the app the user was working in, which only regains
/// focus once our window is actually gone. Sending immediately races the window
/// manager and pastes into nothing.
pub const FOCUS_SETTLE: Duration = Duration::from_millis(120);
