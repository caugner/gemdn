use futures_util::stream::StreamExt;
use reqwest::Client;
use reqwest_eventsource::{Event, RequestBuilderExt};
use serde_json::json;
use std::env;

#[tokio::main]
async fn main() {
    let client = Client::new();
    let api_key = env::var("API_KEY").expect("Usage: API_KEY=... cargo run");

    println!("Sending request...");
    let req = client.post("https://generativelanguage.googleapis.com/v1beta/models/gemini-pro:streamGenerateContent")
        .query(&[("key", &api_key)])
        .json(&json!({
            "contents": [{
                "parts": [{
                    "text": "Write a story about a magic backpack."
                }]
            }]
        }));
    let mut es = req
        .eventsource()
        .unwrap();

    println!("Waiting for messages...");
    while let Some(event) = es.next().await {
        match event {
            Ok(Event::Open) => println!("Connection Open!"),
            Ok(Event::Message(message)) => println!("Message: {:#?}", message),
            Err(err) => {
                dbg!(&err);
                println!("Error: {}", err);
                es.close();
            }
        }
    }
    println!("Wrapping up..");
}
