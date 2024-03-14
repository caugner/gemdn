use futures_util::TryStreamExt;
use gemini::{
    GenerateContentResponse, GenerateContentResponseChunk, GenerateContentResponseError, Part,
};
use reqwest::Client;
use reqwest_streams::*;
use serde_json::{json, Value};
use std::env;

mod fixtures;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = Client::new();
    let api_key = env::var("API_KEY").expect("Usage: API_KEY=... cargo run");

    println!("Preparing request...");
    let req = client.post("https://generativelanguage.googleapis.com/v1beta/models/gemini-pro:streamGenerateContent")
        .header(reqwest::header::ACCEPT, "application/json; charset=UTF-8")
        .query(&[("key", &api_key)])
        .json(&json!({
            "contents": [{
                "parts": [{
                    "text": "Write a story about a magic backpack."
                }]
            }]
        }));

    println!("Sending request...");
    let res = req.send().await?;

    println!("Collecting chunks...");
    let stream = res.json_array_stream::<serde_json::Value>(1024 * 1024);
    let chunks: Vec<serde_json::Value> = stream.try_collect().await?;

    println!("Processing chunks...");
    for chunk in chunks.iter() {
        let chunk = parse_chunk(chunk);
        match chunk {
            Ok(chunk) => {
                let text = chunk
                    .candidates
                    .iter()
                    .filter_map(|candidate| match &candidate.content {
                        Some(content) => Some(content),
                        _ => None,
                    })
                    .flat_map(|content| {
                        content.parts.iter().map(|part| match part {
                            Part::Text(text) => Some(text.clone()),
                            _ => None,
                        })
                    })
                    .flatten()
                    .collect::<String>();
                print!("{}", text);
            }
            Err(err) => {
                println!();
                println!("Error: {:?}", err.error);
            }
        }
    }

    println!();
    println!("Wrapping up..");

    Ok(())
}

fn parse_chunks(
    data: &serde_json::Value,
) -> Result<Vec<GenerateContentResponseChunk>, GenerateContentResponseError> {
    let Value::Array(items) = data else {
        panic!("Response should be an array.")
    };

    let mut chunks: Vec<GenerateContentResponseChunk> = Vec::new();
    for item in items.iter() {
        let chunk = parse_chunk(item);
        match chunk {
            Ok(chunk) => chunks.push(chunk),
            Err(err) => return Err(err),
        }
    }

    Ok(chunks)
}

fn parse_chunk(
    item: &serde_json::Value,
) -> Result<GenerateContentResponseChunk, GenerateContentResponseError> {
    let Value::Object(_) = item else {
        panic!("Each item should be a chunk object!")
    };

    let item: GenerateContentResponse = serde_json::from_value(item.clone()).unwrap();

    match item {
        GenerateContentResponse::Chunk(chunk) => Ok(chunk),
        GenerateContentResponse::Error(err) => Err(err),
    }
}

#[tokio::test]
async fn it_should_parse_response() {
    let data: serde_json::Value = serde_json::from_str(fixtures::EXAMPLE_RESPONSE).unwrap();
    let res = parse_chunks(&data);
    assert!(res.is_ok());
}

#[tokio::test]
async fn it_should_parse_error() {
    let data: serde_json::Value = serde_json::from_str(fixtures::EXAMPLE_ERROR).unwrap();
    let res = parse_chunks(&data);
    assert!(res.is_err());
}
