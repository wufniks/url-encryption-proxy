use std::net::SocketAddr;

use axum::{body::Body, extract::Path, response::Response, routing::get, Extension, Router};
use hyper::{client::HttpConnector, Request, Uri};

mod handler;

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

pub async fn build_app(client: Client) -> Router {
    Router::new()
        .route("/*path", get(handler::proxy_handler))
        .layer(Extension(client))
}

async fn handler(
    Extension(client): Extension<Client>,
    // NOTE: Make sure to put the request extractor last because once the request
    // is extracted, extensions can't be extracted anymore.
    mut req: Request<Body>,
) -> Response<Body> {
    let path = req.uri().path();
    let path_query = req
        .uri()
        .path_and_query()
        .map(|v| v.as_str())
        .unwrap_or(path);

    let uri = format!("http://127.0.0.1:3000{}", path_query);

    *req.uri_mut() = Uri::try_from(uri).unwrap();

    client.request(req).await.unwrap()
}
