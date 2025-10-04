#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use crate::state::AppState;
use std::sync::Arc;
use tauri::Manager;
// CORRECTED: Import `Target` and `TargetKind` for the updated tauri-plugin-log v2 API.
use tauri_plugin_log::{Target, TargetKind, TimezoneStrategy};

mod commands;
mod core;
mod network;
mod server;
mod state;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let app_state = Arc::new(AppState::default());

    // Initialize the logger plugin using the correct API for Tauri v2.
    // The plugin is configured here and then registered on the main `tauri::Builder`.
    let log_plugin = tauri_plugin_log::Builder::new()
        // CORRECTED: Each target must be created with `Target::new` and an enum variant from `TargetKind`.
        .targets([
            Target::new(TargetKind::Stdout),
            Target::new(TargetKind::Webview),
            // `TargetKind::LogDir` automatically resolves the correct application log directory.
            Target::new(TargetKind::LogDir {
                file_name: Some("kindle-app.log".into()),
            }),
        ])
        .timezone_strategy(TimezoneStrategy::UseLocal)
        .build();

tauri::Builder::default()
        // The state is moved into Tauri's state manager. Both the server and commands will access it from here.
        .manage(app_state)
        .plugin(log_plugin)
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            // Retrieve the state from Tauri's manager to ensure the server and commands
            // use the exact same state instance.
            let server_state = app.state::<Arc<AppState>>().inner().clone();

            // Spawn the web server in a background async task.
            tauri::async_runtime::spawn(async move {
                server::run_server(server_state).await;
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_text,
            // `set_text` command removed, functionality is now handled by an HTTP endpoint.
            commands::get_server_info
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
