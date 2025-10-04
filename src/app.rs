use serde::{Deserialize, Serialize};
use wasm_bindgen::{prelude::*, JsCast};
use wasm_bindgen_futures::spawn_local;
use web_sys::{Headers, Request, RequestInit, Response};
use yew::prelude::*;

// The Tauri `invoke` is still used for reading initial state.
#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = ["window", "__TAURI__", "core"])]
    async fn invoke(cmd: &str, args: JsValue) -> JsValue;
}

// Payload for the HTTP POST request. Uses an owned String to be 'static,
// which is safer for async operations.
#[derive(Serialize, Deserialize)]
struct SetTextArgs {
    new_text: String,
}

/// The root component of the Yew frontend application.
#[function_component(App)]
pub fn app() -> Html {
    let editor_content = use_state(String::new);
    let server_info = use_state(|| "Загрузка информации о сервере...".to_string());
    let save_status = use_state(String::new);
    let is_saving = use_state(|| false);

    // Effect to load initial data from the Rust backend when the component mounts.
    {
        let editor_content = editor_content.clone();
        let server_info = server_info.clone();
        use_effect_with((), move |_| {
            spawn_local(async move {
                // Fetch initial text for the editor via Tauri command
                let text = invoke("get_text", JsValue::NULL)
                    .await
                    .as_string()
                    .unwrap_or_default();
                editor_content.set(text);

                // Fetch server network information via Tauri command
                let info = invoke("get_server_info", JsValue::NULL)
                    .await
                    .as_string()
                    .unwrap_or_else(|| "Ошибка получения информации о сервере".to_string());
                server_info.set(info);
            });
            // The return is a teardown function, empty in this case.
            || {}
        });
    }

    // Callback for textarea input changes.
    let on_input = {
        let editor_content = editor_content.clone();
        let save_status = save_status.clone();
        Callback::from(move |e: InputEvent| {
            let value = e.target_unchecked_into::<web_sys::HtmlTextAreaElement>().value();
            editor_content.set(value);
            save_status.set("".to_string()); // Clear save status on edit
        })
    };

    // Callback for the save button click, now using a direct HTTP POST request.
    // This bypasses the Tauri `invoke` layer for this action, making it a standard
    // web request that can be easily debugged in the browser's network tab.
    let on_save = {
        let editor_content = editor_content.clone();
        let save_status = save_status.clone();
        let is_saving = is_saving.clone();
        Callback::from(move |e: MouseEvent| {
            e.prevent_default();
            if *is_saving {
                return;
            }

            is_saving.set(true);
            save_status.set("Сохранение...".to_string());

            let content_to_save = (*editor_content).clone();
            let save_status_clone = save_status.clone();
            let is_saving_clone = is_saving.clone();

            spawn_local(async move {
                let payload = SetTextArgs {
                    new_text: content_to_save,
                };
                let payload_js_value = serde_wasm_bindgen::to_value(&payload).unwrap_or(JsValue::NULL);

                let headers = Headers::new().unwrap();
                headers.set("Content-Type", "application/json").unwrap();

                let mut opts = RequestInit::new();
                opts.method("POST");
                opts.body(Some(&js_sys::JSON::stringify(&payload_js_value).unwrap()));
                opts.headers(&headers);

                // The URL points to the local Axum server, which must be configured to accept this request.
                let url = "http://localhost:5001/api/content";
                let request = Request::new_with_str_and_init(url, &opts).unwrap();

                let window = web_sys::window().unwrap();
                let resp_value = wasm_bindgen_futures::JsFuture::from(window.fetch_with_request(&request)).await;

                match resp_value {
                    Ok(resp) => {
                        let resp: Response = resp.dyn_into().unwrap();
                        if resp.ok() {
                            save_status_clone.set("Сохранено!".to_string());
                        } else {
                            let status = resp.status();
                            let status_text = resp.status_text();
                            let error_msg = format!("Ошибка сохранения: {} {}", status, status_text);
                            save_status_clone.set(error_msg);
                        }
                    }
                    Err(_) => {
                        save_status_clone.set("Ошибка сети. Сервер доступен?".to_string());
                    }
                }

                is_saving_clone.set(false);
            });
        })
    };


    html! {
        <main class="container">
            <div class="server-info">
                <p>{ &*server_info }</p>
            </div>

            <div class="editor-wrapper">
                <textarea
                    class="editor-textarea"
                    value={(*editor_content).clone()}
                    oninput={on_input}
                    placeholder="Введите ваш Markdown-текст здесь..."
                />
            </div>

            <div class="controls">
                <button onclick={on_save} disabled={*is_saving}>
                    { if *is_saving { "Сохранение..." } else { "Сохранить и обновить читалку" } }
                </button>
                <span class="save-status">{&*save_status}</span>
            </div>
        </main>
    }
}
