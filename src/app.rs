use serde::{Deserialize, Serialize};
use wasm_bindgen::{prelude::*, JsCast};
use wasm_bindgen_futures::spawn_local;
use web_sys::{Headers, HtmlInputElement, Request, RequestInit, Response};
use yew::prelude::*;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = ["window", "__TAURI__", "core"])]
    async fn invoke(cmd: &str, args: JsValue) -> JsValue;

    #[wasm_bindgen(js_namespace = ["window", "__TAURI__", "event"])]
    async fn listen(event: &str, handler: &Closure<dyn FnMut(JsValue)>) -> JsValue;
}

// Payload-структуры
#[derive(Serialize)]
struct SetTextArgs {
    new_text: String,
}
#[derive(Serialize)]
struct FetchUrlPayload {
    url: String,
}
#[derive(Serialize)]
struct SetSendOnCopyArgs {
    enabled: bool,
}
#[derive(Serialize)]
struct SetAddToEditorArgs {
    enabled: bool,
}

#[derive(Deserialize)]
struct ApiResponse {
    message: String,
}

#[derive(Deserialize, Debug)]
struct TauriEvent<T> {
    payload: T,
}

#[function_component(App)]
pub fn app() -> Html {
    // --- Состояние редактора ---
    let editor_content = use_state(String::new);
    let editor_ref = use_mut_ref(|| String::new()); // всегда актуальное значение

    // синхронизация editor_ref при каждом изменении состояния
    {
        let editor_content = editor_content.clone();
        let editor_ref = editor_ref.clone();
        use_effect_with(
            (*editor_content).clone(),
            move |val| {
                *editor_ref.borrow_mut() = val.clone();
                || {}
            },
        );
    }

    // --- остальные состояния ---
    let save_status = use_state(String::new);
    let is_saving = use_state(|| false);
    let url_input = use_state(String::new);
    let fetch_status = use_state(String::new);
    let is_fetching = use_state(|| false);
    let server_info = use_state(|| "Загрузка информации о сервере...".to_string());
    let send_on_copy = use_state(|| false);
    let add_to_editor_on_copy = use_state(|| false);

    // --- загрузка данных при старте ---
    {
        let editor_content = editor_content.clone();
        let editor_ref = editor_ref.clone();
        let server_info = server_info.clone();
        use_effect_with((), move |_| {
            spawn_local(async move {
                let text = invoke("get_text", JsValue::NULL).await.as_string().unwrap_or_default();
                *editor_ref.borrow_mut() = text.clone();
                editor_content.set(text);

                let info = invoke("get_server_info", JsValue::NULL)
                    .await
                    .as_string()
                    .unwrap_or_else(|| "Ошибка получения информации о сервере".to_string());
                server_info.set(info);
            });
            || {}
        });
    }

    // --- слушатель событий clipboard-add-to-editor ---
    {
        let editor_ref = editor_ref.clone();
        let editor_content = editor_content.clone();

        use_effect_with((), move |_| {
            spawn_local(async move {
                let callback = Closure::wrap(Box::new(move |event: JsValue| {
                    if let Ok(evt) = serde_wasm_bindgen::from_value::<TauriEvent<String>>(event) {
                        let text_to_append = evt.payload;
                        let current = editor_ref.borrow().clone();

                        web_sys::console::log_1(
                            &format!("Clipboard event: current='{}', append='{}'",
                                     current, text_to_append).into(),
                        );

                        let new_content = if current.trim().is_empty() {
                            text_to_append
                        } else {
                            format!("{}\n\n{}", current, text_to_append)
                        };

                        *editor_ref.borrow_mut() = new_content.clone();
                        editor_content.set(new_content.clone());

                        spawn_local(async move {
                            let args = SetTextArgs { new_text: new_content };
                            let js_payload =
                                serde_wasm_bindgen::to_value(&args).expect("serde convert");
                            invoke("set_text", js_payload).await;
                        });
                    }
                }) as Box<dyn FnMut(JsValue)>);

                listen("clipboard-add-to-editor", &callback).await;
                callback.forget();
            });
            || {}
        });
    }

    // --- обработка ручного ввода ---
    let on_input = {
        let editor_content = editor_content.clone();
        let editor_ref = editor_ref.clone();
        let save_status = save_status.clone();
        Callback::from(move |e: InputEvent| {
            let value = e.target_unchecked_into::<web_sys::HtmlTextAreaElement>().value();
            editor_content.set(value.clone());
            *editor_ref.borrow_mut() = value;
            save_status.set("".to_string());
        })
    };

    // --- сохранение текста ---
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
                let payload = SetTextArgs { new_text: content_to_save };
                let js_payload = serde_wasm_bindgen::to_value(&payload).unwrap();
                let headers = Headers::new().unwrap();
                headers.set("Content-Type", "application/json").unwrap();
                let mut opts = RequestInit::new();
                opts.set_method("POST");
                let body_str = js_sys::JSON::stringify(&js_payload).unwrap();
                opts.set_body(&body_str);
                opts.set_headers(&headers);
                let request =
                    Request::new_with_str_and_init("http://localhost:5001/api/content", &opts)
                        .unwrap();
                let window = web_sys::window().unwrap();
                let resp_value =
                    wasm_bindgen_futures::JsFuture::from(window.fetch_with_request(&request)).await;

                match resp_value {
                    Ok(resp) => {
                        let resp: Response = resp.dyn_into().unwrap();
                        if resp.ok() {
                            save_status_clone.set("Сохранено!".to_string());
                        } else {
                            let error_msg = format!(
                                "Ошибка сохранения: {} {}",
                                resp.status(),
                                resp.status_text()
                            );
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

    // --- ввод URL ---
    let on_url_input = {
        let url_input = url_input.clone();
        let fetch_status = fetch_status.clone();
        Callback::from(move |e: InputEvent| {
            let value = e.target_unchecked_into::<HtmlInputElement>().value();
            url_input.set(value);
            fetch_status.set("".to_string());
        })
    };

    // --- загрузка URL ---
    let on_fetch_url = {
        let url_input = url_input.clone();
        let fetch_status = fetch_status.clone();
        let is_fetching = is_fetching.clone();
        let editor_content = editor_content.clone();
        let editor_ref = editor_ref.clone();

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
            let editor_ref_clone = editor_ref.clone();

            spawn_local(async move {
                let payload = FetchUrlPayload { url: url_to_fetch };
                let js_payload = serde_wasm_bindgen::to_value(&payload).unwrap();
                let headers = Headers::new().unwrap();
                headers.set("Content-Type", "application/json").unwrap();
                let mut opts = RequestInit::new();
                opts.set_method("POST");
                let body_str = js_sys::JSON::stringify(&js_payload).unwrap();
                opts.set_body(&body_str);
                opts.set_headers(&headers);
                let request =
                    Request::new_with_str_and_init("http://localhost:5001/api/url", &opts).unwrap();
                let window = web_sys::window().unwrap();
                let resp_value =
                    wasm_bindgen_futures::JsFuture::from(window.fetch_with_request(&request)).await;

                match resp_value {
                    Ok(resp_val) => {
                        let resp: Response = resp_val.dyn_into().unwrap();
                        if let Ok(json_promise) = resp.json() {
                            if let Ok(json_val) =
                                wasm_bindgen_futures::JsFuture::from(json_promise).await
                            {
                                if let Ok(data) =
                                    serde_wasm_bindgen::from_value::<ApiResponse>(json_val)
                                {
                                    if resp.ok() {
                                        fetch_status_clone.set("Отправлено!".to_string());
                                        let text = invoke("get_text", JsValue::NULL)
                                            .await
                                            .as_string()
                                            .unwrap_or_default();
                                        *editor_ref_clone.borrow_mut() = text.clone();
                                        editor_content_clone.set(text);
                                    } else {
                                        fetch_status_clone
                                            .set(format!("Ошибка: {}", data.message));
                                    }
                                } else {
                                    fetch_status_clone
                                        .set("Ошибка: неверный формат ответа.".to_string());
                                }
                            } else {
                                fetch_status_clone
                                    .set("Ошибка: не удалось прочитать ответ.".to_string());
                            }
                        } else {
                            fetch_status_clone
                                .set("Ошибка: ответ сервера - не JSON.".to_string());
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

    // --- чекбоксы ---
    let on_send_toggle = {
        let send_on_copy = send_on_copy.clone();
        let add_to_editor_on_copy = add_to_editor_on_copy.clone();
        Callback::from(move |_e: Event| {
            let new_value = !*send_on_copy;
            send_on_copy.set(new_value);

            if new_value && *add_to_editor_on_copy {
                add_to_editor_on_copy.set(false);
                spawn_local(async {
                    let args = SetAddToEditorArgs { enabled: false };
                    invoke(
                        "set_add_to_editor_on_copy",
                        serde_wasm_bindgen::to_value(&args).unwrap(),
                    )
                    .await;
                });
            }

            spawn_local(async move {
                let args = SetSendOnCopyArgs { enabled: new_value };
                invoke("set_send_on_copy", serde_wasm_bindgen::to_value(&args).unwrap()).await;
            });
        })
    };

    let on_add_toggle = {
        let add_to_editor_on_copy = add_to_editor_on_copy.clone();
        let send_on_copy = send_on_copy.clone();
        Callback::from(move |_e: Event| {
            let new_value = !*add_to_editor_on_copy;
            add_to_editor_on_copy.set(new_value);

            if new_value && *send_on_copy {
                send_on_copy.set(false);
                spawn_local(async move {
                    let args = SetSendOnCopyArgs { enabled: false };
                    invoke("set_send_on_copy", serde_wasm_bindgen::to_value(&args).unwrap())
                        .await;
                });
            }

            spawn_local(async move {
                let args = SetAddToEditorArgs { enabled: new_value };
                invoke(
                    "set_add_to_editor_on_copy",
                    serde_wasm_bindgen::to_value(&args).unwrap(),
                )
                .await;
            });
        })
    };

    // --- рендер ---
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

                <div class="toggle-controls">
                     <div class="auto-send-toggle">
                        <input
                            type="checkbox"
                            id="sendOnCopy"
                            checked={*send_on_copy}
                            onchange={on_send_toggle}
                        />
                        <label for="sendOnCopy">{"Отправлять текст при копировании"}</label>
                    </div>
                    <div class="auto-send-toggle">
                        <input
                            type="checkbox"
                            id="addOnCopy"
                            checked={*add_to_editor_on_copy}
                            onchange={on_add_toggle}
                        />
                        <label for="addOnCopy">{"Добавлять в редактор при копировании"}</label>
                    </div>
                </div>
            </div>
        </main>
    }
}
