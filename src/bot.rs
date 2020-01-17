//! Functionality related to the Telegram Bot API and bot behavior.

use actix_web::{web, HttpResponse};
use bson::{bson, doc};
use log::{debug, error, info};
use mongodb::Collection;
use serde_json::{json, Value};
use std::sync::Arc;

use crate::{Dialogue, State};

/// Convenience type for wrapping blocking DB access in async.
type DbError = actix_threadpool::BlockingError<mongodb::error::Error>;

/// Deals with the Telegram Bot API, delegating the processing of the message.
pub async fn handle_webhook(
    update: web::Json<Value>,
    users: web::Data<Collection>,
    dialogue: web::Data<Dialogue>,
) -> HttpResponse {
    if let Ok((chat_id, user_id, user_name, text)) =
        extract_message_data(&update)
    {
        debug!("Received valid request: {}", update);
        // Get our response
        let response = match handle_message(
            user_id,
            user_name.into(),
            text.into(),
            users.into_inner(),
            dialogue.into_inner(),
        )
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
    user_name: String,
    text: String,
    users: Arc<Collection>,
    dialogue: Arc<Dialogue>,
) -> Result<String, DbError> {
    // Since the MongoDB driver doesn't offer an async API yet, we have to
    // manually move operations into a dedicated blocking threadpool. Because
    // this code has a few DB accesses and the Dialogue it uses does as well,
    // the Dialogue API was kept blocking and we just run the whole thing
    // off-thread.
    actix_threadpool::run(move || {
        // Query user data and add if it doesn't exist yet
        let current_state: State = if let Some(user_data) =
            users.find_one(doc! {"user_id": user_id}, None)?
        {
            user_data
                .get_i32("current_state")
                .unwrap_or_else(|_| State::Initial.into())
                .into()
        } else {
            info!("New user {} with ID {}", user_name, user_id);
            users.insert_one(
                doc! {"user_id": user_id, "user_name": user_name},
                None,
            )?;
            State::Initial
        };

        debug!("{} currently in state {:?}", user_id, current_state);
        // Use the state machine to get the next state
        let (next_state, state_msg, transition_msg) =
            dialogue.advance(&text, current_state, user_id, &users)?;
        debug!("Next state for {}: {:?}", user_id, next_state);

        // Update data with the next state
        users.update_one(
            doc! {"user_id": user_id},
            doc! {"$set": {"current_state": i32::from(next_state) }},
            None,
        )?;

        Ok(if let Some(transition_msg) = transition_msg {
            format!("{}\n{}", transition_msg, state_msg)
        } else {
            state_msg
        })
    })
    .await
}
