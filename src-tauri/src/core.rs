use sha1::{Digest, Sha1};

/// Processes a Markdown string into HTML and computes its SHA1 hash.
/// This function is central to determining if the content has changed.
///
/// # Arguments
/// * `markdown_text` - A string slice containing the Markdown text.
///
/// # Returns
/// A tuple containing:
/// * `String` - The generated HTML.
/// * `String` - The hex-encoded SHA1 hash of the HTML.
pub fn process_markdown(markdown_text: &str) -> (String, String) {
    let html_content = markdown::to_html_with_options(markdown_text, &markdown::Options::gfm())
        .unwrap_or_else(|e| format!("<p>Markdown processing error: {}</p>", e));

    let mut hasher = Sha1::new();
    hasher.update(html_content.as_bytes());
    let hash_result = hasher.finalize();
    let current_hash = format!("{:x}", hash_result);

    (html_content, current_hash)
}
