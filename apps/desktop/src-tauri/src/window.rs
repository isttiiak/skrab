use tauri::{AppHandle, Manager, WebviewUrl, WebviewWindow, WebviewWindowBuilder};

use crate::error::Result;

pub const MAIN: &str = "main";
/// The always-on-top smart-paste widget.
pub const PINS: &str = "pins";

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

/// Shows the pinned-items widget, creating it on first use.
///
/// Built on demand rather than declared in tauri.conf.json so that a user who never
/// pins anything never pays for a second webview process.
pub fn show_pins(app: &AppHandle) -> Result<()> {
    if let Some(window) = app.get_webview_window(PINS) {
        window.show()?;
        window.set_focus()?;
        return Ok(());
    }

    let window = WebviewWindowBuilder::new(
        app,
        PINS,
        // Same bundle as the main panel; the query string picks the view.
        WebviewUrl::App("index.html?view=pins".into()),
    )
    .title("Skrab · Pinned")
    .inner_size(280.0, 380.0)
    .min_inner_size(220.0, 160.0)
    // The whole point is that it floats beside the form you are filling in.
    .always_on_top(true)
    .decorations(false)
    .resizable(true)
    .shadow(true)
    // Keep it out of the taskbar/dock switcher: it is an accessory, not an app.
    .skip_taskbar(true)
    .build()?;

    window.set_focus()?;
    Ok(())
}

pub fn toggle_pins(app: &AppHandle) {
    match app.get_webview_window(PINS) {
        Some(window) if window.is_visible().unwrap_or(false) => {
            if let Err(e) = window.hide() {
                log::error!("failed to hide the pins widget: {e}");
            }
        }
        _ => {
            if let Err(e) = show_pins(app) {
                log::error!("failed to open the pins widget: {e}");
            }
        }
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
