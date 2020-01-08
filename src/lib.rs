use actix_web::{web, HttpResponse};
use bson::{bson, doc};
use log::{debug, error, info};
use mongodb::Collection;
use serde_json::{json, Value};
use tokio::task;

mod error;

/// Deals with the Telegram Bot API, delegating the processing of the message.
pub async fn handle_webhook(
    update: web::Json<Value>,
    users: web::Data<Collection>,
) -> HttpResponse {
    if let Ok((chat_id, user_id, user_name, text)) =
        extract_message_data(&update)
    {
        debug!("Received valid request: {}", update);
        // Get our response
        let response = match handle_message(user_id, user_name, text, &users)
            .await
        {
            Ok(response) => response,
            Err(msg) => {
                error!("Error while responding: {}", msg);
                "I encountered an internal error. Sorry about that 😬".into()
            }
        };
        // Reply by responding to the original HTTP request
        HttpResponse::Ok().json(json!(
            {"method": "sendMessage", "chat_id": chat_id, "text": response}
        ))
    } else {
        info!("Unable to convert all expected values: {}", update);
        // Send no response
        HttpResponse::Ok().finish()
    }
}

/// Returns the converted `chat_id`, `user_id`, `user_name` and `text` from the
/// message or fails if one is unavailable.
fn extract_message_data(update: &Value) -> Result<(u64, u64, &str, &str), ()> {
    // Get untyped (fallible) information from message
    let chat_id = &update["message"]["chat"]["id"];
    let user_id = &update["message"]["from"]["id"];
    let user_name = &update["message"]["from"]["first_name"];
    let text = &update["message"]["text"];

    // Attempt conversion to what we'll actually use
    let chat_id = chat_id.as_u64().ok_or(())?;
    let user_id = user_id.as_u64().ok_or(())?;
    let user_name = user_name.as_str().ok_or(())?;
    let text = text.as_str().ok_or(())?;

    Ok((chat_id, user_id, user_name, text))
}

/// Deals with the user message and provides a response.
async fn handle_message(
    user_id: u64,
    user_name: &str,
    text: &str,
    users: &Collection,
) -> Result<String, error::InternalError> {
    // This is unfortunately necessary for spawn_blocking
    let users_f = users.clone();
    let users_i = users.clone();
    // Query user data and add if it doesn't exist yet
    if let Some(user_data) = task::spawn_blocking(move || {
        users_f.find_one(doc! {"user_id": user_id}, None)
    })
    .await??
    {
        Ok(format!("Welcome back, {}! You sent:\n{}", user_name, text))
    } else {
        info!("New user {} with ID {}", user_name, user_id);
        task::spawn_blocking(move || {
            users_i.insert_one(doc! {"user_id": user_id}, None)
        })
        .await??;
        Ok(format!("Welcome, {}! You were added to the DB.", user_name))
    }
}
