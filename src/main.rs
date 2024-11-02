use colorize::AnsiColor;
use inquire::InquireError::{OperationCanceled, OperationInterrupted};
use inquire::{Confirm, Select, Text};
use reqwest::blocking::Client;
use serde_json::json;
use std::fs;
use strum_macros::Display;

#[derive(Display, Debug)]
enum ProtocolOptions {
    Online,
    User,
    Quit,
}

use ProtocolOptions::*;

struct SentimentReport {
    neutral_score: f64,
    positive_score: f64,
    negative_score: f64,
}
/// input feed protocol loop
/// Take input
/// Process it (give user some kind of waiting prompt to account for delay)
/// Return in a nicely formatted the result of the result of the analysis
/// Ask for input for again and run the loop again
/// If termination signal is given,
/// Give an option to save the analysis results to a file
///
/// This function serves as a second main if user selected
/// the User option for the source of data.
///
/// It will keep running until user terminates or an unrecoverable error occurs.
/// Alternatively allow use to go from user input strings to online feed.
fn user_input_feed_protocol(client: &Client, huggingface_api_key: &str) {
    loop {
        // retrieve from user
        let user_post = match Text::new(
            "Enter the text you want to analyze (You can leave at any point using ESC/Ctrl-C):  ",
        )
        .prompt()
        {
            Ok(user_prompt) => user_prompt,
            Err(OperationCanceled | OperationInterrupted) => {
                eprintln!(
                    "{}",
                    "Received termination signal. Program will now gracefully terminate.".yellow()
                );
                std::process::exit(0);
            }
            Err(err) => {
                eprintln!("{}", format!("An error ocurred: {err}").red());
                std::process::exit(-1);
            }
        };

        match sentiment_analysis_request(client, &user_post, huggingface_api_key) {
            Ok(sentiment_analysis) => {
                dbg!(sentiment_analysis);
            }
            Err(_) => {
                let try_again = match Confirm::new(
                    "The prompt sentiment analysis failed. Do you want to try again?",
                )
                .with_help_message(
                    "If not program will terminate (in case of failure you will be prompted again)",
                )
                .prompt()
                {
                    Ok(should_try_again) => should_try_again,
                    Err(OperationCanceled | OperationInterrupted) => {
                        eprintln!(
                            "{}",
                            "A termination signal has been sent. Program will terminate.".red()
                        );
                        std::process::exit(-1);
                    }
                    Err(err) => {
                        eprintln!("An error occurred as we were awaiting confirmation from user : {err}\nProgram will now terminate");
                        std::process::exit(-1);
                    }
                };

                if try_again {
                    todo!("Not yet implemented")
                }
            }
        };
    }
}

fn online_feed_protocol() {}

fn sentiment_analysis_request(
    client: &Client,
    text: &str,
    api_key: &str,
) -> Result<String, String> {
    let response = client
        .post(DEFAULT_MODEL_PATH)
        .bearer_auth(api_key)
        .json(&json!({"inputs":text}))
        .send()
        .map_err(|e| e.to_string())?;

    if response.status().is_success() {
        let sentiment_analysis = response
            .text()
            .expect("Should have been able to read sentiment analysis.");
        Ok(sentiment_analysis)
    } else {
        let error_message = format!(
            "Sentiment analysis failed with status {}",
            response.status()
        )
        .red();
        Err(error_message)
    }
}

fn check_for_api_key_file() -> (bool, String) {
    let api_key = fs::read_to_string(API_KEY_SAVE_PATH).unwrap_or_default();

    if api_key.is_empty() {
        (false, api_key)
    } else {
        (true, api_key)
    }
}

fn prompt_user_for_api_key(client: &Client) -> Result<String, reqwest::Error> {
    println!(
        "{}",
        format!("Please provide the API key to HuggingFace API key to use for sentiment analysis")
            .yellow()
    );
    loop {
        match Text::new("Enter key here (should have Inference perm): ").prompt() {
            Ok(api_key) => {
                let payload = json!({"inputs": "Hello, I will make money, retire my parents, and escape from the rat race. Then I'll learn mandarin."});

                let response = client
                    .post(DEFAULT_MODEL_PATH)
                    .bearer_auth(&api_key)
                    .json(&payload)
                    .send()?;

                if response.status().is_success() {
                    return Ok(api_key);
                } else {
                    eprintln!("{}", "API key validation failed. Please try again.".red());
                }
            }
            Err(OperationCanceled | OperationInterrupted) => {
                std::process::exit(0);
            }
            Err(err) => {
                eprintln!(
                    "{}",
                    format!("Some error {err} occurred as we were awaiting API Key").red()
                )
            }
        }
    }
}

///
/// Save api_key_to_file at this point should have already been validated by the prompt method
fn save_api_key_to_file(huggingface_api_key: &str) {
    match fs::write(API_KEY_SAVE_PATH, huggingface_api_key) {
        Ok(_) => {
            println!("Saved API key succesfully");
        }
        Err(e) => {
            eprintln!("{}", format!("Failed to save API key: {e}").red());
        }
    };
}

static API_KEY_SAVE_PATH: &str = "./saved_key.txt";
static DEFAULT_MODEL_PATH: &str =
    "https://api-inference.huggingface.co/models/cardiffnlp/twitter-roberta-base-sentiment-latest";

/// Entry point of the program.
///
/// Takes inputs from the user and then redirect to the proper function.
///
/// 1. Get API key from USER (or file)
/// 2. Get a feed of data
/// 3. Run the transformer on the sentiment classifier
///
/// We want the ability:
///  - to pass either a user feed and classify sentiment of what user says
///  - taking a feed of data (say Tweeter/RSS) and then evaluating the sentiment of the thing
///
/// This could be easily used as a component for a trade signal given the right feed.
fn main() {
    //General Command Flow
    //I. Check that saved API key exists, and if it does retrieve it (else prompt user)
    // For the former just check if the file exists for the latter just prompt for a key and attempt connection.
    let (api_key_was_saved, api_key_from_file) = check_for_api_key_file();
    let client = Client::new();
    let huggingface_api_key = if api_key_was_saved {
        api_key_from_file
    } else {
        prompt_user_for_api_key(&client)
            .expect("Should have been able to get the content from API key if valid")
    };
    if !api_key_was_saved {
        //II. Ask user if we should save it, and if so we save:
        let should_save_api_key_to_file = match Confirm::new("Should we save the API_KEY File")
            .with_default(false)
            .with_help_message("In this implementation, API key is not encrypted.")
            .prompt()
        {
            Ok(reply) => reply,
            Err(err) => {
                eprintln!(
                "There was an error in confirming if should save API key. Terminating process\nError Code: {err}"
            );
                std::process::exit(-1);
            }
        };

        if should_save_api_key_to_file {
            save_api_key_to_file(&huggingface_api_key);
        }
    }

    //III. Prompt the user for which path we should elect and jump to the respective logic.
    let protocol_options = vec![Online, User, Quit];
    let protocol_selection = Select::new(
        "From where will the data to analyze be coming?: ",
        protocol_options,
    )
    .prompt();

    match protocol_selection {
        Ok(choice) => match choice {
            Online => online_feed_protocol(),
            User => user_input_feed_protocol(&client, &huggingface_api_key),
            Quit => std::process::exit(0),
        },
        Err(OperationCanceled | OperationInterrupted) => {
            println!("Operation was interrupted or escaped. Terminating.");
            std::process::exit(1);
        }
        Err(err) => {
            println!("{}", format!("An error occured as we were waiting for protocol selection. \nError Code: {err}\n\nProgram will now terminate").red());
            std::process::exit(-1);
        }
    }
}
