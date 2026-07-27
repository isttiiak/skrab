use std::str::FromStr;

use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut, ShortcutState};
use ts_rs::TS;

use crate::error::Result;
use crate::window;

/// Everything a hotkey can trigger.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export, export_to = "HotkeyAction.ts")]
#[serde(rename_all = "camelCase")]
pub enum HotkeyAction {
    TogglePanel,
    ToggleAlwaysOnTop,
    CaptureRegion,
    CaptureFullscreen,
}

impl HotkeyAction {
    pub const ALL: [HotkeyAction; 4] = [
        HotkeyAction::TogglePanel,
        HotkeyAction::ToggleAlwaysOnTop,
        HotkeyAction::CaptureRegion,
        HotkeyAction::CaptureFullscreen,
    ];

    /// Human label, shown in Settings.
    pub fn label(self) -> &'static str {
        match self {
            HotkeyAction::TogglePanel => "Clipboard history",
            HotkeyAction::ToggleAlwaysOnTop => "Keep panel on top",
            HotkeyAction::CaptureRegion => "Capture a region",
            HotkeyAction::CaptureFullscreen => "Capture the screen",
        }
    }
}

/// The user's accelerator for each action.
///
/// Stored as Tauri accelerator strings (`"CmdOrCtrl+Shift+V"`) rather than parsed
/// shortcuts so the value survives round-tripping through JSON settings and can be
/// shown back to the user exactly as typed. `CmdOrCtrl` resolves to Command on macOS
/// and Control elsewhere, which is what users expect from a cross-platform default.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "HotkeyBindings.ts")]
#[serde(rename_all = "camelCase", default)]
pub struct HotkeyBindings {
    pub toggle_panel: String,
    pub toggle_always_on_top: String,
    pub capture_region: String,
    pub capture_fullscreen: String,
}

impl Default for HotkeyBindings {
    fn default() -> Self {
        Self {
            toggle_panel: "CmdOrCtrl+Shift+V".to_owned(),
            toggle_always_on_top: "CmdOrCtrl+Shift+P".to_owned(),
            capture_region: "CmdOrCtrl+Shift+A".to_owned(),
            capture_fullscreen: "CmdOrCtrl+Shift+S".to_owned(),
        }
    }
}

impl HotkeyBindings {
    pub fn get(&self, action: HotkeyAction) -> &str {
        match action {
            HotkeyAction::TogglePanel => &self.toggle_panel,
            HotkeyAction::ToggleAlwaysOnTop => &self.toggle_always_on_top,
            HotkeyAction::CaptureRegion => &self.capture_region,
            HotkeyAction::CaptureFullscreen => &self.capture_fullscreen,
        }
    }

    /// Actions whose accelerator string is identical to an earlier one.
    ///
    /// Registering the same accelerator twice silently gives the second one to
    /// whichever action registered first, so an internal clash has to be caught
    /// before it reaches the OS.
    pub fn internal_conflicts(&self) -> Vec<HotkeyAction> {
        let mut seen: Vec<&str> = Vec::new();
        let mut clashing = Vec::new();

        for action in HotkeyAction::ALL {
            let accel = self.get(action).trim();
            if accel.is_empty() {
                continue;
            }
            if seen.iter().any(|s| s.eq_ignore_ascii_case(accel)) {
                clashing.push(action);
            } else {
                seen.push(accel);
            }
        }
        clashing
    }
}

/// Outcome of trying to register one binding, reported back to Settings.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "HotkeyStatus.ts")]
#[serde(rename_all = "camelCase")]
pub struct HotkeyStatus {
    pub action: HotkeyAction,
    pub label: String,
    pub accelerator: String,
    pub registered: bool,
    /// Why it failed, in words the user can act on.
    pub problem: Option<String>,
}

/// The currently registered bindings, shared with the shortcut handler.
pub struct HotkeyState(RwLock<Vec<(HotkeyAction, Shortcut)>>);

impl HotkeyState {
    pub fn new() -> Self {
        Self(RwLock::new(Vec::new()))
    }

    fn action_for(&self, pressed: &Shortcut) -> Option<HotkeyAction> {
        self.0
            .read()
            .iter()
            .find(|(_, shortcut)| shortcut == pressed)
            .map(|(action, _)| *action)
    }
}

/// Installs the shortcut plugin. Bindings are applied separately by `apply`.
pub fn setup(app: &AppHandle) -> Result<()> {
    app.manage(HotkeyState::new());

    app.plugin(
        tauri_plugin_global_shortcut::Builder::new()
            .with_handler(|app, shortcut, event| {
                // Fire on press only; the release event would toggle straight back.
                if event.state() != ShortcutState::Pressed {
                    return;
                }
                let Some(action) = app.state::<HotkeyState>().action_for(shortcut) else {
                    return;
                };
                dispatch(app, action);
            })
            .build(),
    )?;

    Ok(())
}

fn dispatch(app: &AppHandle, action: HotkeyAction) {
    match action {
        HotkeyAction::TogglePanel => window::toggle(app),
        HotkeyAction::ToggleAlwaysOnTop => {
            let pinned = window::is_always_on_top(app);
            if let Err(e) = window::set_always_on_top(app, !pinned) {
                log::error!("could not toggle always-on-top: {e}");
            }
        }
        HotkeyAction::CaptureRegion => crate::screenshot::overlay::open(app),
        HotkeyAction::CaptureFullscreen => crate::screenshot::overlay::capture_fullscreen_now(app),
    }
}

/// Replaces every registered accelerator with the given bindings.
///
/// Unregisters everything first so a rebind cannot leave the old accelerator live.
/// A binding that fails is reported rather than aborting the rest — one taken
/// shortcut should not cost the user their other three.
pub fn apply(app: &AppHandle, bindings: &HotkeyBindings) -> Vec<HotkeyStatus> {
    let manager = app.global_shortcut();
    if let Err(e) = manager.unregister_all() {
        log::warn!("could not clear existing shortcuts: {e}");
    }

    let conflicts = bindings.internal_conflicts();
    let mut registered: Vec<(HotkeyAction, Shortcut)> = Vec::new();
    let mut statuses = Vec::new();

    for action in HotkeyAction::ALL {
        let accelerator = bindings.get(action).trim().to_owned();
        let mut status = HotkeyStatus {
            action,
            label: action.label().to_owned(),
            accelerator: accelerator.clone(),
            registered: false,
            problem: None,
        };

        if accelerator.is_empty() {
            status.problem = Some("No shortcut set.".to_owned());
            statuses.push(status);
            continue;
        }

        if conflicts.contains(&action) {
            status.problem = Some("Another Skrab action already uses this shortcut.".to_owned());
            statuses.push(status);
            continue;
        }

        match Shortcut::from_str(&accelerator) {
            Err(_) => {
                status.problem = Some(format!("\"{accelerator}\" is not a valid shortcut."));
            }
            Ok(shortcut) => match manager.register(shortcut) {
                Ok(()) => {
                    status.registered = true;
                    registered.push((action, shortcut));
                }
                Err(e) => {
                    // Almost always means another application owns it.
                    log::warn!("could not register {accelerator} for {action:?}: {e}");
                    status.problem = Some(
                        "Another application is already using this shortcut. Pick a \
                         different combination."
                            .to_owned(),
                    );
                }
            },
        }

        statuses.push(status);
    }

    *app.state::<HotkeyState>().0.write() = registered;

    let live = statuses.iter().filter(|s| s.registered).count();
    log::info!("{live} of {} global shortcuts registered", statuses.len());
    statuses
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_do_not_clash_with_each_other() {
        assert!(HotkeyBindings::default().internal_conflicts().is_empty());
    }

    #[test]
    fn a_duplicate_accelerator_is_reported_once() {
        let bindings = HotkeyBindings {
            toggle_panel: "CmdOrCtrl+Shift+V".to_owned(),
            toggle_always_on_top: "CmdOrCtrl+Shift+V".to_owned(),
            ..Default::default()
        };
        // The first use wins; only the later one is flagged.
        assert_eq!(
            bindings.internal_conflicts(),
            vec![HotkeyAction::ToggleAlwaysOnTop]
        );
    }

    #[test]
    fn conflict_detection_ignores_case() {
        let bindings = HotkeyBindings {
            toggle_panel: "CmdOrCtrl+Shift+V".to_owned(),
            toggle_always_on_top: "cmdorctrl+shift+v".to_owned(),
            ..Default::default()
        };
        assert_eq!(
            bindings.internal_conflicts(),
            vec![HotkeyAction::ToggleAlwaysOnTop]
        );
    }

    #[test]
    fn an_empty_binding_is_not_a_conflict() {
        // Clearing two shortcuts must not report them as clashing with each other.
        let bindings = HotkeyBindings {
            toggle_panel: String::new(),
            toggle_always_on_top: "  ".to_owned(),
            ..Default::default()
        };
        assert!(bindings.internal_conflicts().is_empty());
    }

    #[test]
    fn every_default_parses_as_a_real_accelerator() {
        let bindings = HotkeyBindings::default();
        for action in HotkeyAction::ALL {
            let accel = bindings.get(action);
            assert!(
                Shortcut::from_str(accel).is_ok(),
                "default for {action:?} ({accel}) must parse"
            );
        }
    }

    #[test]
    fn missing_fields_fall_back_to_defaults() {
        // Settings written by an older build will not have the newer actions.
        let bindings: HotkeyBindings =
            serde_json::from_str(r#"{"togglePanel":"CmdOrCtrl+K"}"#).unwrap();
        assert_eq!(bindings.toggle_panel, "CmdOrCtrl+K");
        assert_eq!(bindings.toggle_always_on_top, "CmdOrCtrl+Shift+P");
    }
}
