//! Reminder functionality that is separate from the dialogue.

use actix::clock;
use bson::{bson, doc};
use chrono::prelude::*;
use log::{debug, error};
use mongodb::Collection;
use rand::prelude::*;
use std::time::Duration;

use crate::bot;

/// Periodically checks all users for missed runs and reminds them.
pub async fn remind(users: Collection, token: String) {
    let interval = Duration::from_secs(3600);

    loop {
        // clock::delay_until would've been a more elegant option, but waiting
        // for some interval was more handy during testing and overall it
        // doesn't really hurt to go through all users every hour or so at our
        // scale.
        clock::delay_for(interval).await;

        // Skip if it's not between 20:00 and 20:59. Ideally this would be done
        // on a user-basis (remembering their TZ), but this will do for now.
        let now = Local::now();
        if now.hour() < 20 || now.hour() >= 21 {
            debug!("It's not time yet");
            continue;
        }

        debug!("Going through users to issue reminders");
        // This is unfortunately necessary, since MongoDB's API is blocking
        let users_cpy = users.clone();
        if let Ok(cursor) =
            actix_threadpool::run(move || users_cpy.find(doc! {}, None)).await
        {
            for result in cursor {
                match result {
                    Ok(mut user) => {
                        if let Ok(date) = user.get_str("planned_date") {
                            // Get date & skip to next user if it hasn't come
                            let date =
                                DateTime::parse_from_rfc3339(date).unwrap();
                            if date.date()
                                > Utc::now()
                                    .with_timezone(&date.timezone())
                                    .date()
                            {
                                continue;
                            }

                            // Since the date has passed, remind the user
                            let chat_id = user.get_i64("chat_id").unwrap();
                            let user_id =
                                user.get_i64("user_id").unwrap().to_owned();
                            // Choose a message randomly
                            let messages = [
                                "Did you manage to go on that run we planned?",
                                "Have you had your run today?",
                                "Did you go on that run you wanted today?",
                                "Have you had a chance to go running today?",
                            ];
                            let &chosen = messages
                                .choose(&mut rand::thread_rng())
                                .unwrap();
                            debug!("Reminding {}", user_id);
                            if let Err(e) =
                                bot::send_message(chat_id, chosen, &token)
                                    .await
                            {
                                error!(
                                    "Unable to send message to {}: {}",
                                    user_id, e
                                );
                            } else {
                                // Delete the date to not remind repeatedly
                                user.remove("planned_date");
                                // Again, workaround for blocking API
                                let users_cpy = users.clone();
                                if let Err(e) =
                                    actix_threadpool::run(move || {
                                        users_cpy.update_one(
                                            doc! {"user_id": user_id},
                                            user,
                                            None,
                                        )
                                    })
                                    .await
                                {
                                    error!(
                                        "Unable to update user {}: {}",
                                        user_id, e
                                    );
                                }
                            }
                        }
                    }
                    Err(e) => error!("Unable to read single user: {}", e),
                }
            }
        } else {
            error!("Unable to read user collection");
        }
    }
}
