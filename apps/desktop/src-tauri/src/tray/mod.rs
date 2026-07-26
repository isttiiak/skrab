use tauri::App;
use tauri::menu::{Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};

use crate::error::Result;
use crate::window;

/// Builds the system tray icon and its menu.
///
/// The tray is the app's real entry point — the window is a transient panel, so
/// quitting has to be explicit here rather than implied by closing the window.
pub fn setup(app: &App) -> Result<()> {
    let handle = app.handle();

    let show_item = MenuItem::with_id(handle, "show", "Open Skrab", true, None::<&str>)?;
    let pins_item = MenuItem::with_id(handle, "pins", "Pinned items", true, None::<&str>)?;
    let settings_item = MenuItem::with_id(handle, "settings", "Settings…", true, None::<&str>)?;
    let separator = PredefinedMenuItem::separator(handle)?;
    let quit_item = MenuItem::with_id(handle, "quit", "Quit Skrab", true, None::<&str>)?;

    let menu = Menu::with_items(
        handle,
        &[
            &show_item,
            &pins_item,
            &settings_item,
            &separator,
            &quit_item,
        ],
    )?;

    TrayIconBuilder::with_id("skrab-tray")
        .icon(app.default_window_icon().cloned().ok_or_else(|| {
            crate::error::Error::Other("no default window icon configured".into())
        })?)
        .tooltip("Skrab")
        .menu(&menu)
        // The menu should only open on right-click; left-click summons the panel.
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id().as_ref() {
            "show" => window::show(app),
            "pins" => window::toggle_pins(app),
            "settings" => {
                window::show(app);
                // Phase 1 routes the panel to the settings view here.
            }
            "quit" => app.exit(0),
            other => log::warn!("unhandled tray menu id: {other}"),
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                window::toggle(tray.app_handle());
            }
        })
        .build(app)?;

    Ok(())
}
