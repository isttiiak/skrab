use tauri::App;
use tauri_plugin_global_shortcut::{Code, GlobalShortcutExt, Modifiers, Shortcut, ShortcutState};

use crate::error::Result;
use crate::window;

/// Toggles the smart-paste widget: Cmd+Shift+P on macOS, Ctrl+Shift+P elsewhere.
fn pins_widget_shortcut() -> Shortcut {
    #[cfg(target_os = "macos")]
    let modifiers = Modifiers::SUPER | Modifiers::SHIFT;
    #[cfg(not(target_os = "macos"))]
    let modifiers = Modifiers::CONTROL | Modifiers::SHIFT;

    Shortcut::new(Some(modifiers), Code::KeyP)
}

/// The panel-summoning accelerator: Cmd+Shift+V on macOS, Ctrl+Shift+V elsewhere.
///
/// `Modifiers::SUPER` maps to Command on macOS and the Windows key on Windows,
/// so the two platforms need different modifiers rather than one shared constant.
fn toggle_panel_shortcut() -> Shortcut {
    #[cfg(target_os = "macos")]
    let modifiers = Modifiers::SUPER | Modifiers::SHIFT;
    #[cfg(not(target_os = "macos"))]
    let modifiers = Modifiers::CONTROL | Modifiers::SHIFT;

    Shortcut::new(Some(modifiers), Code::KeyV)
}

/// Registers the global shortcuts.
///
/// A failure here is not fatal: the accelerator may already be taken by another
/// app, and Skrab is still fully usable from the tray. Log it and carry on —
/// Phase 1 surfaces the conflict in settings so the user can rebind.
pub fn setup(app: &App) -> Result<()> {
    let toggle = toggle_panel_shortcut();
    let pins = pins_widget_shortcut();

    app.handle().plugin(
        tauri_plugin_global_shortcut::Builder::new()
            .with_handler(move |app, shortcut, event| {
                // Fire on press only; the release event would toggle straight back.
                if event.state() != ShortcutState::Pressed {
                    return;
                }
                if shortcut == &toggle {
                    window::toggle(app);
                } else if shortcut == &pins {
                    window::toggle_pins(app);
                }
            })
            .build(),
    )?;

    for (shortcut, what) in [(toggle, "clipboard panel"), (pins, "pinned widget")] {
        match app.global_shortcut().register(shortcut) {
            Ok(()) => log::info!("registered global shortcut for the {what}"),
            Err(e) => log::warn!(
                "could not register the {what} shortcut ({e}); the tray icon still \
                 works and the shortcut can be rebound in settings"
            ),
        }
    }

    Ok(())
}
