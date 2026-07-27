use tauri::{AppHandle, Manager, WebviewUrl, WebviewWindow, WebviewWindowBuilder};

use crate::error::Result;

pub const MAIN: &str = "main";
/// The annotation editor.
pub const EDITOR: &str = "editor";

/// Opens the annotation editor on one image clip.
///
/// Its own window rather than a view inside the panel: the panel is a 440px summoned
/// overlay, and annotating a 1920px screenshot in it would be miserable.
pub fn open_editor(app: &AppHandle, clip_id: &str) -> Result<()> {
    // Rebuilt each time so it always opens on the clip that was asked for.
    if let Some(existing) = app.get_webview_window(EDITOR) {
        existing.destroy()?;
    }

    let window = WebviewWindowBuilder::new(
        app,
        EDITOR,
        WebviewUrl::App(format!("index.html?view=editor&id={clip_id}").into()),
    )
    .title("Skrab · Edit")
    .inner_size(1100.0, 760.0)
    .min_inner_size(720.0, 520.0)
    .resizable(true)
    .center()
    .build()?;

    window.set_focus()?;
    Ok(())
}

pub fn main_window(app: &AppHandle) -> Option<WebviewWindow> {
    app.get_webview_window(MAIN)
}

/// Brings the panel to the front and focuses it.
///
/// Focus matters more than visibility here: the panel is summoned by a hotkey
/// while another app owns the keyboard, so showing without focusing would leave
/// the user typing into whatever was behind it.
pub fn show(app: &AppHandle) {
    let Some(window) = main_window(app) else {
        log::warn!("show: main window is gone");
        return;
    };

    if let Err(e) = window.show() {
        log::error!("failed to show main window: {e}");
    }
    if let Err(e) = window.set_focus() {
        log::error!("failed to focus main window: {e}");
    }
}

pub fn hide(app: &AppHandle) {
    if let Some(window) = main_window(app)
        && let Err(e) = window.hide()
    {
        log::error!("failed to hide main window: {e}");
    }
}

/// Pins the panel above every other window, or releases it.
///
/// This is what makes Skrab usable beside a form: the panel stays visible over the
/// app you are filling in, and clicking a clip copies it without the panel
/// disappearing. It is the same window, not a second one — there is only ever one
/// list, and everything in it is reachable.
pub fn set_always_on_top(app: &AppHandle, pinned: bool) -> Result<()> {
    let Some(window) = main_window(app) else {
        return Ok(());
    };
    window.set_always_on_top(pinned)?;
    if pinned {
        window.show()?;
        window.set_focus()?;
    }
    Ok(())
}

pub fn is_always_on_top(app: &AppHandle) -> bool {
    main_window(app)
        .and_then(|w| w.is_always_on_top().ok())
        .unwrap_or(false)
}

/// Hotkey behaviour: if the panel is already up and focused, dismiss it.
pub fn toggle(app: &AppHandle) {
    let Some(window) = main_window(app) else {
        return;
    };

    let visible = window.is_visible().unwrap_or(false);
    let focused = window.is_focused().unwrap_or(false);

    if visible && focused {
        hide(app)
    } else {
        show(app)
    }
}
