//! Gadaj — Polish offline dictation.
//!
//! Architektura: Tauri 2 + Rust core + React/TS frontend.
//! Pipeline: capture audio → VAD → resample → parakeet.cpp → clipboard → paste.

mod audio;
mod commands;
mod history;
mod input;
mod models;
mod pipeline;
mod settings;
mod state;
mod stt;

use std::sync::Arc;
use tauri::{Manager, WindowEvent};
use tauri_plugin_log::{Target, TargetKind};

use crate::pipeline::Pipeline;
use crate::state::AppState;

pub fn run() {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info"))
        .format_timestamp_millis()
        .init();

    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _argv, _cwd| {
            if let Some(win) = app.get_webview_window("main") {
                let _ = win.show();
                let _ = win.unminimize();
                let _ = win.set_focus();
            }
        }))
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_os::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_store::Builder::default().build())
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_clipboard_manager::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            None,
        ))
        .plugin(
            tauri_plugin_log::Builder::new()
                .level(log::LevelFilter::Info)
                .targets([
                    Target::new(TargetKind::Stdout),
                    Target::new(TargetKind::LogDir { file_name: Some("gadaj".into()) }),
                ])
                .build(),
        )
        .setup(|app| {
            let app_state = AppState::new(app.handle().clone())?;
            app.manage(Arc::new(app_state));
            let pipeline = Pipeline::new(app.handle().clone());
            app.manage(pipeline);

            // pokaż okno po setup
            if let Some(win) = app.get_webview_window("main") {
                let _ = win.show();
            }

            Ok(())
        })
        .on_window_event(|window, event| {
            if let WindowEvent::CloseRequested { api, .. } = event {
                // Nie zamykaj, chowaj do traya (kiedy dodamy tray).
                // Na MVP chowamy okno zamiast zamykać proces.
                api.prevent_close();
                let _ = window.hide();
            }
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_settings,
            commands::save_settings,
            commands::get_available_models,
            commands::download_model,
            commands::delete_model,
            commands::get_history_entries,
            commands::delete_history_entry,
            commands::copy_to_clipboard,
            commands::get_mic_level,
            commands::is_model_loaded,
            commands::transcribe_file,
            commands::start_hotkey_listener,
            commands::stop_hotkey_listener,
            commands::show_window,
            commands::get_app_data_dir,
        ])
        .run(tauri::generate_context!())
        .expect("Błąd podczas uruchamiania aplikacji Gadaj");
}
