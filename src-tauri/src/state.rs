use std::sync::atomic::AtomicBool;
use std::sync::{Arc, RwLock};

/// The shared, thread-safe state of the application.
/// This struct holds all data that needs to be accessed by both the
/// Tauri commands (from the UI) and the Axum web server.
pub struct AppState {
    /// The Markdown text content shared between the editor and the web reader.
    /// Wrapped in `RwLock` to allow for concurrent reads and exclusive writes.
    pub shared_text: RwLock<String>,
    /// A flag to enable or disable clipboard monitoring. It's atomic to allow
    /// for safe, lock-free access from the UI command and the monitoring thread.
    pub monitor_clipboard: Arc<AtomicBool>,
}

impl Default for AppState {
    /// Provides a default initial state for the application.
    fn default() -> Self {
        Self {
            shared_text: RwLock::new(
                "## Добро пожаловать!\n\nЭто редактор для вашей E-Ink читалки. Введите текст в формате Markdown здесь, и он появится на странице, которую вы откроете на читалке.".to_string(),
            ),
            monitor_clipboard: Arc::new(AtomicBool::new(false)),
        }
    }
}
