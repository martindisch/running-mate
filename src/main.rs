use actix_web::{web, App, HttpServer};
use mongodb::Client;
use simplelog::{ConfigBuilder, LevelFilter, SimpleLogger};
use std::env;

/// The main entrypoint, which starts the web server.
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

    // Connect to DB & get handles to user DB and the books collection
    let client = Client::with_uri_str("mongodb://db:27017/")
        .expect("Unable to connect to MongoDB");
    let db = client.database("running-mate");
    let collection = db.collection("users");

    // Start server listening for webhook requests
    HttpServer::new(move || {
        App::new()
            .data(collection.clone())
            .route(&endpoint, web::post().to(running_mate::handle_webhook))
    })
    .bind("0.0.0.0:8080")?
    .run()
    .await
}
