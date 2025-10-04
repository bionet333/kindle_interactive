use crate::{core::process_markdown, state::AppState};
use axum::{
    extract::State,
    http::{
        header::{CACHE_CONTROL, CONTENT_TYPE, EXPIRES, PRAGMA},
        HeaderMap,
        Method,
        StatusCode,
    },
    response::{Html, IntoResponse, Json},
    routing::{get, post},
    Router,
};
use log::{error, info, warn};
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::TcpListener;
use tower_http::cors::{Any, CorsLayer};

/// The port on which the web server will listen.
pub const SERVER_PORT: u16 = 5001;

#[derive(Serialize, Deserialize, Debug, PartialEq)]
struct ContentResponse {
    html: String,
    hash: String,
}

// Payload for the POST /api/content endpoint.
#[derive(Deserialize, Debug)]
struct SetTextPayload {
    new_text: String,
}

/// Initializes and runs the Axum web server.
pub async fn run_server(app_state: Arc<AppState>) {
    // Explicitly configure CORS to allow POST requests with a JSON content type from any origin.
    // This is crucial for the Yew web UI to be able to save content via HTTP request.
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods([Method::GET, Method::POST])
        .allow_headers([CONTENT_TYPE]);

    let app = Router::new()
        .route("/get", get(get_page_handler))
        // The /api/content route now handles both GET for polling and POST for updates.
        .route(
            "/api/content",
            get(api_content_handler).post(api_set_content_handler),
        )
        .with_state(app_state)
        .layer(cors);

    let addr = SocketAddr::from(([0, 0, 0, 0], SERVER_PORT));
    info!("🚀 E-Ink server listening on http://{}/get", addr);

    if let Ok(listener) = TcpListener::bind(addr).await {
        if let Err(e) = axum::serve(listener, app).await {
            error!("Server error: {}", e);
        }
    } else {
        error!("Failed to bind to address {}", addr);
    }
}

/// Returns a HeaderMap with directives to prevent caching.
fn no_cache_headers() -> HeaderMap {
    let mut headers = HeaderMap::new();
    headers.insert(
        CACHE_CONTROL,
        "no-cache, no-store, must-revalidate".parse().unwrap(),
    );
    headers.insert(PRAGMA, "no-cache".parse().unwrap());
    headers.insert(EXPIRES, "0".parse().unwrap());
    headers
}

/// Handler for the `/get` route, serving the main reader page.
async fn get_page_handler(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    info!("Request received for initial page /get");
    let shared_text = match state.shared_text.read() {
        Ok(guard) => guard.clone(),
        Err(e) => {
            error!("Failed to acquire read lock for /get: {}", e);
            let error_html = "<h1>Ошибка на сервере</h1><p>Не удалось загрузить содержимое. Пожалуйста, перезапустите приложение.</p>";
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                no_cache_headers(),
                Html(error_html.to_string()),
            )
                .into_response();
        }
    };

    let (initial_content, initial_hash) = process_markdown(&shared_text);
    info!("Serving initial page with hash: {}", initial_hash);

    let html_template = GET_TEMPLATE
        .replace("{{ initial_hash }}", &initial_hash)
        .replace(
            "{{ initial_content | tojson }}",
            &serde_json::to_string(&initial_content).unwrap_or_else(|_| "''".to_string()),
        );

    (no_cache_headers(), Html(html_template)).into_response()
}

/// Handler for the `/api/content` route, providing content updates.
async fn api_content_handler(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    info!("Polling request received for /api/content");
    let shared_text = match state.shared_text.read() {
        Ok(guard) => guard.clone(),
        Err(e) => {
            warn!("Failed to acquire read lock for /api/content: {}", e);
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos();
            let error_response = ContentResponse {
                html: "<h2>Ошибка на сервере</h2><p>Не удалось получить доступ к данным. Попробуйте перезапустить приложение.</p>".to_string(),
                hash: format!("error-{}", now),
            };
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                no_cache_headers(),
                Json(error_response),
            )
                .into_response();
        }
    };

    let (html_content, current_hash) = process_markdown(&shared_text);

    let response = ContentResponse {
        html: html_content,
        hash: current_hash,
    };

    (StatusCode::OK, no_cache_headers(), Json(response)).into_response()
}

/// Handler for the `POST /api/content` route, updating the shared text.
async fn api_set_content_handler(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<SetTextPayload>,
) -> impl IntoResponse {
    info!("Request received to update content via POST /api/content");
    match state.shared_text.write() {
        Ok(mut text) => {
            *text = payload.new_text;
            info!("Successfully updated shared text from API.");
            (StatusCode::OK, Json("Content updated successfully."))
        }
        Err(e) => {
            error!("Failed to acquire write lock for /api/content: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json("Failed to update content due to a server error."),
            )
        }
    }
}


const GET_TEMPLATE: &str = r#"
<!DOCTYPE html>
<html lang="ru">
<head>
    <meta charset="UTF-8">
    <title>Текст для чтения</title>
    <meta name="viewport" content="width=device-width, initial-scale=1.0, user-scalable=no">
    <style>
        html, body { margin: 0; padding: 0; height: 100%; overflow: hidden; font-family: 'Georgia', serif; color: #111; background-color: #fdfdfd; }
        #book-viewport { height: calc(100% - 40px); overflow: hidden; }
        #ui-bar { height: 40px; position: fixed; bottom: 0; left: 0; width: 100%; background-color: rgba(255, 255, 255, 0.9); border-top: 1px solid #ddd; display: flex; justify-content: center; align-items: center; box-sizing: border-box; padding: 0 1em; user-select: none; font-family: sans-serif; color: #555; }
        #book-pages-container { display: flex; height: 100%; }
        .page { flex-shrink: 0; width: 100%; height: 100%; box-sizing: border-box; padding: 1em 1.5em; overflow: hidden; font-size: 1.3em; line-height: 1.6; }
        .page h1, .page h2, .page h3 { line-height: 1.2; }
        .page img { max-width: 100%; height: auto; }
        .page blockquote { border-left: 4px solid #ccc; padding-left: 1em; margin-left: 0; }
        .page pre, .page code { white-space: pre-wrap !important; word-break: break-all; font-size: 0.85em; background-color: #f3f3f3; border-radius: 4px; padding: 2px 4px; }
        .page pre { padding: 1em; }
    </style>
</head>
<body>
    <div id="book-viewport"><div id="book-pages-container"></div></div>
    <div id="ui-bar"><div id="page-counter"></div></div>
    <script>
        let currentPage = 0;
        let totalPages = 0;
        let currentHash = "{{ initial_hash }}";
        const viewport = document.getElementById('book-viewport');
        const pagesContainer = document.getElementById('book-pages-container');
        const pageCounter = document.getElementById('page-counter');
        function paginate(sourceHtml) {
            pagesContainer.innerHTML = '';
            const sourceDiv = document.createElement('div');
            sourceDiv.innerHTML = sourceHtml;
            const elements = Array.from(sourceDiv.children);
            if (elements.length === 0) {
                pagesContainer.innerHTML = '<div class="page"><p>Нет текста.</p></div>';
                totalPages = 1; return;
            }
            const pageHeight = viewport.offsetHeight;
            let currentPageHTML = '';
            const pagesContent = [];
            const measurePage = document.createElement('div');
            measurePage.className = 'page';
            measurePage.style.visibility = 'hidden';
            measurePage.style.position = 'absolute';
            measurePage.style.height = 'auto';
            document.body.appendChild(measurePage);
            for (const el of elements) {
                const testHTML = currentPageHTML + el.outerHTML;
                measurePage.innerHTML = testHTML;
                if (measurePage.scrollHeight > pageHeight && currentPageHTML !== '') {
                    pagesContent.push(currentPageHTML);
                    currentPageHTML = el.outerHTML;
                } else { currentPageHTML = testHTML; }
            }
            if (currentPageHTML !== '') { pagesContent.push(currentPageHTML); }
            document.body.removeChild(measurePage);
            pagesContent.forEach(pageHtml => {
                const pageDiv = document.createElement('div');
                pageDiv.className = 'page';
                pageDiv.innerHTML = pageHtml;
                pagesContainer.appendChild(pageDiv);
            });
            totalPages = pagesContent.length;
        }
        function showPage(pageIndex) {
            if (pageIndex < 0 || pageIndex >= totalPages) return;
            currentPage = pageIndex;
            pagesContainer.style.transform = `translateX(-${currentPage * 100}%)`;
            pageCounter.textContent = totalPages > 0 ? `Страница ${currentPage + 1} из ${totalPages}` : '';
        }
        function setupNavigation() {
            viewport.addEventListener('click', (event) => {
                if (event.target.closest('#ui-bar')) return;
                if (event.clientX > window.innerWidth / 2) { showPage(currentPage + 1); } else { showPage(currentPage - 1); }
            });
        }
        async function checkForUpdates() {
            try {
                // Use a cache-busting query parameter for robustness, though server headers should suffice
                const response = await fetch(`/api/content?_=${new Date().getTime()}`);
                if (!response.ok) return;
                const data = await response.json();
                if (data.hash !== currentHash) {
                    currentHash = data.hash;
                    paginate(data.html);
                    showPage(0);
                }
            } catch (error) { console.error('Ошибка при проверке обновлений:', error); }
        }
        document.addEventListener('DOMContentLoaded', () => {
            paginate({{ initial_content | tojson }});
            showPage(0);
            setupNavigation();
            setInterval(checkForUpdates, 3000);
            let resizeTimeout;
            window.addEventListener('resize', () => {
                clearTimeout(resizeTimeout);
                resizeTimeout = setTimeout(() => {
                    const allPageHtml = Array.from(document.querySelectorAll('.page')).map(p => p.innerHTML).join('');
                    paginate(allPageHtml);
                    showPage(Math.min(currentPage, totalPages - 1));
                }, 250);
            });
        });
    </script>
</body>
</html>
"#;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::AppState;
    use axum::{body::Body, http::Request};
    use http_body_util::BodyExt;
    use serde::Deserialize;
    use tower::ServiceExt; // for `oneshot`

    // Helper to build the app router for testing
    fn test_app_router() -> Router {
        let app_state = Arc::new(AppState::default());
        Router::new()
            .route("/get", get(get_page_handler))
            .route("/api/content", get(api_content_handler))
            .with_state(app_state)
    }

    #[tokio::test]
    async fn api_content_handler_returns_no_cache_headers() {
        let app = test_app_router();

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/api/content")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(
            response.headers().get(CACHE_CONTROL).unwrap(),
            "no-cache, no-store, must-revalidate"
        );
        assert_eq!(response.headers().get(PRAGMA).unwrap(), "no-cache");
        assert_eq!(response.headers().get(EXPIRES).unwrap(), "0");
    }

    #[tokio::test]
    async fn get_page_handler_returns_no_cache_headers() {
        let app = test_app_router();

        let response = app
            .oneshot(Request::builder().uri("/get").body(Body::empty()).unwrap())
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(
            response.headers().get(CACHE_CONTROL).unwrap(),
            "no-cache, no-store, must-revalidate"
        );
        assert_eq!(response.headers().get(PRAGMA).unwrap(), "no-cache");
        assert_eq!(response.headers().get(EXPIRES).unwrap(), "0");
    }

    #[tokio::test]
    async fn api_content_handler_returns_json_with_correct_structure() {
        let app = test_app_router();

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/api/content")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        let body = response.into_body().collect().await.unwrap().to_bytes();
        let content_response: ContentResponse = serde_json::from_slice(&body).unwrap();

        // Check if fields exist and have expected types (from default state)
        let (expected_html, expected_hash) = process_markdown(&AppState::default().shared_text.read().unwrap());
        
        assert_eq!(content_response.html, expected_html);
        assert_eq!(content_response.hash, expected_hash);
    }
}
