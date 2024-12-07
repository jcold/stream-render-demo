use gloo::net::http::Request;
use js_sys::Uint8Array;
use leptos::*;
use serde::de::DeserializeOwned;
use thiserror::Error;
use wasm_bindgen::JsCast;
use wasm_bindgen::JsValue;
use wasm_bindgen_futures::spawn_local;
use wasm_bindgen_futures::JsFuture;

#[derive(Debug, Clone, Error)]
enum StreamError {
    #[error("Request failed: {0}")]
    RequestFailed(String),
    #[error("Empty response received")]
    EmptyResponse,
    #[error("Stream read error: {0}")]
    ReadError(String),
    #[error("Parse error: {0}")]
    ParseError(String),
}

async fn fetch_stream<T>(
    request: Request,
    set_items: WriteSignal<Vec<T>>,
    set_error: WriteSignal<Option<Result<(), StreamError>>>,
) where
    T: DeserializeOwned,
{
    let response = match request.send().await {
        Ok(response) => response,
        Err(e) => {
            set_error.set(Some(Err(StreamError::RequestFailed(e.to_string()))));
            return;
        }
    };

    let body = match response.body() {
        Some(body) => body,
        None => {
            set_error.set(Some(Err(StreamError::EmptyResponse)));
            return;
        }
    };

    let reader = body
        .get_reader()
        .unchecked_into::<web_sys::ReadableStreamDefaultReader>();

    loop {
        let chunk = match JsFuture::from(reader.read()).await {
            Ok(chunk) => chunk,
            Err(e) => {
                set_error.set(Some(Err(StreamError::ReadError(format_js_error(&e)))));
                return;
            }
        };

        let chunk: js_sys::Object = chunk.unchecked_into();

        let done = match js_sys::Reflect::get(&chunk, &"done".into()) {
            Ok(done) => match done.as_bool() {
                Some(done) => done,
                None => {
                    set_error.set(Some(Err(StreamError::ParseError(
                        "Failed to convert 'done' to boolean".into(),
                    ))));
                    return;
                }
            },
            Err(e) => {
                set_error.set(Some(Err(StreamError::ParseError(format_js_error(&e)))));
                return;
            }
        };

        if done {
            break;
        }

        let value = match js_sys::Reflect::get(&chunk, &"value".into()) {
            Ok(value) => value,
            Err(e) => {
                set_error.set(Some(Err(StreamError::ParseError(format_js_error(&e)))));
                return;
            }
        };

        let value: Uint8Array = value.unchecked_into();
        let value_vec = value.to_vec();

        let text = match String::from_utf8(value_vec) {
            Ok(text) => text,
            Err(e) => {
                set_error.set(Some(Err(StreamError::ParseError(e.to_string()))));
                return;
            }
        };

        let item: T = match serde_json::from_str(&text) {
            Ok(item) => item,
            Err(e) => {
                set_error.set(Some(Err(StreamError::ParseError(e.to_string()))));
                return;
            }
        };

        set_items.update(|items| items.push(item));
    }

    // 常结束时设置 Ok(())
    set_error.set(Some(Ok(())));
}

#[component]
pub fn StreamExample() -> impl IntoView {
    let (items, set_items) = create_signal(Vec::new());
    let stream_error = create_rw_signal(None::<Result<(), StreamError>>);

    spawn_local(async move {
        let request = Request::get("http://localhost:3000/stream")
            .header("Content-Type", "application/json")
            .build()
            .unwrap();

        fetch_stream::<serde_json::Value>(request, set_items, stream_error.write_only()).await;
    });

    view! {
        <h1>"Stream Render Example"</h1>
        {move || stream_error.get().map(|result| match result {
            Ok(_) => view! { <div class="success">"Stream completed successfully"</div> },
            Err(err) => view! {
                <div class="error">
                    "Error: " {err.to_string()}
                </div>
            }
        })}
        <ul>
            {move || items.get().iter().map(|item| view! {
                <li>{serde_json::to_string_pretty(item).unwrap_or_else(|_| "Invalid JSON".into())}</li>
            }).collect_view()}
        </ul>
    }
}

fn format_js_error(err: &JsValue) -> String {
    err.as_string().unwrap_or_else(|| "Unknown JS error".into())
}
