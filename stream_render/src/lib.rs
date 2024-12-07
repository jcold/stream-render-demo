use gloo::net::http::Request;
use js_sys::Uint8Array;
use leptos::*;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::spawn_local;
use wasm_bindgen_futures::JsFuture;

#[component]
pub fn StreamExample() -> impl IntoView {
    // 创建一个响应式信号，用于存储流数据
    let (items, set_items) = create_signal( Vec::new());

    // 启动异步任务监听流
    spawn_local(async move {
        let response = Request::get("http://localhost:3000/stream")
            .send()
            .await
            .unwrap();

        if let Some(body) = response.body() {
            // 获取流的 reader
            let reader = body
                .get_reader()
                .unchecked_into::<web_sys::ReadableStreamDefaultReader>();

            loop {
                // 从流中读取数据块
                match JsFuture::from(reader.read()).await {
                    Ok(chunk) => {
                        let chunk: js_sys::Object = chunk.unchecked_into();
                        let done = js_sys::Reflect::get(&chunk, &"done".into())
                            .unwrap()
                            .as_bool()
                            .unwrap();

                        if done {
                            break;
                        }

                        let value = js_sys::Reflect::get(&chunk, &"value".into())
                            .unwrap();
                        let value: Uint8Array = value.unchecked_into();
                        let value_vec = value.to_vec();
                        let text = String::from_utf8_lossy(&value_vec);

                        // 更新信号中的数据
                        set_items.update(|items| items.push(text.to_string()));
                    }
                    Err(_) => break, // 处理流读取错误
                }
            }
        }
    });

    // 动态渲染信号数据
    view! {
        <h1>"Stream Render Example"</h1>
        <ul>
            {move || items.get().iter().map(|item| view! {
                <li>{item}</li>
            }).collect_view()}
        </ul>
    }
}
