use actix_web::{web, App, HttpResponse, HttpServer};
use log::{debug, info};
use serde_json::{json, Value};
use simplelog::{ConfigBuilder, LevelFilter, SimpleLogger};
use std::env;

async fn handle(update: web::Json<Value>) -> HttpResponse {
    // Get untyped (fallible) information from message
    let chat_id = &update["message"]["chat"]["id"];
    let user_id = &update["message"]["from"]["id"];
    let user_name = &update["message"]["from"]["first_name"];
    let text = &update["message"]["text"];

    // If any of these is Null, don't respond
    if [chat_id, user_id, user_name, text]
        .iter()
        .any(|&v| v == &Value::Null)
    {
        info!("An expected value was null: {}", update);
        return HttpResponse::Ok().finish();
    }

    debug!("Received valid request, responding: {}", update);

    // Respond using the same HTTP request
    let response = format!(
        "Hello, {}! You sent:\n{}",
        user_name.as_str().unwrap(),
        text
    );
    HttpResponse::Ok().json(
        json!({"method": "sendMessage", "chat_id": chat_id, "text": response}),
    )
}

#[actix_rt::main]
async fn main() -> std::io::Result<()> {
    // Set up logging facility
    SimpleLogger::init(
        LevelFilter::Debug,
        ConfigBuilder::new().set_time_to_local(true).build(),
    )
    .expect("Failed initializing logger");

    // The endpoint we're listening on is a secret
    let endpoint = env::var("TELEGRAM_WEBHOOK")
        .expect("Webhook environment variable not set");

    // Start server listening for webhook requests
    HttpServer::new(move || {
        App::new().route(&endpoint, web::post().to(handle))
    })
    .bind("0.0.0.0:8080")?
    .run()
    .await
}
