use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use crate::context::Context;
use crate::response::{IntoResponse, Response};

pub(crate) type BoxFuture = Pin<Box<dyn Future<Output = Response> + Send>>;
pub(crate) type BoxHandler = Arc<dyn Fn(Context) -> BoxFuture + Send + Sync>;

/// A request handler.
///
/// Implemented for both sync and async closures taking a [`Context`]:
///
/// ```ignore
/// server.get("/sync", |ctx: Context| ctx.string("hi"));
/// server.get("/async", |ctx: Context| async move { ctx.string("hi") });
/// ```
///
/// The `M` parameter is a marker that lets both forms coexist; you never name it.
pub trait Handler<M>: Send + Sync + 'static {
    fn call(&self, ctx: Context) -> BoxFuture;
}

#[doc(hidden)]
pub struct SyncHandler;

#[doc(hidden)]
pub struct AsyncHandler;

impl<F, R> Handler<SyncHandler> for F
where
    F: Fn(Context) -> R + Send + Sync + 'static,
    R: IntoResponse,
{
    fn call(&self, ctx: Context) -> BoxFuture {
        let res = (self)(ctx).into_response();
        Box::pin(std::future::ready(res))
    }
}

impl<F, Fut> Handler<AsyncHandler> for F
where
    F: Fn(Context) -> Fut + Send + Sync + 'static,
    Fut: Future + Send + 'static,
    Fut::Output: IntoResponse,
{
    fn call(&self, ctx: Context) -> BoxFuture {
        let fut = (self)(ctx);
        Box::pin(async move { fut.await.into_response() })
    }
}

pub(crate) fn boxed<H, M>(handler: H) -> BoxHandler
where
    H: Handler<M>,
{
    Arc::new(move |ctx| handler.call(ctx))
}
