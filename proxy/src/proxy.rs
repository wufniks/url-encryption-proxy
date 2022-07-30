use std::str::FromStr;

use axum::middleware::{from_fn, Next};
use axum::response::IntoResponse;
use axum::{error_handling::HandleError, Router};
use http::{Request, StatusCode, Uri};
use hyper::{client::HttpConnector, Body};
use tower_http::gateway::Gateway;

use crate::Error;

pub type Client = hyper::client::Client<HttpConnector, Body>;

pub async fn build_proxy(client: Client) -> Result<Router, Error> {
    let gateway = Gateway::new(client, Uri::from_static("http://127.0.0.1:3000"))?;
    let router = Router::new()
        .route(
            "/",
            HandleError::new(gateway, |_| async { StatusCode::BAD_GATEWAY }),
        )
        .layer(from_fn(shield));
    Ok(router)
}

async fn shield(mut req: Request<Body>, next: Next<Body>) -> Result<impl IntoResponse, StatusCode> {
    let mut parts = req.uri().clone().into_parts();
    let protected_path = parts.path_and_query.and_then(|path_and_query| {
        http::uri::PathAndQuery::from_str(&format!("{}postfix", path_and_query.path())).ok()
    });
    parts.path_and_query = protected_path;
    *req.uri_mut() = Uri::from_parts(parts).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(next.run(req).await)
}
