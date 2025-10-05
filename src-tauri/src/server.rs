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
            "{{ initial_content_json }}",
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
        html, body { 
            margin: 0; 
            padding: 0; 
            width: 100%;
            height: 100%; 
            overflow: hidden; /* Prevent vertical scrollbar */
            font-family: 'Georgia', serif; 
            color: #111; 
            background-color: #fdfdfd; 
        }

        #content-wrapper {
            /* Это наш вьюпорт для прокрутки. Он должен быть равен ширине экрана. */
            height: calc(100vh - 40px);
            width: 100vw;
            overflow: hidden;
            scroll-snap-type: x mandatory;
        }

        #content-container {
            /* Это широкий элемент с колонками. */
            height: 100%;
            
            /* Отступы по бокам ДОЛЖНЫ быть здесь. Это создает отступы для первой и последней страницы. */
            padding-left: 25px;
            padding-right: 25px;
            box-sizing: border-box;
            
            /* Ширина КОНТЕНТА внутри одной колонки. */
            column-width: calc(100vw - 50px);
            
            /* Промежуток МЕЖДУ колонками. */
            column-gap: 50px;
            
            /* Стандартные стили текста */
            font-size: 1.3em; 
            line-height: 1.6;
            text-align: justify;
        }

        #content-container::after {
            content: '';
            display: block; /* Важно использовать block, чтобы он занял свою колонку */
            width: calc(100vw - 50px); /* Ширина контента одной страницы */
            height: 1px; /* Минимальная высота, чтобы элемент существовал */
            break-before: column; /* Гарантируем, что он всегда начнет новую колонку */
        }
                
        /* Rules to prevent elements from breaking across columns (pages) */
        #content-container h1, 
        #content-container h2, 
        #content-container h3,
        #content-container pre, 
        #content-container blockquote, 
        #content-container table, 
        #content-container img,
        #content-container figure {
            break-inside: avoid;
        }
        
        #content-container p {
            widows: 2;
            orphans: 2;
        }
        
        #content-container h1, #content-container h2, #content-container h3 { 
            line-height: 1.2; 
            text-align: left;
        }

        #content-container img { 
            max-width: 100%; 
            height: auto; 
        }
        
        #content-container blockquote { 
            border-left: 4px solid #ccc; 
            padding-left: 1em; 
            margin-left: 0; 
        }
        #content-container pre, #content-container code { 
            white-space: pre-wrap !important; 
            word-break: break-word;
            font-size: 0.85em; 
            background-color: #f3f3f3; 
            border-radius: 4px; 
            padding: 2px 4px;
            text-align: left;
        }
        #content-container pre { 
            padding: 1em; 
            overflow-x: auto;
        }

        /* UI Bar styling (unchanged) */
        #ui-bar { 
            height: 40px; 
            position: fixed; 
            bottom: 0; 
            left: 0; 
            width: 100%; 
            background-color: rgba(255, 255, 255, 0.9); 
            border-top: 1px solid #ddd; 
            display: flex; 
            justify-content: center; 
            align-items: center; 
            box-sizing: border-box; 
            padding: 0 1em; 
            user-select: none; 
            font-family: sans-serif; 
            color: #555; 
        }
    </style>
</head>
<body>
    <div id="content-wrapper">
        <div id="content-container"></div>
    </div>
    <div id="ui-bar"><div id="page-counter"></div></div>
    
    <script>
        let currentPage = 0;
        let totalPages = 0;
        let currentHash = "{{ initial_hash }}";
        let isUpdating = false;

        const wrapper = document.getElementById('content-wrapper');
        const container = document.getElementById('content-container');
        const pageCounter = document.getElementById('page-counter');
        
        function updateLayout() {
            // Используем Math.ceil для подсчета. Если контент занимает 2.1 страницы,
            // нам нужно 3 "экрана" для его отображения. Это самый надежный способ.
            const realTotalPages = Math.ceil(container.scrollWidth / wrapper.clientWidth);

            // Количество страниц для пользователя = реальное количество минус одна (фиктивная).
            totalPages = Math.max(1, realTotalPages - 1);

            // Ограничиваем currentPage, чтобы пользователь не мог перейти на фиктивную страницу.
            currentPage = Math.max(0, Math.min(currentPage, totalPages - 1));
            
            updateUi();
        }

        function updateUi() {
            if (totalPages > 0) {
                pageCounter.textContent = `Страница ${currentPage + 1} из ${totalPages}`;
                
                // Больше никаких сложных формул!
                // Просто прокручиваем на N экранов. Браузер сам справится с позиционированием.
                const scrollLeftPosition = currentPage * wrapper.clientWidth;

                wrapper.scrollTo({
                    left: scrollLeftPosition,
                    behavior: 'auto'
                });
            } else {
                pageCounter.textContent = 'Нет страниц';
            }
        }

        function showPage(pageIndex) {
            if (isUpdating || pageIndex < 0 || pageIndex >= totalPages) return;
            currentPage = pageIndex;
            updateUi();
        }

        function setupNavigation() {
            document.body.addEventListener('click', (event) => {
                if (event.target.closest('#ui-bar') || event.button !== 0) return;
                
                const rect = document.body.getBoundingClientRect();
                if (event.clientX > rect.left + rect.width / 2) {
                    showPage(currentPage + 1);
                } else {
                    showPage(currentPage - 1);
                }
            });
        }

        async function checkForUpdates() {
            if (isUpdating) return;
            try {
                const response = await fetch(`/api/content?_=${new Date().getTime()}`);
                if (!response.ok) return;
                const data = await response.json();
                
                if (data.hash !== currentHash) {
                    isUpdating = true;
                    console.log("Получено обновление контента. Новый хэш:", data.hash);
                    currentHash = data.hash;
                    
                    container.innerHTML = data.html;
                    
                    setTimeout(() => {
                        currentPage = 0; // Сброс на первую страницу при обновлении
                        updateLayout();
                        isUpdating = false;
                    }, 100); 
                }
            } catch (error) {
                console.error('Ошибка при проверке обновлений:', error);
                isUpdating = false;
            }
        }

        function initialize(initialContent) {
            isUpdating = true;
            container.innerHTML = initialContent;
            
            setTimeout(() => {
                updateLayout();
                setupNavigation();
                setInterval(checkForUpdates, 3000);
                isUpdating = false;
            }, 100);

            let resizeTimeout;
            window.addEventListener('resize', () => {
                clearTimeout(resizeTimeout);
                resizeTimeout = setTimeout(updateLayout, 250);
            });
        }
        
        document.addEventListener('DOMContentLoaded', () => {
            initialize({{ initial_content_json }});
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
