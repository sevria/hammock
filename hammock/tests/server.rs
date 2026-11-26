use std::net::SocketAddr;

use hammock::{Context, Server, StatusCode};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};

async fn spawn(server: Server) -> SocketAddr {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(server.serve(listener));
    addr
}

/// Sends a raw HTTP/1.1 request and returns the full response text.
async fn send(addr: SocketAddr, method: &str, path: &str, body: &str) -> String {
    let mut stream = TcpStream::connect(addr).await.unwrap();
    let req = format!(
        "{method} {path} HTTP/1.1\r\nHost: x\r\nConnection: close\r\nContent-Length: {}\r\n\r\n{body}",
        body.len()
    );
    stream.write_all(req.as_bytes()).await.unwrap();
    let mut out = String::new();
    stream.read_to_string(&mut out).await.unwrap();
    out
}

#[tokio::test]
async fn routes_requests() {
    let mut server = Server::new();
    server
        .get("/", |ctx: Context| ctx.string("Hello, World!"))
        .get("/users/me", |ctx: Context| ctx.string("me"))
        .get("/users/:id", |ctx: Context| {
            ctx.string(format!("user {}", ctx.param("id").unwrap()))
        })
        .get("/search", |ctx: Context| {
            ctx.string(ctx.query("q").unwrap_or_default())
        })
        .post("/echo", |ctx: Context| async move {
            let v: serde_json::Value = ctx.bind_json().unwrap();
            ctx.json(&v).with_status(StatusCode::CREATED)
        });
    let addr = spawn(server).await;

    let res = send(addr, "GET", "/", "").await;
    assert!(res.starts_with("HTTP/1.1 200"), "{res}");
    assert!(res.contains("text/plain"));
    assert!(res.ends_with("Hello, World!"));

    assert!(
        send(addr, "GET", "/users/me", "")
            .await
            .ends_with("\r\n\r\nme")
    );
    assert!(
        send(addr, "GET", "/users/42", "")
            .await
            .ends_with("user 42")
    );
    assert!(
        send(addr, "GET", "/search?q=a+b%21", "")
            .await
            .ends_with("a b!")
    );

    let res = send(addr, "POST", "/echo", r#"{"a":1}"#).await;
    assert!(res.starts_with("HTTP/1.1 201"), "{res}");
    assert!(res.contains("application/json"));
    assert!(res.ends_with(r#"{"a":1}"#));

    assert!(
        send(addr, "GET", "/nope", "")
            .await
            .starts_with("HTTP/1.1 404")
    );
    assert!(
        send(addr, "DELETE", "/", "")
            .await
            .starts_with("HTTP/1.1 405")
    );
}
