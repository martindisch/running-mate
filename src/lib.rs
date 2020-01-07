use actix_web::{web, HttpResponse};
use log::{debug, info};
use serde_json::{json, Value};

/// Deals with the Telegram Bot API, delegating the processing of the message.
pub async fn handle_webhook(update: web::Json<Value>) -> HttpResponse {
    if let Ok((chat_id, user_id, user_name, text)) =
        extract_message_data(&update)
    {
        debug!("Received valid request: {}", update);
        // Get our response
        let response = handle_message(user_id, user_name, text).await;
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
async fn handle_message(user_id: u64, user_name: &str, text: &str) -> String {
    format!("Hello, {}! You sent:\n{}", user_name, text)
}
