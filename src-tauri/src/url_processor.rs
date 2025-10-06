use ammonia::Builder;
use readability::extractor;
use std::collections::{HashMap, HashSet};
use url::Url;

/// Fetches a URL, extracts the main content, sanitizes it, and converts it to Markdown.
///
/// This function now uses a multi-stage process for higher quality output:
/// 1. Fetch the URL using `reqwest`.
/// 2. Extract the core article content using `readability`.
/// 3. Sanitize the extracted HTML using `ammonia`, allowing only a curated set of
///    tags and attributes suitable for a clean reading experience. This removes
///    scripts, styles, and unwanted clutter.
/// 4. Convert the clean HTML to Markdown using `html2md`.
///
/// # Arguments
/// * `url_str` - The URL of the article to process.
///
/// # Returns
/// A `Result` containing the processed Markdown string on success, or an error string on failure.
pub async fn process_url(url_str: &str) -> Result<String, String> {
    let url = Url::parse(url_str).map_err(|e| format!("Неверный URL: {}", e))?;

    let client = reqwest::Client::builder()
        .user_agent("Mozilla/5.0 (X11; Linux x86_64; rv:109.0) Gecko/20100101 Firefox/115.0")
        .timeout(std::time::Duration::from_secs(20))
        .build()
        .map_err(|e| format!("Ошибка создания HTTP клиента: {}", e))?;

    let response = client
        .get(url.clone())
        .send()
        .await
        .map_err(|e| format!("Ошибка загрузки страницы: {}", e))?;

    if !response.status().is_success() {
        return Err(format!(
            "Ошибка загрузки: сервер ответил со статусом {}",
            response.status()
        ));
    }

    let content_bytes = response
        .bytes()
        .await
        .map_err(|e| format!("Ошибка чтения тела ответа: {}", e))?;

    let mut reader = &content_bytes[..];
    let product = extractor::extract(&mut reader, &url)
        .map_err(|e| format!("Ошибка извлечения контента: {}", e))?;

    let extracted_html = product.content;
    if extracted_html.trim().is_empty() {
        return Err("Не удалось извлечь основное содержимое со страницы.".to_string());
    }

    // CORRECTED: `tag_attributes` expects a single HashMap argument.
    let mut tag_attrs = HashMap::new();
    tag_attrs.insert("a", ["href"].iter().cloned().collect::<HashSet<_>>());
    tag_attrs.insert(
        "img",
        ["src", "alt", "title"]
            .iter()
            .cloned()
            .collect::<HashSet<_>>(),
    );

    // Sanitize the extracted HTML to keep only essential tags for reading.
    let cleaned_html = Builder::new()
        .tags(
            [
                "h1", "h2", "h3", "h4", "h5", "h6", "p", "br", "hr", "strong", "em", "b", "i",
                "u", "del", "s", "strike", "blockquote", "ul", "ol", "li", "pre", "code", "img",
                "figure", "figcaption", "table", "thead", "tbody", "tr", "th", "td", "a",
            ]
            .iter()
            .cloned()
            .collect::<HashSet<_>>(),
        )
        .tag_attributes(tag_attrs) // CORRECTED: Pass the HashMap here.
        .link_rel(None) // Don't add rel="noopener noreferrer"
        // CORRECTED: `strip_unallowed_tags` does not exist; stripping is the default behavior.
        .clean(&extracted_html)
        .to_string();

    // CORRECTED: Use the original `html2md` crate's `parse_html` function.
    let markdown = html2md::parse_html(&cleaned_html);

    if markdown.trim().is_empty() {
        return Err("Извлеченное содержимое оказалось пустым после обработки.".to_string());
    }

    let title_md = if !product.title.is_empty() {
        format!("# {}\n\n", product.title.trim())
    } else {
        String::new()
    };

    Ok(format!("{}{}", title_md, markdown.trim()))
}
