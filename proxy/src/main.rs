use std::net::SocketAddr;

use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use proxy::{build_proxy, mock_server, Client};

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::new(
            std::env::var("RUST_LOG").unwrap_or_default(),
        ))
        .with(tracing_subscriber::fmt::layer())
        .init();

    tokio::spawn(mock_server());

    let client = Client::new();
    let proxy = build_proxy(client).await?;

    let addr = SocketAddr::from(([127, 0, 0, 1], 4000));
    println!("reverse proxy listening on {}", addr);
    axum::Server::bind(&addr)
        .serve(proxy.into_make_service())
        .await
        .unwrap();
    Ok(())
}
