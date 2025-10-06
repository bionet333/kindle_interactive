use crate::network::get_local_ip_address;
use crate::server::SERVER_PORT;
use crate::state::AppState;
use std::sync::atomic::Ordering;
use std::sync::Arc;
use tauri::State;

/// Retrieves the current shared text from the application state.
#[tauri::command]
pub fn get_text(state: State<Arc<AppState>>) -> Result<String, String> {
    state
        .shared_text
        .read()
        .map(|text| text.clone())
        .map_err(|e| format!("Failed to acquire read lock: {}", e))
}

// The `set_text` command has been removed. The frontend now communicates
// directly with the embedded Axum server via an HTTP POST request to update the content.
// This decouples the UI from the Tauri command system for this specific action,
// making it a standard web interaction that is easier to debug.

/// Gets the local network address for the web reader.
#[tauri::command]
pub fn get_server_info() -> Result<String, String> {
    match get_local_ip_address() {
        Some(ip) => Ok(format!(
            "Откройте на читалке: http://{}:{}/get",
            ip, SERVER_PORT
        )),
        None => Ok("Не удалось определить IP-адрес. Проверьте подключение к сети.".to_string()),
    }
}

/// Enables or disables automatic clipboard monitoring.
#[tauri::command]
pub fn set_clipboard_monitoring(enabled: bool, state: State<Arc<AppState>>) -> Result<(), String> {
    state
        .monitor_clipboard
        .store(enabled, Ordering::Relaxed);
    log::info!("Clipboard monitoring set to: {}", enabled);
    Ok(())
}
