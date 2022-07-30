use std::fmt;
use std::task::{Context, Poll};

use axum::{body::Body, http::Request};
use http::uri::PathAndQuery;
use http::Uri;
use tower::{Layer, Service};

pub trait Dispatch {
    fn pick(&self, path_and_query: &PathAndQuery) -> Option<PathAndQuery>;
}

impl<F> Dispatch for F
where
    F: Fn(&PathAndQuery) -> Option<PathAndQuery>,
{
    fn pick(&self, path_and_query: &PathAndQuery) -> Option<PathAndQuery> {
        self(path_and_query)
    }
}

pub struct ShieldLayer<D> {
    dispatcher: D,
}

impl<S, D> Layer<S> for ShieldLayer<D>
where
    D: Dispatch + Clone,
{
    type Service = Shield<S, D>;

    fn layer(&self, inner: S) -> Self::Service {
        Shield {
            inner,
            dispatcher: self.dispatcher.clone(),
        }
    }
}

impl<D> ShieldLayer<D> {
    pub fn new(dispatcher: D) -> Self {
        Self { dispatcher }
    }
}

pub struct Shield<S, D> {
    inner: S,
    dispatcher: D,
}

impl<S, D> Shield<S, D> {
    pub fn new(inner: S, dispatcher: D) -> Self {
        Self { inner, dispatcher }
    }
}

impl<S, D> Service<Request<Body>> for Shield<S, D>
where
    S: Service<Request<Body>>,
    D: Dispatch,
{
    type Response = S::Response;
    type Error = S::Error;
    type Future = S::Future;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, mut req: Request<Body>) -> Self::Future {
        let mut parts = req.uri().clone().into_parts();
        let protected_path = parts
            .path_and_query
            .and_then(|path| self.dispatcher.pick(&path));

        parts.path_and_query = protected_path;
        if let Some(protected_uri) = Uri::from_parts(parts).ok() {
            *req.uri_mut() = protected_uri;
        }
        self.inner.call(req)
    }
}

impl<S, D> Clone for Shield<S, D>
where
    S: Clone,
    D: Clone,
{
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
            dispatcher: self.dispatcher.clone(),
        }
    }
}

impl<S, D> fmt::Debug for Shield<S, D>
where
    S: fmt::Debug,
    D: fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let Self { inner, dispatcher } = self;
        f.debug_struct("Steer")
            .field("inner", inner)
            .field("dispatcher", dispatcher)
            .finish()
    }
}
