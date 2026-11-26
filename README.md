# Hammock

Express-style REST APIs for Rust.

```rust
use hammock::{Context, Server};

#[tokio::main]
async fn main() {
    let mut server = Server::new();

    server.get("/", |ctx: Context| ctx.string("Hello, World!"));

    server.listen(":3000").await.unwrap();
}
```

## Installation

```toml
[dependencies]
hammock = { git = "https://github.com/sevria/hammock" }
tokio = { version = "1", features = ["macros", "rt-multi-thread"] }
```

## Routing

Register handlers with `get`, `post`, `put`, `patch`, `delete`, or `route` for any method. Calls can be chained.

```rust
server
    .get("/users", list_users)
    .get("/users/:id", get_user)       // named param
    .get("/files/*path", serve_file)   // wildcard, must be the last segment
    .route(Method::OPTIONS, "/", options);
```

- A route with no params takes priority over one with params, so `/users/me` wins over `/users/:id`.
- Otherwise, routes are matched in the order you register them.
- An unknown path returns `404 Not Found`. A known path with the wrong method returns `405 Method Not Allowed`.

## Handlers

A handler is a closure or function that takes a `Context`. It can be sync or async:

```rust
server.get("/sync", |ctx: Context| ctx.string("hi"));

server.get("/async", |ctx: Context| async move {
    let data = fetch_something().await;
    ctx.json(&data)
});
```

A handler can return any type that implements `IntoResponse`:

| Return type         | Response                    |
| ------------------- | --------------------------- |
| `Response`          | as-is                       |
| `String`, `&str`    | `200`, `text/plain`         |
| `StatusCode`        | that status, empty body     |
| `()`                | `204 No Content`            |
| `(StatusCode, T)`   | `T` with the given status   |
| `Result<T, E>`      | either side's response      |

## Context

Reading the request:

```rust
ctx.method();               // &Method
ctx.path();                 // "/users/42"
ctx.param("id");            // Option<&str>
ctx.query("page");          // Option<String>, percent-decoded
ctx.header("user-agent");   // Option<&str>
ctx.body();                 // &Bytes
ctx.bind_json::<T>();       // serde_json::Result<T>
```

Building a response:

```rust
ctx.string("text");                 // text/plain
ctx.html("<h1>hi</h1>");            // text/html
ctx.json(&value);                   // application/json
ctx.status(StatusCode::ACCEPTED);   // empty body
ctx.no_content();                   // 204
ctx.redirect("/login");             // 302

ctx.json(&user)
    .with_status(StatusCode::CREATED)
    .with_header("x-request-id", "abc");
```

## Listening

```rust
server.listen(":3000").await?;           // all interfaces, port 3000
server.listen("127.0.0.1:3000").await?;  // specific address

// Or use a listener you already bound (port 0 is useful in tests)
let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
server.serve(listener).await?;
```

The server handles both HTTP/1.1 and HTTP/2 (via hyper).

## Examples

```sh
cargo run -p hello-world
cargo run -p users
```

## Status

Early development. Not yet supported: middleware, graceful shutdown, request body size limits.

## License

[MIT](LICENSE)
