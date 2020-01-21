//! Dialogue flow control.

use bson::{bson, doc, Bson, Document};
use chrono::{prelude::*, Duration};
use log::debug;
use mongodb::Collection;
use rand::prelude::*;
use reqwest::blocking::Client;
use serde_json::Value;
use std::{collections::HashMap, error, fmt};

/// The states our dialogue supports.
#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy)]
pub enum State {
    Initial,
    DetermineExperience,
    ScheduleFirstRun,
    AskAboutRun,
    SuggestChange,
    AskAlternative,
}

impl From<i32> for State {
    fn from(id: i32) -> Self {
        match id {
            0 => Self::Initial,
            30 => Self::DetermineExperience,
            40 => Self::ScheduleFirstRun,
            50 => Self::AskAboutRun,
            60 => Self::SuggestChange,
            70 => Self::AskAlternative,
            // This is not entirely correct and should be turned into a TryFrom
            // implementation instead
            _ => Self::Initial,
        }
    }
}

impl From<State> for i32 {
    fn from(state: State) -> i32 {
        match state {
            State::Initial => 0,
            State::DetermineExperience => 30,
            State::ScheduleFirstRun => 40,
            State::AskAboutRun => 50,
            State::SuggestChange => 60,
            State::AskAlternative => 70,
        }
    }
}

/// The "value" of a state, containing the message and transition function, as
/// well as the error message.
#[derive(Clone)]
struct StateContent {
    message: &'static (dyn Fn(u64, &Collection) -> Result<String, FlowError>
                  + Send
                  + Sync),
    error: &'static str,
    #[allow(clippy::type_complexity)]
    transition: &'static (dyn Fn(
        &str,
        u64,
        &Collection,
        &str,
    ) -> Result<(State, Option<String>), FlowError>
                  + Send
                  + Sync),
}

/// The state machine for dialogue handling.
#[derive(Clone)]
pub struct Dialogue {
    state_table: HashMap<State, StateContent>,
    current_state: State,
}

impl Dialogue {
    /// Initializes a new state machine in a given state.
    pub fn from_state(state: State) -> Self {
        let mut state_table: HashMap<State, StateContent> = HashMap::new();

        state_table.insert(
            State::Initial,
            StateContent {
                message: &|_, _| Ok("Sorry to see you go.".into()),
                error: "You'll never see this error",
                transition: &|_, _, _, _| {
                    Ok((State::DetermineExperience, None))
                },
            },
        );

        state_table.insert(
            State::DetermineExperience,
            StateContent {
                message: &|user_id, users| {
                    // Both these unwraps are safe, because we always have a
                    // user with a name
                    let user = users.find_one(doc! {"user_id": user_id}, None)?.unwrap();
                    let user_name = user.get_str("user_name").unwrap();
                    let messages: &[&str] = &[
                        &format!("Hi {}, good to see you! Do you have any running experience?", user_name),
                        &format!("Welcome {}, it's great to have you! Did you use to run before?", user_name),
                        &format!("Hello {}, let's get you started! Did you do much running until now?", user_name),
                    ];
                    select_message(messages, user_id, users)
                },
                error: "I'm afraid I don't understand that. Do you have any running experience?",
                transition: &|response, _, _, wit| {
                    let api_resp = wit_ai(response, wit)?;
                    match api_resp["entities"]["response"][0]["value"].as_str() {
                        Some("positive") => Ok((State::ScheduleFirstRun, Some("Great to hear!".into()))),
                        Some("negative") => Ok((State::ScheduleFirstRun, Some("That's fine, don't worry about it.".into()))),
                        _ => Err(FlowError::NoMatch),
                    }
                },
            },
        );

        state_table.insert(
            State::ScheduleFirstRun,
            StateContent {
                message: &|user_id, users| {
                    select_message(&[
                        "When and for how long would you like to go running?",
                        "When do you want to go on your next run, and for how long?",
                    ], user_id, users)
                },
                error: "I sometimes have trouble distinguishing the duration and the date. Could you try again in a different way?",
                transition: &|response, user_id, users, wit| {
                    let api_resp = wit_ai(response, wit)?;
                    match (
                        api_resp["entities"]["datetime"][0]["value"].as_str(),
                        api_resp["entities"]["duration"][0]["normalized"]["value"].as_i64()
                    ) {
                        (Some(date), Some(seconds)) => {
                            // Parse date and duration
                            let date = DateTime::parse_from_rfc3339(date)?;
                            let duration = Duration::seconds(seconds);
                            // Fetch the user's document, which we know exists
                            let mut user_doc = users.find_one(doc! {"user_id": user_id}, None)?.unwrap();
                            // Insert the values we want to remember
                            user_doc.insert("planned_date", date.to_string());
                            user_doc.insert("planned_duration", duration.num_seconds());
                            // Store update
                            users.update_one(doc! {"user_id": user_id}, user_doc, None)?;
                            debug!("{} committed to {} minutes on {}", user_id, duration.num_minutes(), date.format("%Y-%m-%d"));
                            Ok((State::AskAboutRun, None))
                        },
                        _ => {
                            Err(FlowError::NoMatch)
                        }
                    }
                },
            },
        );

        state_table.insert(
            State::AskAboutRun,
            StateContent {
                message: &|user_id, users| {
                    select_message(&[
                        "Awesome, let me know how it went as soon as you're back!",
                        "That's great, tell me how it went when you're done!",
                        "Very cool, be sure to tell me about it afterwards!"
                    ], user_id, users)
                },
                error: "I didn't understand that, please let me know how your run went.",
                transition: &|response, user_id, users, wit| {
                    // Fetch the user's document, which we know exists
                    let mut user_doc = users.find_one(doc! {"user_id": user_id}, None)?.unwrap();
                    let user_name = user_doc.get_str("user_name").unwrap();
                    let api_resp = wit_ai(response, wit)?;
                    // Abort early if no sentiment was detected
                    let sentiment = api_resp["entities"]["sentiment"][0]["value"].as_str();
                    if sentiment.is_none() {
                        return Err(FlowError::NoMatch);
                    }
                    // Otherwise, determine the response
                    let sentiment = sentiment.unwrap();
                    let response = match sentiment {
                        "positive" => select_message(&[
                            &format!("Awesome work, {}!", user_name),
                            "Amazing!",
                        ], user_id, users),
                        "neutral" => select_message(&[
                            &format!("That's pretty good, keep it up, {}.", user_name),
                            "Nice!",
                        ], user_id, users),
                        "negative" => select_message(&[
                            &format!("No worries {}, you'll get there.", user_name),
                            "Never mind that, just keep trying.",
                        ], user_id, users),
                        _ => unreachable!(),
                    }?;
                    // and store the experience
                    user_doc.insert("last_experience", sentiment);
                    users.update_one(doc! {"user_id": user_id}, user_doc, None)?;

                    Ok((State::SuggestChange, Some(response)))
                },
            },
        );

        state_table.insert(
            State::SuggestChange,
            StateContent {
                message: &|user_id, users| {
                    // Fetch the user's document and the experience, which we
                    // know exist
                    let user_doc = users.find_one(doc! {"user_id": user_id}, None)?.unwrap();
                    let experience = user_doc.get_str("last_experience").unwrap();
                    let mut duration = Duration::seconds(user_doc.get_i64("planned_duration").unwrap());
                    let mut date = Utc::now();
                    let response = match experience {
                        "positive" => {
                            // If it went well, take a day and add 5 minutes
                            duration = duration + Duration::minutes(5);
                            date = date + Duration::days(2);
                            select_message(&[
                                &format!("Since it went so well, why don't you take a break and try {} minutes the day after tomorrow?", duration.num_minutes()),
                                &format!("It looks like you can even try {} minutes in two days, what do you think?", duration.num_minutes()),
                            ], user_id, users)
                        },
                        "neutral" => {
                            // If it was ok, take a day and keep the duration
                            date = date + Duration::days(2);
                            select_message(&[
                                &format!("{} minutes seems to be a good time for you right now, how about you go again in two days?", duration.num_minutes()),
                                &format!("It's going well for you, do you want to do {} minutes again the day after tomorrow?", duration.num_minutes()),
                            ], user_id, users)
                        },
                        "negative" => {
                            // If it went badly, try 5 minutes less tomorrow
                            duration = duration - Duration::minutes(5);
                            date = date + Duration::days(1);
                            select_message(&[
                                &format!("It's probably best to go again tomorrow, but only for {} minutes. Is that ok?", duration.num_minutes()),
                                &format!("Why don't you reduce the duration to {} minutes, but try again tomorrow?", duration.num_minutes()),
                            ], user_id, users)
                        },
                        _ => unreachable!(),
                    };

                    // Read the user data again, just to be sure (since we
                    // change it in select_message)
                    let mut user_doc = users.find_one(doc! {"user_id": user_id}, None)?.unwrap();
                    // Insert the values we want to remember
                    user_doc.insert("planned_date", date.to_string());
                    user_doc.insert("planned_duration", duration.num_seconds());
                    users.update_one(doc! {"user_id": user_id}, user_doc, None)?;
                    debug!("Suggested {} for {} minutes to {}", date.format("%Y-%m-%d"), duration.num_minutes(), user_id);

                    response
                },
                error:
                    "Sorry, I don't get it. Does my suggestion work for you?",
                transition: &|response, user_id, users, wit| {
                    let api_resp = wit_ai(response, wit)?;
                    match (
                        api_resp["entities"]["datetime"][0]["value"].as_str(),
                        api_resp["entities"]["duration"][0]["normalized"]["value"].as_i64(),
                        api_resp["entities"]["response"][0]["value"].as_str(),
                    ) {
                        // This is the case where the user makes another
                        // proposal
                        (Some(date), Some(seconds), _) => {
                            // Parse date and duration
                            let date = DateTime::parse_from_rfc3339(date)?;
                            let duration = Duration::seconds(seconds);
                            // Fetch the user's document, which we know exists
                            let mut user_doc = users.find_one(doc! {"user_id": user_id}, None)?.unwrap();
                            // Insert the values we want to remember
                            user_doc.insert("planned_date", date.to_string());
                            user_doc.insert("planned_duration", duration.num_seconds());
                            // Store update
                            users.update_one(doc! {"user_id": user_id}, user_doc, None)?;
                            debug!("{} committed to {} minutes on {}", user_id, duration.num_minutes(), date.format("%Y-%m-%d"));
                            Ok((State::AskAboutRun, None))
                        },
                        // User agrees
                        (_, _, Some("positive")) => Ok((State::AskAboutRun, None)),
                        // User disagrees without alternative
                        (_, _, Some("negative")) => Ok((State::AskAlternative, None)),
                        _ => Err(FlowError::NoMatch)
                    }
                },
            },
        );

        state_table.insert(
            State::AskAlternative,
            StateContent {
                message: &|user_id, users| {
                    select_message(&[
                        "Then what do you want to do?",
                        "So what would you prefer?",
                    ], user_id, users)
                },
                error: "Could you try telling me what you'd like to do instead again?",
                transition: &|response, user_id, users, wit| {
                    let api_resp = wit_ai(response, wit)?;
                    match (
                        api_resp["entities"]["datetime"][0]["value"].as_str(),
                        api_resp["entities"]["duration"][0]["normalized"]["value"].as_i64(),
                        api_resp["entities"]["intent"][0]["value"].as_str(),
                    ) {
                        // This is the case where the user makes another
                        // proposal
                        (Some(date), Some(seconds), _) => {
                            // Parse date and duration
                            let date = DateTime::parse_from_rfc3339(date)?;
                            let duration = Duration::seconds(seconds);
                            // Fetch the user's document, which we know exists
                            let mut user_doc = users.find_one(doc! {"user_id": user_id}, None)?.unwrap();
                            // Insert the values we want to remember
                            user_doc.insert("planned_date", date.to_string());
                            user_doc.insert("planned_duration", duration.num_seconds());
                            // Store update
                            users.update_one(doc! {"user_id": user_id}, user_doc, None)?;
                            debug!("{} committed to {} minutes on {}", user_id, duration.num_minutes(), date.format("%Y-%m-%d"));
                            Ok((State::AskAboutRun, None))
                        },
                        // User wants to quit
                        (_, _, Some("quit")) => {
                            // Delete user record
                            users.delete_one(doc! {"user_id": user_id}, None)?;
                            Ok((State::Initial, None))
                        },
                        _ => Err(FlowError::NoMatch)
                    }
                },
            },
        );

        Self {
            state_table,
            current_state: state,
        }
    }

    /// Puts the given input into the state machine and returns the result.
    ///
    /// The first message is the state's or the error message, the second
    /// the transition's.
    pub fn advance(
        &self,
        input: &str,
        previous_state: State,
        user_id: u64,
        collection: &Collection,
        wit: &str,
    ) -> Result<(State, String, Option<String>), FlowError> {
        // Get current state, which we know exists (safe to unwrap)
        let current_state = self
            .state_table
            .get(&previous_state)
            .expect("Current state not found");
        // Use transition function to get next state and optional message
        match (current_state.transition)(input, user_id, collection, wit) {
            Ok((next_state, transition_msg)) => {
                // Get next state's message (again safe to unwrap)
                let state_msg =
                    (self
                        .state_table
                        .get(&next_state)
                        .expect("Next state not found")
                        .message)(user_id, collection)?;
                // Return the transition's and next state's messages
                Ok((next_state, state_msg, transition_msg))
            }
            Err(FlowError::NoMatch) => {
                // Return just the state's error message
                Ok((previous_state, current_state.error.into(), None))
            }
            // Otherwise (MongoDB or reqwest error), just pass it on
            Err(e) => Err(e),
        }
    }
}

/// Checks how often the given messages have been shown and returns the index
/// of the least frequently used one. Only works for non-empty slices.
fn select_message(
    messages: &[&str],
    user_id: u64,
    users: &Collection,
) -> Result<String, FlowError> {
    debug!("Considering messages {:?}", messages);
    // Fetch the user's document, which we know exists
    let mut user_doc =
        users.find_one(doc! {"user_id": user_id}, None)?.unwrap();
    // Get (or create) the document containing the frequencies
    let frequencies_doc = user_doc
        .entry("frequencies".into())
        .or_insert(Bson::Document(Document::new()))
        .as_document_mut()
        .unwrap();
    // Build a list of the frequencies, initializing unknown ones with 0
    let frequencies: Vec<i32> = messages
        .iter()
        .map(|&m| frequencies_doc.get_i32(m).unwrap_or(0))
        .collect();
    debug!("Current frequencies are {:?}", frequencies);
    // Determine the minimum
    let &min = frequencies.iter().min().unwrap();
    // Get the list of indices of all least frequently used ones
    let infreq: Vec<usize> = frequencies
        .into_iter()
        .enumerate()
        .filter(|(_, frequency)| frequency == &min)
        .map(|(i, _frequency)| i)
        .collect();
    // Randomly select one if there were multiple
    let &chosen = infreq.choose(&mut rand::thread_rng()).unwrap();
    debug!("Chose message with index {}", chosen);
    // Increment the usage of the chosen message
    frequencies_doc.insert(messages[chosen], min + 1);
    users.update_one(doc! {"user_id": user_id}, user_doc, None)?;

    Ok(messages[chosen].into())
}

/// Makes a blocking request to the API of Wit.ai and returns the JSON
/// response.
fn wit_ai(input: &str, token: &str) -> Result<Value, reqwest::Error> {
    Client::new()
        .get("https://api.wit.ai/message")
        .bearer_auth(token)
        .query(&[("q", input)])
        .send()?
        .json()
}

/// The error returned from state and transition functions.
#[derive(Debug)]
pub enum FlowError {
    /// Error related to MongoDB.
    Database(mongodb::error::Error),
    /// Error related to Wit.ai request.
    Wit(reqwest::Error),
    /// Error related to date/time parsing.
    Time(chrono::ParseError),
    /// Unable to determine user intent.
    NoMatch,
}

impl From<mongodb::error::Error> for FlowError {
    fn from(e: mongodb::error::Error) -> Self {
        Self::Database(e)
    }
}

impl From<reqwest::Error> for FlowError {
    fn from(e: reqwest::Error) -> Self {
        Self::Wit(e)
    }
}

impl From<chrono::ParseError> for FlowError {
    fn from(e: chrono::ParseError) -> Self {
        Self::Time(e)
    }
}

impl fmt::Display for FlowError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::Database(e) => e.fmt(f),
            Self::Wit(e) => e.fmt(f),
            Self::Time(e) => e.fmt(f),
            Self::NoMatch => write!(f, "No match found for user input"),
        }
    }
}

impl error::Error for FlowError {}
