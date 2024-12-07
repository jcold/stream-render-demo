use axum::body::Body;
use axum::response::Response;
use axum::{routing::get, Router};
use chrono;
use futures::stream::{self};
use tokio::time::Duration;
use tokio_stream::StreamExt;
use axum::http::Method;
use tower_http::cors::{Any, CorsLayer};

#[tokio::main]
async fn main() {
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods([Method::GET])
        .allow_headers(Any);

    let app = Router::new()
        .route("/stream", get(stream_handler))
        .layer(cors);

    println!("服务器启动在 http://localhost:3000");
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

async fn stream_handler() -> Response<Body> {
    let stream = stream::iter(0..10).then(|i| async move {
        tokio::time::sleep(Duration::from_secs(3)).await;
        let payload = serde_json::json!({
            "index": i,
            "timestamp": chrono::Utc::now().timestamp(),
            "message": format!("这是第 {} 条消息", i + 1)
        });
        Ok::<_, std::convert::Infallible>(payload.to_string())
    });

    let body = Body::from_stream(stream);
    Response::builder()
        .header("content-type", "application/json")
        .body(body)
        .unwrap()
}
