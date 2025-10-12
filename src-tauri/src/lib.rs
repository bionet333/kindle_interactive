#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use crate::state::AppState;
use std::sync::Arc;
use tauri::Manager;
use tauri_plugin_log::{Target, TargetKind, TimezoneStrategy};

mod clipboard;
mod commands;
mod core;
mod network;
mod server;
mod state;
mod url_processor;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let app_state = Arc::new(AppState::default());

    let log_plugin = tauri_plugin_log::Builder::new()
        .targets([
            Target::new(TargetKind::Stdout),
            Target::new(TargetKind::Webview),
            Target::new(TargetKind::LogDir {
                file_name: Some("ki.log".into()),
            }),
        ])
        .timezone_strategy(TimezoneStrategy::UseLocal)
        .build();

    tauri::Builder::default()
        .manage(app_state)
        .plugin(log_plugin)
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            let managed_state = app.state::<Arc<AppState>>().inner().clone();
            let app_handle = app.handle().clone();

            // Spawn the web server in a background async task.
            let server_state = managed_state.clone();
            tauri::async_runtime::spawn(async move {
                server::run_server(server_state).await;
            });

            // Spawn the clipboard monitor in a dedicated background thread.
            let clipboard_state = managed_state;
            clipboard::spawn_monitor(clipboard_state, app_handle);

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_text,
            commands::set_text,
            commands::get_server_info,
            commands::set_send_on_copy,
            commands::set_add_to_editor_on_copy
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
