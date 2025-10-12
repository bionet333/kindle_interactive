use crate::state::AppState;
use arboard::Clipboard;
use log::{error, info, warn};
use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::thread;
use std::time::Duration;
use tauri::Emitter;

/// Spawns a dedicated thread to monitor the system clipboard for changes.
///
/// Depending on the `AppState` flags, this function can:
/// 1. Directly replace the shared text for the e-reader.
/// 2. Emit an event to the frontend to add the text to the editor.
pub fn spawn_monitor(state: Arc<AppState>, handle: tauri::AppHandle) {
    thread::spawn(move || {
        info!("Clipboard monitoring thread started.");
        let mut clipboard = match Clipboard::new() {
            Ok(cb) => cb,
            Err(e) => {
                error!("Failed to initialize clipboard: {}. Thread will exit.", e);
                return;
            }
        };

        let mut last_text = clipboard.get_text().unwrap_or_default();

        loop {
            let send_enabled = state.send_on_copy.load(Ordering::Relaxed);
            let add_to_editor_enabled = state.add_to_editor_on_copy.load(Ordering::Relaxed);

            if !send_enabled && !add_to_editor_enabled {
                thread::sleep(Duration::from_millis(500));
                continue;
            }

            match clipboard.get_text() {
                Ok(current_text) => {
                    if !current_text.trim().is_empty() && current_text != last_text {
                        if send_enabled {
                            info!("New text detected. Sending to e-reader.");
                            match state.shared_text.write() {
                                Ok(mut shared_text) => {
                                    *shared_text = current_text.clone();
                                    last_text = current_text;
                                }
                                Err(e) => {
                                    error!("Failed to lock shared_text for sending: {}", e);
                                }
                            }
                        } else if add_to_editor_enabled {
                            info!("New text detected. Emitting event to add to editor.");
                            if let Err(e) = handle.emit("clipboard-add-to-editor", &current_text) {
                                error!("Failed to emit clipboard event: {}", e);
                            }
                            last_text = current_text;
                        }
                    }
                }
                Err(e) => {
                    warn!("Could not read text from clipboard: {}", e);
                    last_text.clear();
                }
            }

            thread::sleep(Duration::from_millis(500));
        }
    });
}
