//! Dialogue flow control.

use rand::prelude::*;
use std::collections::HashMap;

/// The states our dialogue supports.
#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy)]
pub enum State {
    Initial,
    CheckName,
    RequestName,
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
            10 => Self::CheckName,
            20 => Self::RequestName,
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
            State::CheckName => 10,
            State::RequestName => 20,
            State::DetermineExperience => 30,
            State::ScheduleFirstRun => 40,
            State::AskAboutRun => 50,
            State::SuggestChange => 60,
            State::AskAlternative => 70,
        }
    }
}

/// The state machine for dialogue handling.
#[derive(Clone)]
pub struct Dialogue<'a> {
    state_table: StateMap<'a>,
    current_state: State,
}

impl<'a> Dialogue<'a> {
    /// Initializes a new state machine in a given state.
    pub fn from_state(state: State) -> Self {
        let mut state_table: StateMap = HashMap::new();

        state_table.insert(
            State::Initial,
            (
                &|_| Ok("Sorry to see you go.".into()),
                "You'll never see this error",
                &|_| Ok(Ok((State::CheckName, None))),
            ),
        );

        state_table.insert(
            State::CheckName,
            (
                &|_| {
                    let messages = [
                        "Hi there! May I call you XXX?",
                        "Welcome, good to see you! Can I call you XXX?",
                    ];
                    let selected = select_message(&messages);
                    Ok(messages[selected].into())
                },
                "Sorry, I didn't quite catch that. Can I call you XXX?",
                &|response| match response {
                    "Sure" => Ok(Ok((State::DetermineExperience, None))),
                    "No, YYY" => Ok(Ok((State::DetermineExperience, None))),
                    "No" => Ok(Ok((State::RequestName, None))),
                    _ => Ok(Err(())),
                },
            ),
        );

        state_table.insert(
            State::RequestName,
            (
                &|_| {
                    let messages = [
                        "Then what can I call you?",
                        "What's your name then?",
                    ];
                    let selected = select_message(&messages);
                    Ok(messages[selected].into())
                },
                "Can you try again?",
                &|response| match response {
                    "YYY" => Ok(Ok((State::DetermineExperience, None))),
                    _ => Ok(Err(())),
                },
            ),
        );

        state_table.insert(
            State::DetermineExperience,
            (
                &|_| {
                    let messages = [
                        "Ok, YYY! Do you have any running experience?",
                        "Cool! Did you use to run before?",
                    ];
                    let selected = select_message(&messages);
                    Ok(messages[selected].into())
                },
                "I'm afraid I don't understand that. Do you have any running experience?",
                &|response| match response {
                    "Yes" => Ok(Ok((State::ScheduleFirstRun, Some("That's great!".into())))),
                    "No" => Ok(Ok((State::ScheduleFirstRun, Some("That's fine, don't worry about it. Let's get you started then.".into())))),
                    _ => Ok(Err(())),
                },
            ),
        );

        state_table.insert(
            State::ScheduleFirstRun,
            (
                &|_| {
                    let messages = [
                        "When and for how long would you like to go running?",
                        "When do you want to go on your next run, and for how long?",
                    ];
                    let selected = select_message(&messages);
                    Ok(messages[selected].into())
                },
                "Could you repeat when and how long you want your next run to be?",
                &|response| match response {
                    "Tomorrow, 30 minutes" => Ok(Ok((State::AskAboutRun, None))),
                    _ => Ok(Err(())),
                },
            ),
        );

        state_table.insert(
            State::AskAboutRun,
            (
                &|_| {
                    let messages = [
                        "Awesome, let me know how it went!",
                        "That's great, tell me how it went!",
                        "Very cool, be sure to tell me about it afterwards!"
                    ];
                    let selected = select_message(&messages);
                    Ok(messages[selected].into())
                },
                "I didn't understand that, please let me know how your run went.",
                &|response| match response {
                    "Good" => Ok(Ok((State::SuggestChange, Some("Very cool!".into())))),
                    "Not great" => Ok(Ok((State::SuggestChange, Some("Don't worry, you'll get there.".into())))),
                    _ => Ok(Err(())),
                },
            ),
        );

        state_table.insert(
            State::SuggestChange,
            (
                &|_| {
                    let messages = [
                        "How about you try 35 minutes tomorrow?",
                        "Do you want to go for 35 minutes tomorrow?",
                        "Think you can manage 35 minutes tomorrow?",
                    ];
                    let selected = select_message(&messages);
                    Ok(messages[selected].into())
                },
                "Sorry, I don't get it. Does my suggestion work for you?",
                &|response| match response {
                    "Sure" => Ok(Ok((State::AskAboutRun, None))),
                    "I think I can even do 40 minutes the day after" => {
                        Ok(Ok((State::AskAboutRun, None)))
                    }
                    "No" => Ok(Ok((State::AskAlternative, None))),
                    _ => Ok(Err(())),
                },
            ),
        );

        state_table.insert(
            State::AskAlternative,
            (
                &|_| {
                    let messages = [
                        "Then what do you want to do?",
                        "So what would you prefer?",
                    ];
                    let selected = select_message(&messages);
                    Ok(messages[selected].into())
                },
                "Could you try telling me what you'd like to do instead again?",
                &|response| match response {
                    "Quit" => Ok(Ok((State::Initial, None))),
                    "I think I can even do 40 minutes the day after" => Ok(Ok((State::AskAboutRun, None))),
                    _ => Ok(Err(())),
                },
            ),
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
    ) -> Result<(State, String, Option<String>), mongodb::error::Error> {
        // Get current state, which we know exists (safe to unwrap)
        let current_state = self
            .state_table
            .get(&previous_state)
            .expect("Current state not found");
        // Use transition function to get next state and optional message
        if let Ok((next_state, transition_msg)) = current_state.2(input)? {
            // Get next state's message (again safe to unwrap)
            let state_msg =
                self.state_table
                    .get(&next_state)
                    .expect("Next state not found")
                    .0("User ID & collection reference")?;
            // Return the transition's and next state's messages
            Ok((next_state, state_msg, transition_msg))
        } else {
            // Return just the state's error message
            Ok((previous_state, current_state.1.into(), None))
        }
    }
}

/// Checks how often the given messages have been shown and returns the index
/// of the least frequently used one. Only works for non-empty slices.
fn select_message(messages: &[&str]) -> usize {
    // Currently we just do it randomly
    rand::thread_rng().gen_range(0, messages.len())
}

/// Type alias for map of states with their message function, failure message
/// and transition function.
type StateMap<'a> = HashMap<
    State,
    (
        &'a dyn Fn(&str) -> Result<String, mongodb::error::Error>,
        &'a str,
        &'a dyn Fn(
            &str,
        ) -> Result<
            Result<(State, Option<String>), ()>,
            mongodb::error::Error,
        >,
    ),
>;
