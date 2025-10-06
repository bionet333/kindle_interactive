use serde::{Deserialize, Serialize};
use wasm_bindgen::{prelude::*, JsCast};
use wasm_bindgen_futures::spawn_local;
use web_sys::{Headers, HtmlInputElement, Request, RequestInit, Response};
use yew::prelude::*;

// The Tauri `invoke` is still used for reading initial state.
#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = ["window", "__TAURI__", "core"])]
    async fn invoke(cmd: &str, args: JsValue) -> JsValue;
}

// Payload for the HTTP POST request to save text.
#[derive(Serialize)]
struct SetTextArgs {
    new_text: String,
}

// Payload for the HTTP POST request to fetch a URL.
#[derive(Serialize)]
struct FetchUrlPayload {
    url: String,
}

// Payload for the command to toggle clipboard monitoring.
#[derive(Serialize)]
struct SetClipboardMonitoringArgs {
    enabled: bool,
}

// Generic response from API calls.
#[derive(Deserialize)]
struct ApiResponse {
    message: String,
}

/// The root component of the Yew frontend application.
#[function_component(App)]
pub fn app() -> Html {
    // State for the text editor
    let editor_content = use_state(String::new);
    let save_status = use_state(String::new);
    let is_saving = use_state(|| false);

    // State for the URL loader
    let url_input = use_state(String::new);
    let fetch_status = use_state(String::new);
    let is_fetching = use_state(|| false);

    // State for server info
    let server_info = use_state(|| "Загрузка информации о сервере...".to_string());

    // State for clipboard monitoring checkbox
    let auto_send_on_copy = use_state(|| false);

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

    // Callback for the save button click.
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
                let payload_js_value = serde_wasm_bindgen::to_value(&payload).unwrap();

                let headers = Headers::new().unwrap();
                headers.set("Content-Type", "application/json").unwrap();

                let mut opts = RequestInit::new();
                opts.set_method("POST");

                let body_js_value: JsValue = js_sys::JSON::stringify(&payload_js_value).unwrap().into();
                opts.set_body(Some(&body_js_value).as_ref().unwrap());

                opts.set_headers(&headers);

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

    // Callback for URL input field.
    let on_url_input = {
        let url_input = url_input.clone();
        let fetch_status = fetch_status.clone();
        Callback::from(move |e: InputEvent| {
            let value = e.target_unchecked_into::<HtmlInputElement>().value();
            url_input.set(value);
            fetch_status.set("".to_string()); // Clear status on edit
        })
    };

    // Callback to fetch URL and send to reader.
    let on_fetch_url = {
        let url_input = url_input.clone();
        let fetch_status = fetch_status.clone();
        let is_fetching = is_fetching.clone();
        let editor_content = editor_content.clone(); // To clear the editor on success

        Callback::from(move |e: MouseEvent| {
            e.prevent_default();
            if *is_fetching || (*url_input).trim().is_empty() {
                return;
            }

            is_fetching.set(true);
            fetch_status.set("Открываю страницу...".to_string());

            let url_to_fetch = (*url_input).clone();
            let fetch_status_clone = fetch_status.clone();
            let is_fetching_clone = is_fetching.clone();
            let editor_content_clone = editor_content.clone();

            spawn_local(async move {
                let payload = FetchUrlPayload { url: url_to_fetch };
                let payload_js_value = serde_wasm_bindgen::to_value(&payload).unwrap();

                let headers = Headers::new().unwrap();
                headers.set("Content-Type", "application/json").unwrap();

                let mut opts = RequestInit::new();
                opts.set_method("POST");

                let body_js_value: JsValue = js_sys::JSON::stringify(&payload_js_value).unwrap().into();
                opts.set_body(Some(&body_js_value).as_ref().unwrap());
                
                opts.set_headers(&headers);

                let url = "http://localhost:5001/api/url";
                let request = Request::new_with_str_and_init(url, &opts).unwrap();

                let window = web_sys::window().unwrap();
                let resp_value = wasm_bindgen_futures::JsFuture::from(window.fetch_with_request(&request)).await;

                match resp_value {
                    Ok(resp_val) => {
                        let resp: Response = resp_val.dyn_into().unwrap();
                        match resp.json() {
                            Ok(json_promise) => {
                                match wasm_bindgen_futures::JsFuture::from(json_promise).await {
                                    Ok(json_val) => {
                                        match serde_wasm_bindgen::from_value::<ApiResponse>(json_val) {
                                            Ok(data) => {
                                                if resp.ok() {
                                                    fetch_status_clone.set("Отправлено!".to_string());
                                                    // Also update editor content with the new text from server
                                                    let text = invoke("get_text", JsValue::NULL).await.as_string().unwrap_or_default();
                                                    editor_content_clone.set(text);
                                                } else {
                                                    fetch_status_clone.set(format!("Ошибка: {}", data.message));
                                                }
                                            },
                                            Err(_) => fetch_status_clone.set("Ошибка: неверный формат ответа.".to_string())
                                        }
                                    },
                                    Err(_) => fetch_status_clone.set("Ошибка: не удалось прочитать ответ.".to_string())
                                }
                            },
                            Err(_) => fetch_status_clone.set("Ошибка: ответ сервера - не JSON.".to_string())
                        }
                    }
                    Err(_) => {
                        fetch_status_clone.set("Ошибка сети. Сервер доступен?".to_string());
                    }
                }
                is_fetching_clone.set(false);
            });
        })
    };

    // Callback for the clipboard monitoring checkbox.
    let on_auto_send_toggle = {
        let auto_send_on_copy = auto_send_on_copy.clone();
        Callback::from(move |_e: Event| {
            let new_value = !*auto_send_on_copy;
            auto_send_on_copy.set(new_value);

            spawn_local(async move {
                let args = SetClipboardMonitoringArgs { enabled: new_value };
                let args_js = serde_wasm_bindgen::to_value(&args).unwrap();
                invoke("set_clipboard_monitoring", args_js).await;
            });
        })
    };

    html! {
        <main class="container">
            <div class="server-info">
                <p>{ &*server_info }</p>
            </div>

            <div class="url-loader">
                <input
                    type="url"
                    class="url-input"
                    placeholder="Вставьте URL статьи для отправки на читалку"
                    value={(*url_input).clone()}
                    oninput={on_url_input}
                    disabled={*is_fetching}
                />
                <button onclick={on_fetch_url} disabled={*is_fetching}>
                    { if *is_fetching { "Загрузка..." } else { "Отправить" } }
                </button>
                <span class="fetch-status">{&*fetch_status}</span>
            </div>

            <div class="editor-wrapper">
                <textarea
                    class="editor-textarea"
                    value={(*editor_content).clone()}
                    oninput={on_input}
                    placeholder="Или введите ваш Markdown-текст здесь..."
                />
            </div>

            <div class="controls">
                <button onclick={on_save} disabled={*is_saving}>
                    { if *is_saving { "Сохранение..." } else { "Сохранить и обновить читалку" } }
                </button>
                <span class="save-status">{&*save_status}</span>

                <div class="auto-send-toggle">
                    <input
                        type="checkbox"
                        id="autoSend"
                        checked={*auto_send_on_copy}
                        onchange={on_auto_send_toggle}
                    />
                    <label for="autoSend">{"Отправлять при копировании"}</label>
                </div>
            </div>
        </main>
    }
}
