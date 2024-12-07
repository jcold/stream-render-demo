use gloo::net::http::Request;
use js_sys::Uint8Array;
use leptos::*;
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

#[component]
pub fn StreamExample() -> impl IntoView {
    let (items, set_items) = create_signal(Vec::new());
    let stream_error = create_rw_signal(None::<StreamError>);

    spawn_local(async move {
        let response = match Request::get("http://localhost:3000/stream").send().await {
            Ok(response) => response,
            Err(e) => {
                stream_error.set(Some(StreamError::RequestFailed(e.to_string())));
                return;
            }
        };

        let body = match response.body() {
            Some(body) => body,
            None => {
                stream_error.set(Some(StreamError::EmptyResponse));
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
                    stream_error.set(Some(StreamError::ReadError(format_js_error(&e))));
                    break;
                }
            };

            let chunk: js_sys::Object = chunk.unchecked_into();

            let done = match js_sys::Reflect::get(&chunk, &"done".into()) {
                Ok(done) => match done.as_bool() {
                    Some(done) => done,
                    None => {
                        stream_error.set(Some(StreamError::ParseError(
                            "Failed to convert 'done' to boolean".into(),
                        )));
                        break;
                    }
                },
                Err(e) => {
                    stream_error.set(Some(StreamError::ParseError(format_js_error(&e))));
                    break;
                }
            };

            if done {
                break;
            }

            let value = match js_sys::Reflect::get(&chunk, &"value".into()) {
                Ok(value) => value,
                Err(e) => {
                    stream_error.set(Some(StreamError::ParseError(format_js_error(&e))));
                    continue;
                }
            };

            let value: Uint8Array = value.unchecked_into();
            let value_vec = value.to_vec();

            let text = match String::from_utf8(value_vec) {
                Ok(text) => text,
                Err(e) => {
                    stream_error.set(Some(StreamError::ParseError(e.to_string())));
                    continue;
                }
            };

            set_items.update(|items| items.push(text));
        }
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
                <li>{item}</li>
            }).collect_view()}
        </ul>
    }
}

fn format_js_error(err: &JsValue) -> String {
    err.as_string().unwrap_or_else(|| "Unknown JS error".into())
}
