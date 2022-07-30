mod error;
mod proxy;
mod shield;

use std::net::SocketAddr;

use axum::{body::Body, extract::Path, routing::get, Router};
use hyper::{client::HttpConnector, Request};

pub use self::{
    error::Error,
    proxy::build_proxy,
    shield::{Shield, ShieldLayer},
};

pub type Client = hyper::client::Client<HttpConnector, Body>;

pub async fn mock_server() {
    let app = Router::new().route(
        "/*path",
        get(|Path(path): Path<String>| async move {
            tracing::info!(method = "GET", %path, "get service");
            "Hello, world!"
        })
        .post(|Path(path): Path<String>, req: Request<Body>| async move {
            tracing::info!(method = "POST", %path, "post service");
            let (_, body) = req.into_parts();
            format!("Message received!: {:?}", hyper::body::to_bytes(body).await)
        }),
    );

    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
    tracing::info!("mock server listening on {}", addr);
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
}
