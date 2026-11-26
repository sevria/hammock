//! Hammock: Express-style REST APIs for Rust.
//!
//! ```no_run
//! use hammock::{Context, Server};
//!
//! #[tokio::main]
//! async fn main() {
//!     let mut server = Server::new();
//!
//!     server.get("/", |ctx: Context| ctx.string("Hello, World!"));
//!
//!     server.listen(":3000").await.unwrap();
//! }
//! ```

mod context;
mod handler;
mod response;
mod router;
mod server;

pub use context::Context;
pub use handler::Handler;
pub use response::{IntoResponse, Response};
pub use server::Server;

pub use hyper::{HeaderMap, Method, StatusCode, header};
