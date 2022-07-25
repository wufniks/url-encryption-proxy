use axum::{error_handling::HandleError, Router};
use http::{StatusCode, Uri};
use hyper::{client::HttpConnector, Body};
use tower_http::gateway::Gateway;

use crate::Error;

pub type Client = hyper::client::Client<HttpConnector, Body>;

pub async fn build_proxy(client: Client) -> Result<Router, Error> {
    let gateway = Gateway::new(client, Uri::from_static("http://127.0.0.1:4000"))?;
    Ok(Router::new().nest(
        "/",
        HandleError::new(gateway, |_| async { StatusCode::BAD_GATEWAY }),
    ))
}
