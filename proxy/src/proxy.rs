use std::fmt::Write;
use std::{collections::HashMap, sync::Arc};

use axum::{
    error_handling::HandleErrorLayer, extract::Path, response::Response, routing::any, BoxError,
    Router,
};
use bytes::{BufMut, BytesMut};
use http::{Request, StatusCode, Uri};
use hyper::{client::HttpConnector, Body};
use tokio::sync::Mutex;
use tower::{ServiceBuilder, ServiceExt};
use tower_http::{gateway::Gateway, trace::TraceLayer};
use tower_service::Service;

use crate::{Encript, Error};

pub type Client = hyper::client::Client<HttpConnector, Body>;

pub async fn build_proxy(client: Client) -> Result<Router, Error> {
    let mut gateway = Gateway::new(client, Uri::from_static("http://127.0.0.1:3000"))?.then(
        |result: Result<Response<Body>, hyper::Error>| async move {
            match result {
                Ok(mut res) => {
                    let body = res.body_mut();
                    let mut buf = BytesMut::new();
                    let original = hyper::body::to_bytes(body).await.unwrap();
                    buf.put(original);
                    write!(buf, "http://localhost/inserted-path")?;
                    Ok(buf.freeze())
                }
                Err(e) => Err(Error::from(e)),
            }
        },
    );
    let handler = |Path(path): Path<String>, req: Request<Body>| async move {
        tracing::info!(?path, "handler");
        gateway.call(req).await.map_err(|_| StatusCode::BAD_GATEWAY)
    };
    let cache = Arc::new(Mutex::new(HashMap::new()));
    let url_encript = Encript::new(cache);
    let router = Router::new().route("/*path", any(handler)).layer(
        ServiceBuilder::new()
            .layer(TraceLayer::new_for_http())
            .layer(HandleErrorLayer::new(handle_timeout_error))
            // NOTE: 엄밀히 filter가 주 목적이 아니지만, request를 async하게 변조해 줄 수 있는
            // 가장 간단한 방법이라 생각해서 이렇게 처리함.
            .filter_async(url_encript),
    );
    Ok(router)
}

async fn handle_timeout_error(err: BoxError) -> (StatusCode, String) {
    if err.is::<tower::timeout::error::Elapsed>() {
        (
            StatusCode::REQUEST_TIMEOUT,
            "Request took too long".to_string(),
        )
    } else {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Unhandled internal error: {}", err),
        )
    }
}
