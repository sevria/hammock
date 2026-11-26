use hammock::{Context, Server, StatusCode};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
struct User {
    id: u64,
    name: String,
}

#[tokio::main]
async fn main() {
    let mut server = Server::new();

    server
        .get("/users/:id", |ctx: Context| {
            let id = ctx.param("id").unwrap_or_default().parse().unwrap_or(0);
            ctx.json(&User {
                id,
                name: "Ada".into(),
            })
        })
        .post("/users", |ctx: Context| async move {
            match ctx.bind_json::<User>() {
                Ok(user) => ctx.json(&user).with_status(StatusCode::CREATED),
                Err(e) => ctx
                    .string(e.to_string())
                    .with_status(StatusCode::BAD_REQUEST),
            }
        });

    server.listen("127.0.0.1:3000").await.unwrap();
}
