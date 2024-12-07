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
    set_error: WriteSignal<Option<StreamError>>,
) where
    T: DeserializeOwned,
{
    let response = request
        .send()
        .await
        .map_err(|e| StreamError::RequestFailed(e.to_string()))?;

    let body = response.body().ok_or(StreamError::EmptyResponse)?;

    let reader = body
        .get_reader()
        .unchecked_into::<web_sys::ReadableStreamDefaultReader>();

    loop {
        let chunk = JsFuture::from(reader.read())
            .await
            .map_err(|e| StreamError::ReadError(format_js_error(&e)))?;

        let chunk: js_sys::Object = chunk.unchecked_into();

        let done = match js_sys::Reflect::get(&chunk, &"done".into()) {
            Ok(done) => done.as_bool().ok_or_else(|| {
                StreamError::ParseError("Failed to convert 'done' to boolean".into())
            })?,
            Err(e) => return Err(StreamError::ParseError(format_js_error(&e))),
        };

        if done {
            break;
        }

        let value = js_sys::Reflect::get(&chunk, &"value".into())
            .map_err(|e| StreamError::ParseError(format_js_error(&e)))?;

        let value: Uint8Array = value.unchecked_into();
        let value_vec = value.to_vec();

        let text =
            String::from_utf8(value_vec).map_err(|e| StreamError::ParseError(e.to_string()))?;

        let item: T =
            serde_json::from_str(&text).map_err(|e| StreamError::ParseError(e.to_string()))?;

        set_items.update(|items| items.push(item));
    }
}

#[component]
pub fn StreamExample() -> impl IntoView {
    let (items, set_items) = create_signal(Vec::new());
    let stream_error = create_rw_signal(None::<StreamError>);

    spawn_local(async move {
        // 由调用方构造请求
        let request =
            Request::get("http://localhost:3000/stream").header("Content-Type", "application/json");

        fetch_stream::<serde_json::Value>(
            request.build().unwrap(),
            set_items,
            stream_error.write_only(),
        )
        .await;
    });

    view! {
        <h1>"Stream Render Example"</h1>
        {move || stream_error.get().map(|err| view! {
            <div class="error">
                "Error: " {move || err.to_string()}
            </div>
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
