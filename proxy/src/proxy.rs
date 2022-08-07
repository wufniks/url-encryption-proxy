use std::fmt::Write;
use std::{collections::HashMap, sync::Arc};

use axum::middleware::{from_fn, Next};
use axum::{error_handling::HandleErrorLayer, extract::Path, routing::any, BoxError, Router};
use bytes::{BufMut, BytesMut};
use http::{Request, StatusCode, Uri};
use hyper::{client::HttpConnector, Body};
use tokio::sync::Mutex;
use tower::{ServiceBuilder, ServiceExt};
use tower_http::{gateway::Gateway, trace::TraceLayer};
use tower_service::Service;

use crate::{Encrypt, Error};

pub type Client = hyper::client::Client<HttpConnector, Body>;

pub async fn build_proxy(client: Client) -> Result<Router, Error> {
    let mut gateway = Gateway::new(client, Uri::from_static("http://127.0.0.1:3000"))?;
    let handler = |Path(path): Path<String>, req: Request<Body>| async move {
        tracing::info!(?path, uri=?req.uri(), "handler");
        gateway.call(req).await.map_err(|_| StatusCode::BAD_GATEWAY)
    };
    let cache = Arc::new(Mutex::new(HashMap::new()));

    let encrypt_handler = {
        let cache = Arc::clone(&cache);
        move |original: String| {
            let cache = Arc::clone(&cache);
            async move {
                let encrypted: String = original.strip_prefix('/').unwrap().chars().rev().collect();
                let encrypted = format!("/{}", encrypted);
                {
                    let mut guard = cache.lock().await;
                    guard.insert(encrypted.clone(), original.clone());
                    tracing::info!("added {} -> {}", encrypted, original);
                }
                Result::<String, BoxError>::Ok(encrypted)
            }
        }
    };
    let mut encrypt = tower::service_fn(encrypt_handler);

    let modify_html = |body| async move {
        let mut buf = BytesMut::new();
        buf.put(body);
        let encrypted_path = encrypt
            .ready()
            .await
            .unwrap()
            .call("/original-path".to_owned())
            .await
            .unwrap();

        write!(buf, "http://localhost{encrypted_path}").unwrap();
        buf.freeze()
    };

    let url_encrypt = Encrypt::new(Arc::clone(&cache));
    let insert_url = move |request: Request<Body>, next: Next<Body>| {
        let modify_html = modify_html.clone();
        async move {
            let res = next.run(request).await;
            let (_parts, body) = res.into_parts();
            let original = hyper::body::to_bytes(body).await.unwrap();
            modify_html(original).await
        }
    };

    let router = Router::new().route("/*path", any(handler)).layer(
        ServiceBuilder::new()
            .layer(TraceLayer::new_for_http())
            .layer(
                ServiceBuilder::new()
                    .layer(HandleErrorLayer::new(handle_boxed_error))
                    // NOTE: 엄밀히 filter가 주 목적이 아니지만, request를 async하게 변조해 줄 수 있는
                    // 가장 간단한 방법이라 생각해서 이렇게 처리함.
                    .filter_async(url_encrypt),
            )
            .layer(from_fn(insert_url)),
    );
    Ok(router)
}

async fn handle_boxed_error(err: BoxError) -> (StatusCode, String) {
    // TODO: better error handling
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        format!("Unhandled internal error: {}", err),
    )
}
