use actix_web::{web, App, HttpResponse, HttpServer};
use serde_json::{json, Value};
use std::env;

async fn handle(update: web::Json<Value>) -> HttpResponse {
    // Get untyped (fallible) information from message
    let chat_id = &update["message"]["chat"]["id"];
    let user = &update["message"]["from"]["first_name"];
    let text = &update["message"]["text"];

    // Respond using the same HTTP request
    let response =
        format!("Hello, {}! You sent:\n{}", user.as_str().unwrap(), text);
    HttpResponse::Ok().json(
        json!({"method": "sendMessage", "chat_id": chat_id, "text": response}),
    )
}

#[actix_rt::main]
async fn main() -> std::io::Result<()> {
    // The endpoint we're listening on is a secret
    let endpoint = env::var("TELEGRAM_WEBHOOK").unwrap();

    // Start server listening for webhook requests
    HttpServer::new(move || {
        App::new().route(&endpoint, web::post().to(handle))
    })
    .bind("0.0.0.0:8080")?
    .run()
    .await
}
