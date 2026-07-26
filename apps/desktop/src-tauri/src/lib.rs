mod clipboard;
mod commands;
mod db;
mod error;
mod hotkeys;
mod security;
mod settings;
mod tray;
mod window;

use tauri::{Manager, WindowEvent};

pub use error::{Error, Result};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let mut builder = tauri::Builder::default();

    // Single instance must be registered before anything that touches the
    // database or the clipboard — a second process would otherwise start its own
    // monitor and write to the same SQLite file.
    #[cfg(desktop)]
    {
        builder = builder.plugin(tauri_plugin_single_instance::init(|app, _argv, _cwd| {
            log::info!("second instance launched; focusing the existing window");
            window::show(app);
        }));
    }

    builder
        .plugin(
            tauri_plugin_log::Builder::new()
                .level(if cfg!(debug_assertions) {
                    log::LevelFilter::Debug
                } else {
                    log::LevelFilter::Info
                })
                .build(),
        )
        .plugin(tauri_plugin_os::init())
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_store::Builder::new().build())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_window_state::Builder::new().build())
        .setup(|app| {
            let app_data_dir = app
                .path()
                .app_data_dir()
                .map_err(|_| error::Error::NoAppDataDir)?;

            let database = db::Database::open(&app_data_dir)?;
            let loaded = database.with(settings::load)?;

            app.manage(clipboard::MonitorState::new(loaded.monitoring_enabled));
            app.manage(settings::SettingsState::new(loaded));
            app.manage(database);

            tray::setup(app)?;

            #[cfg(desktop)]
            {
                app.handle().plugin(tauri_plugin_notification::init())?;
                // The updater plugin is deliberately NOT registered yet: it requires
                // `plugins.updater.pubkey` in tauri.conf.json, and the signing keypair
                // is generated in Phase 2 along with the release pipeline.
                app.handle().plugin(tauri_plugin_autostart::init(
                    tauri_plugin_autostart::MacosLauncher::LaunchAgent,
                    None,
                ))?;
                hotkeys::setup(app)?;
            }

            clipboard::spawn(app.handle().clone());

            Ok(())
        })
        .on_window_event(|window, event| {
            // Skrab lives in the tray. Closing the panel hides it; it does not quit.
            if let WindowEvent::CloseRequested { api, .. } = event {
                api.prevent_close();
                if let Err(e) = window.hide() {
                    log::error!("failed to hide window on close request: {e}");
                }
            }
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_app_info,
            commands::list_clips,
            commands::get_clip_content,
            commands::copy_clip,
            commands::set_clip_favorite,
            commands::set_clip_pinned,
            commands::delete_clip,
            commands::clear_history,
            commands::history_stats,
            commands::get_settings,
            commands::save_settings,
            commands::set_monitoring,
            commands::hide_panel,
            commands::open_data_dir,
        ])
        .run(tauri::generate_context!())
        .expect("error while running Skrab");
}
