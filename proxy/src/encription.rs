use std::collections::HashMap;
use std::sync::Arc;

use axum::BoxError;
use axum::{body::Body, http::Request};
use futures::future::BoxFuture;
use http::Uri;
use tokio::sync::Mutex;
use tower::filter::AsyncPredicate;

#[derive(Clone)]
pub struct Encript {
    cache: Arc<Mutex<HashMap<String, String>>>,
}

impl Encript {
    pub fn new(cache: Arc<Mutex<HashMap<String, String>>>) -> Self {
        Self {
            cache,
            // cache: HashMap::new(),
        }
    }
}

impl AsyncPredicate<Request<Body>> for Encript {
    type Future = BoxFuture<'static, Result<Self::Request, BoxError>>;
    type Request = Request<Body>;
    fn check(&mut self, mut req: Request<Body>) -> Self::Future {
        let mut parts = req.uri().clone().into_parts();
        let cache = Arc::clone(&self.cache);

        if let Some(path) = parts
            .path_and_query
            .as_ref()
            .map(|path_and_query| path_and_query.path().to_owned())
        {
            tracing::debug!("found the matching path");
            Box::pin(async move {
                let protected_path = cache.lock().await.get(&path).and_then(|s| s.parse().ok());
                parts.path_and_query = protected_path;
                if let Ok(protected_uri) = Uri::from_parts(parts) {
                    *req.uri_mut() = protected_uri;
                }
                Ok(req)
            })
        } else {
            tracing::debug!("couldn't find the matching path");
            Box::pin(futures::future::ready(Ok(req)))
        }
    }
}
