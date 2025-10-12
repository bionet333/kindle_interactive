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

/// Overwrites the shared text with new content. This is now the primary method for updating
/// the state from the frontend to ensure consistency.
#[tauri::command]
pub fn set_text(new_text: String, state: State<Arc<AppState>>) -> Result<(), String> {
    log::info!("Setting shared text via command.");
    match state.shared_text.write() {
        Ok(mut text) => {
            *text = new_text;
            log::info!("Successfully set shared text from command.");
            Ok(())
        }
        Err(e) => {
            let err_msg = format!("Failed to acquire write lock for set_text: {}", e);
            log::error!("{}", err_msg);
            Err(err_msg)
        }
    }
}

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

/// Enables or disables automatically sending clipboard text to the e-reader.
#[tauri::command]
pub fn set_send_on_copy(enabled: bool, state: State<Arc<AppState>>) -> Result<(), String> {
    state.send_on_copy.store(enabled, Ordering::Relaxed);
    log::info!("Send on copy set to: {}", enabled);
    Ok(())
}

/// Enables or disables automatically adding clipboard text to the editor.
#[tauri::command]
pub fn set_add_to_editor_on_copy(enabled: bool, state: State<Arc<AppState>>) -> Result<(), String> {
    state
        .add_to_editor_on_copy
        .store(enabled, Ordering::Relaxed);
    log::info!("Add to editor on copy set to: {}", enabled);
    Ok(())
}
