use std::sync::atomic::AtomicBool;
use std::sync::{Arc, RwLock};

/// The shared, thread-safe state of the application.
pub struct AppState {
    /// The Markdown text content shared with the web reader.
    pub shared_text: RwLock<String>,
    /// Flag to enable replacing shared text with clipboard content (sends to e-reader).
    pub send_on_copy: Arc<AtomicBool>,
    /// Flag to enable appending clipboard content to the editor (does not send).
    pub add_to_editor_on_copy: Arc<AtomicBool>,
}

impl Default for AppState {
    /// Provides a default initial state for the application.
    fn default() -> Self {
        Self {
            shared_text: RwLock::new(
                "## Добро пожаловать!\n\nЭто редактор для вашей E-Ink читалки. Введите текст в формате Markdown здесь, и он появится на странице, которую вы откроете на читалке.".to_string(),
            ),
            send_on_copy: Arc::new(AtomicBool::new(false)),
            add_to_editor_on_copy: Arc::new(AtomicBool::new(false)),
        }
    }
}
