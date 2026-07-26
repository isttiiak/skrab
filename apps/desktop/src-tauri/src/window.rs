use tauri::{AppHandle, Manager, WebviewWindow};

pub const MAIN: &str = "main";

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
