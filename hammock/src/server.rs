use std::convert::Infallible;
use std::io;
use std::sync::Arc;

use bytes::Bytes;
use http_body_util::{BodyExt, Full};
use hyper::body::Incoming;
use hyper::service::service_fn;
use hyper::{Method, StatusCode};
use hyper_util::rt::{TokioExecutor, TokioIo};
use hyper_util::server::conn::auto;
use tokio::net::TcpListener;

use crate::context::Context;
use crate::handler::{Handler, boxed};
use crate::response::Response;
use crate::router::{Match, Router};

/// An HTTP server with Express-style routing.
///
/// Route paths support `:name` params (`/users/:id`) and a trailing
/// `*name` wildcard (`/files/*path`).
#[derive(Default)]
pub struct Server {
    router: Router,
}

impl Server {
    pub fn new() -> Self {
        Self::default()
    }

    /// Registers a handler for `method` and `path`.
    pub fn route<H, M>(&mut self, method: Method, path: &str, handler: H) -> &mut Self
    where
        H: Handler<M>,
    {
        self.router.add(method, path, boxed(handler));
        self
    }

    pub fn get<H: Handler<M>, M>(&mut self, path: &str, handler: H) -> &mut Self {
        self.route(Method::GET, path, handler)
    }

    pub fn post<H: Handler<M>, M>(&mut self, path: &str, handler: H) -> &mut Self {
        self.route(Method::POST, path, handler)
    }

    pub fn put<H: Handler<M>, M>(&mut self, path: &str, handler: H) -> &mut Self {
        self.route(Method::PUT, path, handler)
    }

    pub fn patch<H: Handler<M>, M>(&mut self, path: &str, handler: H) -> &mut Self {
        self.route(Method::PATCH, path, handler)
    }

    pub fn delete<H: Handler<M>, M>(&mut self, path: &str, handler: H) -> &mut Self {
        self.route(Method::DELETE, path, handler)
    }

    /// Binds to `addr` and serves requests forever.
    ///
    /// `addr` accepts a full socket address (`127.0.0.1:3000`) or a Go-style
    /// port-only form (`:3000`), which binds all interfaces.
    pub async fn listen(self, addr: &str) -> io::Result<()> {
        let addr = match addr.strip_prefix(':') {
            Some(port) => format!("0.0.0.0:{port}"),
            None => addr.to_owned(),
        };
        let listener = TcpListener::bind(&addr).await?;
        self.serve(listener).await
    }

    /// Serves requests on an already-bound listener.
    pub async fn serve(self, listener: TcpListener) -> io::Result<()> {
        let router = Arc::new(self.router);

        loop {
            let (stream, _) = match listener.accept().await {
                Ok(conn) => conn,
                // Per-connection errors (e.g. aborted handshakes, fd exhaustion)
                // shouldn't take the whole server down.
                Err(_) => continue,
            };
            let router = router.clone();

            tokio::spawn(async move {
                let service = service_fn(move |req| {
                    let router = router.clone();
                    async move { Ok::<_, Infallible>(dispatch(&router, req).await) }
                });
                let _ = auto::Builder::new(TokioExecutor::new())
                    .serve_connection(TokioIo::new(stream), service)
                    .await;
            });
        }
    }
}

async fn dispatch(router: &Router, req: hyper::Request<Incoming>) -> hyper::Response<Full<Bytes>> {
    let (parts, body) = req.into_parts();

    let (handler, params) = match router.find(&parts.method, parts.uri.path()) {
        Match::Found(handler, params) => (handler.clone(), params),
        Match::MethodNotAllowed => return error(StatusCode::METHOD_NOT_ALLOWED),
        Match::NotFound => return error(StatusCode::NOT_FOUND),
    };

    let body = match body.collect().await {
        Ok(collected) => collected.to_bytes(),
        Err(_) => return error(StatusCode::BAD_REQUEST),
    };

    let ctx = Context::new(parts.method, parts.uri, parts.headers, params, body);
    handler(ctx).await.into_hyper()
}

fn error(status: StatusCode) -> hyper::Response<Full<Bytes>> {
    Response::text(status.canonical_reason().unwrap_or(""))
        .with_status(status)
        .into_hyper()
}
