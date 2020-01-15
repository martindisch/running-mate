//! Functionality related to the Telegram Bot API and bot behavior.

use actix_web::{web, HttpResponse};
use bson::{bson, doc};
use log::{debug, error, info};
use mongodb::Collection;
use serde_json::{json, Value};

use crate::{Dialogue, State};

/// Convenience type for wrapping blocking DB access in async.
///
/// At least it used to be, now that we're blocking again it's just the normal
/// Mongo error.
type DbError = mongodb::error::Error;

/// Deals with the Telegram Bot API, delegating the processing of the message.
pub async fn handle_webhook(
    update: web::Json<Value>,
    users: web::Data<Collection>,
    dialogue: web::Data<Dialogue<'_>>,
) -> HttpResponse {
    if let Ok((chat_id, user_id, user_name, text)) =
        extract_message_data(&update)
    {
        debug!("Received valid request: {}", update);
        // Get our response
        let response =
            match handle_message(user_id, user_name, text, &users, &dialogue)
                .await
            {
                Ok(response) => response,
                Err(msg) => {
                    error!("Error while responding: {}", msg);
                    "I encountered an internal error. Sorry about that 😬"
                        .into()
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
    dialogue: &Dialogue<'_>,
) -> Result<String, DbError> {
    // Query user data and add if it doesn't exist yet (yes it's blocking and
    // bad, but we need it for simpler error handling right now)
    let current_state: State = if let Some(user_data) =
        users.find_one(doc! {"user_id": user_id}, None)?
    {
        user_data
            .get_i32("current_state")
            .unwrap_or_else(|_| State::Initial.into())
            .into()
    } else {
        info!("New user {} with ID {}", user_name, user_id);
        users.insert_one(doc! {"user_id": user_id}, None)?;
        State::Initial
    };

    debug!("{} currently in state {:?}", user_id, current_state);

    // Runs the closures on this thread, which may block on the DB and is bad.
    // The problem is that async closures aren't a thing yet and we can't do
    // the spawn_blocking thing, because our closures aren't safely shared
    // across threads. A solution could be moving to function pointers instead,
    // which is less elegant but also less problematic.
    let (next_state, state_msg, transition_msg) =
        dialogue.advance(text, current_state)?;

    debug!("Next state for {}: {:?}", user_id, next_state);

    // Again, badly blocking to update with next state
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
}
