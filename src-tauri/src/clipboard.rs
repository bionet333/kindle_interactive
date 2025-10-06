use crate::state::AppState;
use arboard::Clipboard;
use log::{error, info, warn};
use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::thread;
use std::time::Duration;

/// Spawns a dedicated thread to monitor the system clipboard for changes.
///
/// When monitoring is enabled and new, non-empty text is found, it updates
/// the shared application state.
pub fn spawn_monitor(state: Arc<AppState>) {
    thread::spawn(move || {
        info!("Clipboard monitoring thread started.");
        let mut clipboard = match Clipboard::new() {
            Ok(cb) => cb,
            Err(e) => {
                error!("Failed to initialize clipboard: {}. Thread will exit.", e);
                return;
            }
        };

        // Store the last text seen to avoid redundant updates.
        let mut last_text = clipboard.get_text().unwrap_or_default();

        loop {
            // Only proceed if monitoring is enabled.
            if state.monitor_clipboard.load(Ordering::Relaxed) {
                match clipboard.get_text() {
                    Ok(current_text) => {
                        // Check if the text is new, not empty, and different from the last.
                        if !current_text.trim().is_empty() && current_text != last_text {
                            info!("New text detected in clipboard. Updating shared state.");
                            match state.shared_text.write() {
                                Ok(mut shared_text) => {
                                    *shared_text = current_text.clone();
                                    last_text = current_text;
                                }
                                Err(e) => {
                                    error!("Failed to acquire write lock for shared_text: {}", e);
                                }
                            }
                        }
                    }
                    Err(e) => {
                        // This can happen if the clipboard content is not text (e.g., an image).
                        // It's not a critical error, so we log it as a warning and clear last_text
                        // to allow re-copying of text after a non-text item.
                        warn!("Could not read text from clipboard: {}", e);
                        last_text.clear();
                    }
                }
            }
            // Poll every 500ms. This is a reasonable balance to be responsive
            // without excessive CPU usage.
            thread::sleep(Duration::from_millis(500));
        }
    });
}
