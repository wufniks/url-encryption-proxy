use std::str::FromStr;

use axum::{extract::Path, routing::any, Router};
use http::{uri::PathAndQuery, Request, StatusCode, Uri};
use hyper::{client::HttpConnector, Body};
use tower::ServiceBuilder;
use tower_http::{gateway::Gateway, trace::TraceLayer};
use tower_service::Service;

use crate::{Error, ShieldLayer};

pub type Client = hyper::client::Client<HttpConnector, Body>;

pub async fn build_proxy(client: Client) -> Result<Router, Error> {
    let mut gateway = Gateway::new(client, Uri::from_static("http://127.0.0.1:3000"))?;
    let handler = |Path(path): Path<String>, req: Request<Body>| async move {
        tracing::info!(?path, "handler");
        gateway.call(req).await.map_err(|_| StatusCode::BAD_GATEWAY)
    };
    let dispatcher = |path_and_query: &PathAndQuery| {
        http::uri::PathAndQuery::from_str(&format!("{}postfix", path_and_query.path())).ok()
    };
    let router = Router::new().route("/*path", any(handler)).layer(
        ServiceBuilder::new()
            .layer(TraceLayer::new_for_http())
            .layer(ShieldLayer::new(dispatcher)),
    );
    Ok(router)
}
