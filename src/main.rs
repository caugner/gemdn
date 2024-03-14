use atty::Stream;
use futures_util::TryStreamExt;
use gemini::{
    GenerateContentResponse, GenerateContentResponseChunk, GenerateContentResponseError, Part,
};
use reqwest::Client;
use reqwest_streams::*;
use serde_json::{json, Value};
use slog::{debug, slog_o, Drain};
use std::{
    env,
    io::{self, Read},
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let logger = init_logging();

    let client = Client::new();
    let api_key = env::var("API_KEY").expect("Usage: API_KEY=... cargo run");
    let model = env::var("MODEL").unwrap_or("gemini-pro".to_string());
    let url = format!(
        "https://generativelanguage.googleapis.com/v1beta/models/{}:streamGenerateContent",
        model
    );
    let prompt = read_stdin_or("Write a story about a magic backpack.".to_string());

    debug!(logger, "Preparing request"; "model" => format!("{}", model));
    let req = client
        .post(url)
        .header(reqwest::header::ACCEPT, "application/json; charset=UTF-8")
        .query(&[("key", &api_key)])
        .json(&json!({
            "contents": [{
                "parts": [{
                    "text": prompt
                }]
            }]
        }));

    debug!(logger, "Sending request...");
    let res = req.send().await?;

    debug!(logger, "Collecting chunks...");
    let stream = res.json_array_stream::<serde_json::Value>(1024 * 1024);
    let chunks: Vec<serde_json::Value> = stream.try_collect().await?;

    debug!(logger, "Processing chunks...");
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
    debug!(logger, "Wrapping up..");

    Ok(())
}

fn init_logging() -> slog::Logger {
    let decorator = slog_term::TermDecorator::new().build();
    let drain = slog_term::FullFormat::new(decorator).build().fuse();
    let drain = slog_async::Async::new(drain).build().fuse();

    slog::Logger::root(drain, slog_o!())
}

fn read_stdin_or(default: String) -> String {
    let mut input = String::new();

    if !atty::is(Stream::Stdin) {
        io::stdin()
            .read_to_string(&mut input)
            .expect("Failed to read input");
        input
    } else {
        default
    }
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

#[cfg(test)]
mod tests {
    use super::*;

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

    #[tokio::test]
    async fn it_should_parse_response() {
        let data: serde_json::Value = serde_json::from_str(EXAMPLE_RESPONSE).unwrap();
        let res = parse_chunks(&data);
        assert!(res.is_ok());
    }

    #[tokio::test]
    async fn it_should_parse_error() {
        let data: serde_json::Value = serde_json::from_str(EXAMPLE_ERROR).unwrap();
        let res = parse_chunks(&data);
        assert!(res.is_err());
    }

    const EXAMPLE_ERROR: &str = r#"[{
        "error": {
          "code": 503,
          "message": "The model is overloaded. Please try again later.",
          "status": "UNAVAILABLE"
        }
      }
      ]"#;

    const EXAMPLE_RESPONSE: &str = r#"[{
        "candidates": [
          {
            "content": {
              "parts": [
                {
                  "text": "In the quaint, cobbled"
                }
              ],
              "role": "model"
            },
            "finishReason": "STOP",
            "index": 0,
            "safetyRatings": [
              {
                "category": "HARM_CATEGORY_SEXUALLY_EXPLICIT",
                "probability": "NEGLIGIBLE"
              },
              {
                "category": "HARM_CATEGORY_HATE_SPEECH",
                "probability": "NEGLIGIBLE"
              },
              {
                "category": "HARM_CATEGORY_HARASSMENT",
                "probability": "NEGLIGIBLE"
              },
              {
                "category": "HARM_CATEGORY_DANGEROUS_CONTENT",
                "probability": "NEGLIGIBLE"
              }
            ]
          }
        ],
        "promptFeedback": {
          "safetyRatings": [
            {
              "category": "HARM_CATEGORY_SEXUALLY_EXPLICIT",
              "probability": "NEGLIGIBLE"
            },
            {
              "category": "HARM_CATEGORY_HATE_SPEECH",
              "probability": "NEGLIGIBLE"
            },
            {
              "category": "HARM_CATEGORY_HARASSMENT",
              "probability": "NEGLIGIBLE"
            },
            {
              "category": "HARM_CATEGORY_DANGEROUS_CONTENT",
              "probability": "NEGLIGIBLE"
            }
          ]
        }
      }
      ,
      {
        "candidates": [
          {
            "content": {
              "parts": [
                {
                  "text": " streets of Willow Creek"
                }
              ],
              "role": "model"
            },
            "finishReason": "STOP",
            "index": 0,
            "safetyRatings": [
              {
                "category": "HARM_CATEGORY_SEXUALLY_EXPLICIT",
                "probability": "NEGLIGIBLE"
              },
              {
                "category": "HARM_CATEGORY_HATE_SPEECH",
                "probability": "NEGLIGIBLE"
              },
              {
                "category": "HARM_CATEGORY_HARASSMENT",
                "probability": "NEGLIGIBLE"
              },
              {
                "category": "HARM_CATEGORY_DANGEROUS_CONTENT",
                "probability": "NEGLIGIBLE"
              }
            ]
          }
        ]
      }
      ,
      {
        "candidates": [
          {
            "content": {
              "parts": [
                {
                  "text": ", nestled amidst the rolling"
                }
              ],
              "role": "model"
            },
            "finishReason": "STOP",
            "index": 0,
            "safetyRatings": [
              {
                "category": "HARM_CATEGORY_SEXUALLY_EXPLICIT",
                "probability": "NEGLIGIBLE"
              },
              {
                "category": "HARM_CATEGORY_HATE_SPEECH",
                "probability": "NEGLIGIBLE"
              },
              {
                "category": "HARM_CATEGORY_HARASSMENT",
                "probability": "NEGLIGIBLE"
              },
              {
                "category": "HARM_CATEGORY_DANGEROUS_CONTENT",
                "probability": "NEGLIGIBLE"
              }
            ]
          }
        ]
      }
      ,
      {
        "candidates": [
          {
            "content": {
              "parts": [
                {
                  "text": " hills and whispering willows, there existed"
                }
              ],
              "role": "model"
            },
            "finishReason": "STOP",
            "index": 0,
            "safetyRatings": [
              {
                "category": "HARM_CATEGORY_SEXUALLY_EXPLICIT",
                "probability": "NEGLIGIBLE"
              },
              {
                "category": "HARM_CATEGORY_HATE_SPEECH",
                "probability": "NEGLIGIBLE"
              },
              {
                "category": "HARM_CATEGORY_HARASSMENT",
                "probability": "NEGLIGIBLE"
              },
              {
                "category": "HARM_CATEGORY_DANGEROUS_CONTENT",
                "probability": "NEGLIGIBLE"
              }
            ]
          }
        ]
      }
      ,
      {
        "candidates": [
          {
            "content": {
              "parts": [
                {
                  "text": " an extraordinary tale that would forever be etched into the"
                }
              ],
              "role": "model"
            },
            "finishReason": "STOP",
            "index": 0,
            "safetyRatings": [
              {
                "category": "HARM_CATEGORY_SEXUALLY_EXPLICIT",
                "probability": "NEGLIGIBLE"
              },
              {
                "category": "HARM_CATEGORY_HATE_SPEECH",
                "probability": "NEGLIGIBLE"
              },
              {
                "category": "HARM_CATEGORY_HARASSMENT",
                "probability": "NEGLIGIBLE"
              },
              {
                "category": "HARM_CATEGORY_DANGEROUS_CONTENT",
                "probability": "NEGLIGIBLE"
              }
            ]
          }
        ]
      }
      ,
      {
        "candidates": [
          {
            "content": {
              "parts": [
                {
                  "text": " annals of history. It began with an ordinary backpack, a seemingly mundane"
                }
              ],
              "role": "model"
            },
            "finishReason": "STOP",
            "index": 0,
            "safetyRatings": [
              {
                "category": "HARM_CATEGORY_SEXUALLY_EXPLICIT",
                "probability": "NEGLIGIBLE"
              },
              {
                "category": "HARM_CATEGORY_HATE_SPEECH",
                "probability": "NEGLIGIBLE"
              },
              {
                "category": "HARM_CATEGORY_HARASSMENT",
                "probability": "NEGLIGIBLE"
              },
              {
                "category": "HARM_CATEGORY_DANGEROUS_CONTENT",
                "probability": "NEGLIGIBLE"
              }
            ]
          }
        ]
      }
      ,
      {
        "candidates": [
          {
            "content": {
              "parts": [
                {
                  "text": " object destined for a life of textbooks and forgotten lunches"
                }
              ],
              "role": "model"
            },
            "finishReason": "STOP",
            "index": 0,
            "safetyRatings": [
              {
                "category": "HARM_CATEGORY_SEXUALLY_EXPLICIT",
                "probability": "NEGLIGIBLE"
              },
              {
                "category": "HARM_CATEGORY_HATE_SPEECH",
                "probability": "NEGLIGIBLE"
              },
              {
                "category": "HARM_CATEGORY_HARASSMENT",
                "probability": "NEGLIGIBLE"
              },
              {
                "category": "HARM_CATEGORY_DANGEROUS_CONTENT",
                "probability": "NEGLIGIBLE"
              }
            ]
          }
        ]
      }
      ,
      {
        "candidates": [
          {
            "content": {
              "parts": [
                {
                  "text": ". However, as fate would have it,"
                }
              ],
              "role": "model"
            },
            "finishReason": "STOP",
            "index": 0,
            "safetyRatings": [
              {
                "category": "HARM_CATEGORY_SEXUALLY_EXPLICIT",
                "probability": "NEGLIGIBLE"
              },
              {
                "category": "HARM_CATEGORY_HATE_SPEECH",
                "probability": "NEGLIGIBLE"
              },
              {
                "category": "HARM_CATEGORY_HARASSMENT",
                "probability": "NEGLIGIBLE"
              },
              {
                "category": "HARM_CATEGORY_DANGEROUS_CONTENT",
                "probability": "NEGLIGIBLE"
              }
            ]
          }
        ]
      }
      ,
      {
        "candidates": [
          {
            "content": {
              "parts": [
                {
                  "text": " this backpack held a secret that would change"
                }
              ],
              "role": "model"
            },
            "finishReason": "STOP",
            "index": 0,
            "safetyRatings": [
              {
                "category": "HARM_CATEGORY_SEXUALLY_EXPLICIT",
                "probability": "NEGLIGIBLE"
              },
              {
                "category": "HARM_CATEGORY_HATE_SPEECH",
                "probability": "NEGLIGIBLE"
              },
              {
                "category": "HARM_CATEGORY_HARASSMENT",
                "probability": "NEGLIGIBLE"
              },
              {
                "category": "HARM_CATEGORY_DANGEROUS_CONTENT",
                "probability": "NEGLIGIBLE"
              }
            ]
          }
        ]
      }
      ,
      {
        "candidates": [
          {
            "content": {
              "parts": [
                {
                  "text": " the destiny of its young owner,"
                }
              ],
              "role": "model"
            },
            "finishReason": "STOP",
            "index": 0,
            "safetyRatings": [
              {
                "category": "HARM_CATEGORY_SEXUALLY_EXPLICIT",
                "probability": "NEGLIGIBLE"
              },
              {
                "category": "HARM_CATEGORY_HATE_SPEECH",
                "probability": "NEGLIGIBLE"
              },
              {
                "category": "HARM_CATEGORY_HARASSMENT",
                "probability": "NEGLIGIBLE"
              },
              {
                "category": "HARM_CATEGORY_DANGEROUS_CONTENT",
                "probability": "NEGLIGIBLE"
              }
            ]
          }
        ]
      }
      ,
      {
        "candidates": [
          {
            "content": {
              "parts": [
                {
                  "text": " Emily Carter.\n\nEmily, a"
                }
              ],
              "role": "model"
            },
            "finishReason": "STOP",
            "index": 0,
            "safetyRatings": [
              {
                "category": "HARM_CATEGORY_SEXUALLY_EXPLICIT",
                "probability": "NEGLIGIBLE"
              },
              {
                "category": "HARM_CATEGORY_HATE_SPEECH",
                "probability": "NEGLIGIBLE"
              },
              {
                "category": "HARM_CATEGORY_HARASSMENT",
                "probability": "NEGLIGIBLE"
              },
              {
                "category": "HARM_CATEGORY_DANGEROUS_CONTENT",
                "probability": "NEGLIGIBLE"
              }
            ]
          }
        ]
      }
      ,
      {
        "candidates": [
          {
            "content": {
              "parts": [
                {
                  "text": " curious and imaginative girl of twelve"
                }
              ],
              "role": "model"
            },
            "finishReason": "STOP",
            "index": 0,
            "safetyRatings": [
              {
                "category": "HARM_CATEGORY_SEXUALLY_EXPLICIT",
                "probability": "NEGLIGIBLE"
              },
              {
                "category": "HARM_CATEGORY_HATE_SPEECH",
                "probability": "NEGLIGIBLE"
              },
              {
                "category": "HARM_CATEGORY_HARASSMENT",
                "probability": "NEGLIGIBLE"
              },
              {
                "category": "HARM_CATEGORY_DANGEROUS_CONTENT",
                "probability": "NEGLIGIBLE"
              }
            ]
          }
        ]
      }
      ,
      {
        "candidates": [
          {
            "content": {
              "parts": [
                {
                  "text": ", stumbled upon the backpack in"
                }
              ],
              "role": "model"
            },
            "finishReason": "STOP",
            "index": 0,
            "safetyRatings": [
              {
                "category": "HARM_CATEGORY_SEXUALLY_EXPLICIT",
                "probability": "NEGLIGIBLE"
              },
              {
                "category": "HARM_CATEGORY_HATE_SPEECH",
                "probability": "NEGLIGIBLE"
              },
              {
                "category": "HARM_CATEGORY_HARASSMENT",
                "probability": "NEGLIGIBLE"
              },
              {
                "category": "HARM_CATEGORY_DANGEROUS_CONTENT",
                "probability": "NEGLIGIBLE"
              }
            ]
          }
        ]
      }
      ,
      {
        "candidates": [
          {
            "content": {
              "parts": [
                {
                  "text": " her grandmother's attic."
                }
              ],
              "role": "model"
            },
            "finishReason": "STOP",
            "index": 0,
            "safetyRatings": [
              {
                "category": "HARM_CATEGORY_SEXUALLY_EXPLICIT",
                "probability": "NEGLIGIBLE"
              },
              {
                "category": "HARM_CATEGORY_HATE_SPEECH",
                "probability": "NEGLIGIBLE"
              },
              {
                "category": "HARM_CATEGORY_HARASSMENT",
                "probability": "NEGLIGIBLE"
              },
              {
                "category": "HARM_CATEGORY_DANGEROUS_CONTENT",
                "probability": "NEGLIGIBLE"
              }
            ]
          }
        ]
      }
      ,
      {
        "candidates": [
          {
            "content": {
              "parts": [
                {
                  "text": " Its faded leather and worn"
                }
              ],
              "role": "model"
            },
            "finishReason": "STOP",
            "index": 0,
            "safetyRatings": [
              {
                "category": "HARM_CATEGORY_SEXUALLY_EXPLICIT",
                "probability": "NEGLIGIBLE"
              },
              {
                "category": "HARM_CATEGORY_HATE_SPEECH",
                "probability": "NEGLIGIBLE"
              },
              {
                "category": "HARM_CATEGORY_HARASSMENT",
                "probability": "NEGLIGIBLE"
              },
              {
                "category": "HARM_CATEGORY_DANGEROUS_CONTENT",
                "probability": "NEGLIGIBLE"
              }
            ]
          }
        ]
      }
      ,
      {
        "candidates": [
          {
            "content": {
              "parts": [
                {
                  "text": " straps hinted at a life"
                }
              ],
              "role": "model"
            },
            "finishReason": "STOP",
            "index": 0,
            "safetyRatings": [
              {
                "category": "HARM_CATEGORY_SEXUALLY_EXPLICIT",
                "probability": "NEGLIGIBLE"
              },
              {
                "category": "HARM_CATEGORY_HATE_SPEECH",
                "probability": "NEGLIGIBLE"
              },
              {
                "category": "HARM_CATEGORY_HARASSMENT",
                "probability": "NEGLIGIBLE"
              },
              {
                "category": "HARM_CATEGORY_DANGEROUS_CONTENT",
                "probability": "NEGLIGIBLE"
              }
            ]
          }
        ]
      }
      ,
      {
        "candidates": [
          {
            "content": {
              "parts": [
                {
                  "text": " well-traveled,"
                }
              ],
              "role": "model"
            },
            "finishReason": "STOP",
            "index": 0,
            "safetyRatings": [
              {
                "category": "HARM_CATEGORY_SEXUALLY_EXPLICIT",
                "probability": "NEGLIGIBLE"
              },
              {
                "category": "HARM_CATEGORY_HATE_SPEECH",
                "probability": "NEGLIGIBLE"
              },
              {
                "category": "HARM_CATEGORY_HARASSMENT",
                "probability": "NEGLIGIBLE"
              },
              {
                "category": "HARM_CATEGORY_DANGEROUS_CONTENT",
                "probability": "NEGLIGIBLE"
              }
            ]
          }
        ]
      }
      ,
      {
        "candidates": [
          {
            "content": {
              "parts": [
                {
                  "text": " but its true nature remained"
                }
              ],
              "role": "model"
            },
            "finishReason": "STOP",
            "index": 0,
            "safetyRatings": [
              {
                "category": "HARM_CATEGORY_SEXUALLY_EXPLICIT",
                "probability": "NEGLIGIBLE"
              },
              {
                "category": "HARM_CATEGORY_HATE_SPEECH",
                "probability": "NEGLIGIBLE"
              },
              {
                "category": "HARM_CATEGORY_HARASSMENT",
                "probability": "NEGLIGIBLE"
              },
              {
                "category": "HARM_CATEGORY_DANGEROUS_CONTENT",
                "probability": "NEGLIGIBLE"
              }
            ]
          }
        ]
      }
      ,
      {
        "candidates": [
          {
            "content": {
              "parts": [
                {
                  "text": " veiled. As she flipped"
                }
              ],
              "role": "model"
            },
            "finishReason": "STOP",
            "index": 0,
            "safetyRatings": [
              {
                "category": "HARM_CATEGORY_SEXUALLY_EXPLICIT",
                "probability": "NEGLIGIBLE"
              },
              {
                "category": "HARM_CATEGORY_HATE_SPEECH",
                "probability": "NEGLIGIBLE"
              },
              {
                "category": "HARM_CATEGORY_HARASSMENT",
                "probability": "NEGLIGIBLE"
              },
              {
                "category": "HARM_CATEGORY_DANGEROUS_CONTENT",
                "probability": "NEGLIGIBLE"
              }
            ]
          }
        ]
      }
      ,
      {
        "candidates": [
          {
            "content": {
              "parts": [
                {
                  "text": " through the dusty pages"
                }
              ],
              "role": "model"
            },
            "finishReason": "STOP",
            "index": 0,
            "safetyRatings": [
              {
                "category": "HARM_CATEGORY_SEXUALLY_EXPLICIT",
                "probability": "NEGLIGIBLE"
              },
              {
                "category": "HARM_CATEGORY_HATE_SPEECH",
                "probability": "NEGLIGIBLE"
              },
              {
                "category": "HARM_CATEGORY_HARASSMENT",
                "probability": "NEGLIGIBLE"
              },
              {
                "category": "HARM_CATEGORY_DANGEROUS_CONTENT",
                "probability": "NEGLIGIBLE"
              }
            ]
          }
        ]
      }
      ,
      {
        "candidates": [
          {
            "content": {
              "parts": [
                {
                  "text": " of her grandmother'"
                }
              ],
              "role": "model"
            },
            "finishReason": "STOP",
            "index": 0,
            "safetyRatings": [
              {
                "category": "HARM_CATEGORY_SEXUALLY_EXPLICIT",
                "probability": "NEGLIGIBLE"
              },
              {
                "category": "HARM_CATEGORY_HATE_SPEECH",
                "probability": "NEGLIGIBLE"
              },
              {
                "category": "HARM_CATEGORY_HARASSMENT",
                "probability": "NEGLIGIBLE"
              },
              {
                "category": "HARM_CATEGORY_DANGEROUS_CONTENT",
                "probability": "NEGLIGIBLE"
              }
            ]
          }
        ]
      }
      ,
      {
        "candidates": [
          {
            "content": {
              "parts": [
                {
                  "text": "s diary, Emily"
                }
              ],
              "role": "model"
            },
            "finishReason": "STOP",
            "index": 0,
            "safetyRatings": [
              {
                "category": "HARM_CATEGORY_SEXUALLY_EXPLICIT",
                "probability": "NEGLIGIBLE"
              },
              {
                "category": "HARM_CATEGORY_HATE_SPEECH",
                "probability": "NEGLIGIBLE"
              },
              {
                "category": "HARM_CATEGORY_HARASSMENT",
                "probability": "NEGLIGIBLE"
              },
              {
                "category": "HARM_CATEGORY_DANGEROUS_CONTENT",
                "probability": "NEGLIGIBLE"
              }
            ]
          }
        ]
      }
      ,
      {
        "candidates": [
          {
            "content": {
              "parts": [
                {
                  "text": "'s eyes widened"
                }
              ],
              "role": "model"
            },
            "finishReason": "STOP",
            "index": 0,
            "safetyRatings": [
              {
                "category": "HARM_CATEGORY_SEXUALLY_EXPLICIT",
                "probability": "NEGLIGIBLE"
              },
              {
                "category": "HARM_CATEGORY_HATE_SPEECH",
                "probability": "NEGLIGIBLE"
              },
              {
                "category": "HARM_CATEGORY_HARASSMENT",
                "probability": "NEGLIGIBLE"
              },
              {
                "category": "HARM_CATEGORY_DANGEROUS_CONTENT",
                "probability": "NEGLIGIBLE"
              }
            ]
          }
        ]
      }
      ,
      {
        "candidates": [
          {
            "content": {
              "parts": [
                {
                  "text": " in amazement. There"
                }
              ],
              "role": "model"
            },
            "finishReason": "STOP",
            "index": 0,
            "safetyRatings": [
              {
                "category": "HARM_CATEGORY_SEXUALLY_EXPLICIT",
                "probability": "NEGLIGIBLE"
              },
              {
                "category": "HARM_CATEGORY_HATE_SPEECH",
                "probability": "NEGLIGIBLE"
              },
              {
                "category": "HARM_CATEGORY_HARASSMENT",
                "probability": "NEGLIGIBLE"
              },
              {
                "category": "HARM_CATEGORY_DANGEROUS_CONTENT",
                "probability": "NEGLIGIBLE"
              }
            ]
          }
        ]
      }
      ,
      {
        "candidates": [
          {
            "content": {
              "parts": [
                {
                  "text": ", in intricate script"
                }
              ],
              "role": "model"
            },
            "finishReason": "STOP",
            "index": 0,
            "safetyRatings": [
              {
                "category": "HARM_CATEGORY_SEXUALLY_EXPLICIT",
                "probability": "NEGLIGIBLE"
              },
              {
                "category": "HARM_CATEGORY_HATE_SPEECH",
                "probability": "NEGLIGIBLE"
              },
              {
                "category": "HARM_CATEGORY_HARASSMENT",
                "probability": "NEGLIGIBLE"
              },
              {
                "category": "HARM_CATEGORY_DANGEROUS_CONTENT",
                "probability": "NEGLIGIBLE"
              }
            ]
          }
        ]
      }
      ,
      {
        "candidates": [
          {
            "content": {
              "parts": [
                {
                  "text": ", was a captivating"
                }
              ],
              "role": "model"
            },
            "finishReason": "STOP",
            "index": 0,
            "safetyRatings": [
              {
                "category": "HARM_CATEGORY_SEXUALLY_EXPLICIT",
                "probability": "NEGLIGIBLE"
              },
              {
                "category": "HARM_CATEGORY_HATE_SPEECH",
                "probability": "NEGLIGIBLE"
              },
              {
                "category": "HARM_CATEGORY_HARASSMENT",
                "probability": "NEGLIGIBLE"
              },
              {
                "category": "HARM_CATEGORY_DANGEROUS_CONTENT",
                "probability": "NEGLIGIBLE"
              }
            ]
          }
        ]
      }
      ,
      {
        "candidates": [
          {
            "content": {
              "parts": [
                {
                  "text": " account of the backpack"
                }
              ],
              "role": "model"
            },
            "finishReason": "STOP",
            "index": 0,
            "safetyRatings": [
              {
                "category": "HARM_CATEGORY_SEXUALLY_EXPLICIT",
                "probability": "NEGLIGIBLE"
              },
              {
                "category": "HARM_CATEGORY_HATE_SPEECH",
                "probability": "NEGLIGIBLE"
              },
              {
                "category": "HARM_CATEGORY_HARASSMENT",
                "probability": "NEGLIGIBLE"
              },
              {
                "category": "HARM_CATEGORY_DANGEROUS_CONTENT",
                "probability": "NEGLIGIBLE"
              }
            ]
          }
        ]
      }
      ,
      {
        "candidates": [
          {
            "content": {
              "parts": [
                {
                  "text": "'s origins and"
                }
              ],
              "role": "model"
            },
            "finishReason": "STOP",
            "index": 0,
            "safetyRatings": [
              {
                "category": "HARM_CATEGORY_SEXUALLY_EXPLICIT",
                "probability": "NEGLIGIBLE"
              },
              {
                "category": "HARM_CATEGORY_HATE_SPEECH",
                "probability": "NEGLIGIBLE"
              },
              {
                "category": "HARM_CATEGORY_HARASSMENT",
                "probability": "NEGLIGIBLE"
              },
              {
                "category": "HARM_CATEGORY_DANGEROUS_CONTENT",
                "probability": "NEGLIGIBLE"
              }
            ]
          }
        ]
      }
      ,
      {
        "candidates": [
          {
            "content": {
              "parts": [
                {
                  "text": " its extraordinary powers."
                }
              ],
              "role": "model"
            },
            "finishReason": "STOP",
            "index": 0,
            "safetyRatings": [
              {
                "category": "HARM_CATEGORY_SEXUALLY_EXPLICIT",
                "probability": "NEGLIGIBLE"
              },
              {
                "category": "HARM_CATEGORY_HATE_SPEECH",
                "probability": "NEGLIGIBLE"
              },
              {
                "category": "HARM_CATEGORY_HARASSMENT",
                "probability": "NEGLIGIBLE"
              },
              {
                "category": "HARM_CATEGORY_DANGEROUS_CONTENT",
                "probability": "NEGLIGIBLE"
              }
            ]
          }
        ]
      }
      ,
      {
        "candidates": [
          {
            "content": {
              "parts": [
                {
                  "text": "\n\nLegend had"
                }
              ],
              "role": "model"
            },
            "finishReason": "STOP",
            "index": 0,
            "safetyRatings": [
              {
                "category": "HARM_CATEGORY_SEXUALLY_EXPLICIT",
                "probability": "NEGLIGIBLE"
              },
              {
                "category": "HARM_CATEGORY_HATE_SPEECH",
                "probability": "NEGLIGIBLE"
              },
              {
                "category": "HARM_CATEGORY_HARASSMENT",
                "probability": "NEGLIGIBLE"
              },
              {
                "category": "HARM_CATEGORY_DANGEROUS_CONTENT",
                "probability": "NEGLIGIBLE"
              }
            ]
          }
        ]
      }
      ,
      {
        "candidates": [
          {
            "content": {
              "parts": [
                {
                  "text": " it that the"
                }
              ],
              "role": "model"
            },
            "finishReason": "STOP",
            "index": 0,
            "safetyRatings": [
              {
                "category": "HARM_CATEGORY_SEXUALLY_EXPLICIT",
                "probability": "NEGLIGIBLE"
              },
              {
                "category": "HARM_CATEGORY_HATE_SPEECH",
                "probability": "NEGLIGIBLE"
              },
              {
                "category": "HARM_CATEGORY_HARASSMENT",
                "probability": "NEGLIGIBLE"
              },
              {
                "category": "HARM_CATEGORY_DANGEROUS_CONTENT",
                "probability": "NEGLIGIBLE"
              }
            ]
          }
        ]
      }
      ,
      {
        "candidates": [
          {
            "content": {
              "parts": [
                {
                  "text": " backpack was crafted"
                }
              ],
              "role": "model"
            },
            "finishReason": "STOP",
            "index": 0,
            "safetyRatings": [
              {
                "category": "HARM_CATEGORY_SEXUALLY_EXPLICIT",
                "probability": "NEGLIGIBLE"
              },
              {
                "category": "HARM_CATEGORY_HATE_SPEECH",
                "probability": "NEGLIGIBLE"
              },
              {
                "category": "HARM_CATEGORY_HARASSMENT",
                "probability": "NEGLIGIBLE"
              },
              {
                "category": "HARM_CATEGORY_DANGEROUS_CONTENT",
                "probability": "NEGLIGIBLE"
              }
            ]
          }
        ]
      }
      ,
      {
        "candidates": [
          {
            "content": {
              "parts": [
                {
                  "text": " by an ancient"
                }
              ],
              "role": "model"
            },
            "finishReason": "STOP",
            "index": 0,
            "safetyRatings": [
              {
                "category": "HARM_CATEGORY_SEXUALLY_EXPLICIT",
                "probability": "NEGLIGIBLE"
              },
              {
                "category": "HARM_CATEGORY_HATE_SPEECH",
                "probability": "NEGLIGIBLE"
              },
              {
                "category": "HARM_CATEGORY_HARASSMENT",
                "probability": "NEGLIGIBLE"
              },
              {
                "category": "HARM_CATEGORY_DANGEROUS_CONTENT",
                "probability": "NEGLIGIBLE"
              }
            ]
          }
        ]
      }
      ,
      {
        "candidates": [
          {
            "content": {
              "parts": [
                {
                  "text": " sorcerer who imbued"
                }
              ],
              "role": "model"
            },
            "finishReason": "STOP",
            "index": 0,
            "safetyRatings": [
              {
                "category": "HARM_CATEGORY_SEXUALLY_EXPLICIT",
                "probability": "NEGLIGIBLE"
              },
              {
                "category": "HARM_CATEGORY_HATE_SPEECH",
                "probability": "NEGLIGIBLE"
              },
              {
                "category": "HARM_CATEGORY_HARASSMENT",
                "probability": "NEGLIGIBLE"
              },
              {
                "category": "HARM_CATEGORY_DANGEROUS_CONTENT",
                "probability": "NEGLIGIBLE"
              }
            ]
          }
        ]
      }
      ,
      {
        "candidates": [
          {
            "content": {
              "parts": [
                {
                  "text": " it with the"
                }
              ],
              "role": "model"
            },
            "finishReason": "STOP",
            "index": 0,
            "safetyRatings": [
              {
                "category": "HARM_CATEGORY_SEXUALLY_EXPLICIT",
                "probability": "NEGLIGIBLE"
              },
              {
                "category": "HARM_CATEGORY_HATE_SPEECH",
                "probability": "NEGLIGIBLE"
              },
              {
                "category": "HARM_CATEGORY_HARASSMENT",
                "probability": "NEGLIGIBLE"
              },
              {
                "category": "HARM_CATEGORY_DANGEROUS_CONTENT",
                "probability": "NEGLIGIBLE"
              }
            ]
          }
        ]
      }
      ,
      {
        "candidates": [
          {
            "content": {
              "parts": [
                {
                  "text": " ability to transport"
                }
              ],
              "role": "model"
            },
            "finishReason": "STOP",
            "index": 0,
            "safetyRatings": [
              {
                "category": "HARM_CATEGORY_SEXUALLY_EXPLICIT",
                "probability": "NEGLIGIBLE"
              },
              {
                "category": "HARM_CATEGORY_HATE_SPEECH",
                "probability": "NEGLIGIBLE"
              },
              {
                "category": "HARM_CATEGORY_HARASSMENT",
                "probability": "NEGLIGIBLE"
              },
              {
                "category": "HARM_CATEGORY_DANGEROUS_CONTENT",
                "probability": "NEGLIGIBLE"
              }
            ]
          }
        ]
      }
      ,
      {
        "candidates": [
          {
            "content": {
              "parts": [
                {
                  "text": " its wearer to"
                }
              ],
              "role": "model"
            },
            "finishReason": "STOP",
            "index": 0,
            "safetyRatings": [
              {
                "category": "HARM_CATEGORY_SEXUALLY_EXPLICIT",
                "probability": "NEGLIGIBLE"
              },
              {
                "category": "HARM_CATEGORY_HATE_SPEECH",
                "probability": "NEGLIGIBLE"
              },
              {
                "category": "HARM_CATEGORY_HARASSMENT",
                "probability": "NEGLIGIBLE"
              },
              {
                "category": "HARM_CATEGORY_DANGEROUS_CONTENT",
                "probability": "NEGLIGIBLE"
              }
            ]
          }
        ]
      }
      ,
      {
        "candidates": [
          {
            "content": {
              "parts": [
                {
                  "text": " distant realms."
                }
              ],
              "role": "model"
            },
            "finishReason": "STOP",
            "index": 0,
            "safetyRatings": [
              {
                "category": "HARM_CATEGORY_SEXUALLY_EXPLICIT",
                "probability": "NEGLIGIBLE"
              },
              {
                "category": "HARM_CATEGORY_HATE_SPEECH",
                "probability": "NEGLIGIBLE"
              },
              {
                "category": "HARM_CATEGORY_HARASSMENT",
                "probability": "NEGLIGIBLE"
              },
              {
                "category": "HARM_CATEGORY_DANGEROUS_CONTENT",
                "probability": "NEGLIGIBLE"
              }
            ]
          }
        ]
      }
      ,
      {
        "candidates": [
          {
            "content": {
              "parts": [
                {
                  "text": " Each compartment,"
                }
              ],
              "role": "model"
            },
            "finishReason": "STOP",
            "index": 0,
            "safetyRatings": [
              {
                "category": "HARM_CATEGORY_SEXUALLY_EXPLICIT",
                "probability": "NEGLIGIBLE"
              },
              {
                "category": "HARM_CATEGORY_HATE_SPEECH",
                "probability": "NEGLIGIBLE"
              },
              {
                "category": "HARM_CATEGORY_HARASSMENT",
                "probability": "NEGLIGIBLE"
              },
              {
                "category": "HARM_CATEGORY_DANGEROUS_CONTENT",
                "probability": "NEGLIGIBLE"
              }
            ]
          }
        ]
      }
      ,
      {
        "candidates": [
          {
            "content": {
              "parts": [
                {
                  "text": " the diary revealed"
                }
              ],
              "role": "model"
            },
            "finishReason": "STOP",
            "index": 0,
            "safetyRatings": [
              {
                "category": "HARM_CATEGORY_SEXUALLY_EXPLICIT",
                "probability": "NEGLIGIBLE"
              },
              {
                "category": "HARM_CATEGORY_HATE_SPEECH",
                "probability": "NEGLIGIBLE"
              },
              {
                "category": "HARM_CATEGORY_HARASSMENT",
                "probability": "NEGLIGIBLE"
              },
              {
                "category": "HARM_CATEGORY_DANGEROUS_CONTENT",
                "probability": "NEGLIGIBLE"
              }
            ]
          }
        ]
      }
      ,
      {
        "candidates": [
          {
            "content": {
              "parts": [
                {
                  "text": ", possessed a"
                }
              ],
              "role": "model"
            },
            "finishReason": "STOP",
            "index": 0,
            "safetyRatings": [
              {
                "category": "HARM_CATEGORY_SEXUALLY_EXPLICIT",
                "probability": "NEGLIGIBLE"
              },
              {
                "category": "HARM_CATEGORY_HATE_SPEECH",
                "probability": "NEGLIGIBLE"
              },
              {
                "category": "HARM_CATEGORY_HARASSMENT",
                "probability": "NEGLIGIBLE"
              },
              {
                "category": "HARM_CATEGORY_DANGEROUS_CONTENT",
                "probability": "NEGLIGIBLE"
              }
            ]
          }
        ]
      }
      ,
      {
        "candidates": [
          {
            "content": {
              "parts": [
                {
                  "text": " unique enchantment."
                }
              ],
              "role": "model"
            },
            "finishReason": "STOP",
            "index": 0,
            "safetyRatings": [
              {
                "category": "HARM_CATEGORY_SEXUALLY_EXPLICIT",
                "probability": "NEGLIGIBLE"
              },
              {
                "category": "HARM_CATEGORY_HATE_SPEECH",
                "probability": "NEGLIGIBLE"
              },
              {
                "category": "HARM_CATEGORY_HARASSMENT",
                "probability": "NEGLIGIBLE"
              },
              {
                "category": "HARM_CATEGORY_DANGEROUS_CONTENT",
                "probability": "NEGLIGIBLE"
              }
            ]
          }
        ]
      }
      ,
      {
        "candidates": [
          {
            "content": {
              "parts": [
                {
                  "text": " The main compartment"
                }
              ],
              "role": "model"
            },
            "finishReason": "STOP",
            "index": 0,
            "safetyRatings": [
              {
                "category": "HARM_CATEGORY_SEXUALLY_EXPLICIT",
                "probability": "NEGLIGIBLE"
              },
              {
                "category": "HARM_CATEGORY_HATE_SPEECH",
                "probability": "NEGLIGIBLE"
              },
              {
                "category": "HARM_CATEGORY_HARASSMENT",
                "probability": "NEGLIGIBLE"
              },
              {
                "category": "HARM_CATEGORY_DANGEROUS_CONTENT",
                "probability": "NEGLIGIBLE"
              }
            ]
          }
        ]
      }
      ,
      {
        "candidates": [
          {
            "content": {
              "parts": [
                {
                  "text": " allowed one to"
                }
              ],
              "role": "model"
            },
            "finishReason": "STOP",
            "index": 0,
            "safetyRatings": [
              {
                "category": "HARM_CATEGORY_SEXUALLY_EXPLICIT",
                "probability": "NEGLIGIBLE"
              },
              {
                "category": "HARM_CATEGORY_HATE_SPEECH",
                "probability": "NEGLIGIBLE"
              },
              {
                "category": "HARM_CATEGORY_HARASSMENT",
                "probability": "NEGLIGIBLE"
              },
              {
                "category": "HARM_CATEGORY_DANGEROUS_CONTENT",
                "probability": "NEGLIGIBLE"
              }
            ]
          }
        ]
      }
      ,
      {
        "candidates": [
          {
            "content": {
              "parts": [
                {
                  "text": " travel through time"
                }
              ],
              "role": "model"
            },
            "finishReason": "STOP",
            "index": 0,
            "safetyRatings": [
              {
                "category": "HARM_CATEGORY_SEXUALLY_EXPLICIT",
                "probability": "NEGLIGIBLE"
              },
              {
                "category": "HARM_CATEGORY_HATE_SPEECH",
                "probability": "NEGLIGIBLE"
              },
              {
                "category": "HARM_CATEGORY_HARASSMENT",
                "probability": "NEGLIGIBLE"
              },
              {
                "category": "HARM_CATEGORY_DANGEROUS_CONTENT",
                "probability": "NEGLIGIBLE"
              }
            ]
          }
        ]
      }
      ,
      {
        "candidates": [
          {
            "content": {
              "parts": [
                {
                  "text": ", while the"
                }
              ],
              "role": "model"
            },
            "finishReason": "STOP",
            "index": 0,
            "safetyRatings": [
              {
                "category": "HARM_CATEGORY_SEXUALLY_EXPLICIT",
                "probability": "NEGLIGIBLE"
              },
              {
                "category": "HARM_CATEGORY_HATE_SPEECH",
                "probability": "NEGLIGIBLE"
              },
              {
                "category": "HARM_CATEGORY_HARASSMENT",
                "probability": "NEGLIGIBLE"
              },
              {
                "category": "HARM_CATEGORY_DANGEROUS_CONTENT",
                "probability": "NEGLIGIBLE"
              }
            ]
          }
        ]
      }
      ,
      {
        "candidates": [
          {
            "content": {
              "parts": [
                {
                  "text": " smaller pockets granted"
                }
              ],
              "role": "model"
            },
            "finishReason": "STOP",
            "index": 0,
            "safetyRatings": [
              {
                "category": "HARM_CATEGORY_SEXUALLY_EXPLICIT",
                "probability": "NEGLIGIBLE"
              },
              {
                "category": "HARM_CATEGORY_HATE_SPEECH",
                "probability": "NEGLIGIBLE"
              },
              {
                "category": "HARM_CATEGORY_HARASSMENT",
                "probability": "NEGLIGIBLE"
              },
              {
                "category": "HARM_CATEGORY_DANGEROUS_CONTENT",
                "probability": "NEGLIGIBLE"
              }
            ]
          }
        ]
      }
      ,
      {
        "candidates": [
          {
            "content": {
              "parts": [
                {
                  "text": " access to parallel"
                }
              ],
              "role": "model"
            },
            "finishReason": "STOP",
            "index": 0,
            "safetyRatings": [
              {
                "category": "HARM_CATEGORY_SEXUALLY_EXPLICIT",
                "probability": "NEGLIGIBLE"
              },
              {
                "category": "HARM_CATEGORY_HATE_SPEECH",
                "probability": "NEGLIGIBLE"
              },
              {
                "category": "HARM_CATEGORY_HARASSMENT",
                "probability": "NEGLIGIBLE"
              },
              {
                "category": "HARM_CATEGORY_DANGEROUS_CONTENT",
                "probability": "NEGLIGIBLE"
              }
            ]
          }
        ]
      }
      ,
      {
        "candidates": [
          {
            "content": {
              "parts": [
                {
                  "text": " universes, each"
                }
              ],
              "role": "model"
            },
            "finishReason": "STOP",
            "index": 0,
            "safetyRatings": [
              {
                "category": "HARM_CATEGORY_SEXUALLY_EXPLICIT",
                "probability": "NEGLIGIBLE"
              },
              {
                "category": "HARM_CATEGORY_HATE_SPEECH",
                "probability": "NEGLIGIBLE"
              },
              {
                "category": "HARM_CATEGORY_HARASSMENT",
                "probability": "NEGLIGIBLE"
              },
              {
                "category": "HARM_CATEGORY_DANGEROUS_CONTENT",
                "probability": "NEGLIGIBLE"
              }
            ]
          }
        ]
      }
      ,
      {
        "candidates": [
          {
            "content": {
              "parts": [
                {
                  "text": " with its own"
                }
              ],
              "role": "model"
            },
            "finishReason": "STOP",
            "index": 0,
            "safetyRatings": [
              {
                "category": "HARM_CATEGORY_SEXUALLY_EXPLICIT",
                "probability": "NEGLIGIBLE"
              },
              {
                "category": "HARM_CATEGORY_HATE_SPEECH",
                "probability": "NEGLIGIBLE"
              },
              {
                "category": "HARM_CATEGORY_HARASSMENT",
                "probability": "NEGLIGIBLE"
              },
              {
                "category": "HARM_CATEGORY_DANGEROUS_CONTENT",
                "probability": "NEGLIGIBLE"
              }
            ]
          }
        ]
      }
      ,
      {
        "candidates": [
          {
            "content": {
              "parts": [
                {
                  "text": " distinct wonders and"
                }
              ],
              "role": "model"
            },
            "finishReason": "STOP",
            "index": 0,
            "safetyRatings": [
              {
                "category": "HARM_CATEGORY_SEXUALLY_EXPLICIT",
                "probability": "NEGLIGIBLE"
              },
              {
                "category": "HARM_CATEGORY_HATE_SPEECH",
                "probability": "NEGLIGIBLE"
              },
              {
                "category": "HARM_CATEGORY_HARASSMENT",
                "probability": "NEGLIGIBLE"
              },
              {
                "category": "HARM_CATEGORY_DANGEROUS_CONTENT",
                "probability": "NEGLIGIBLE"
              }
            ]
          }
        ]
      }
      ,
      {
        "candidates": [
          {
            "content": {
              "parts": [
                {
                  "text": " challenges.\n\n"
                }
              ],
              "role": "model"
            },
            "finishReason": "STOP",
            "index": 0,
            "safetyRatings": [
              {
                "category": "HARM_CATEGORY_SEXUALLY_EXPLICIT",
                "probability": "NEGLIGIBLE"
              },
              {
                "category": "HARM_CATEGORY_HATE_SPEECH",
                "probability": "NEGLIGIBLE"
              },
              {
                "category": "HARM_CATEGORY_HARASSMENT",
                "probability": "NEGLIGIBLE"
              },
              {
                "category": "HARM_CATEGORY_DANGEROUS_CONTENT",
                "probability": "NEGLIGIBLE"
              }
            ]
          }
        ]
      }
      ,
      {
        "candidates": [
          {
            "content": {
              "parts": [
                {
                  "text": "Emily's"
                }
              ],
              "role": "model"
            },
            "finishReason": "STOP",
            "index": 0,
            "safetyRatings": [
              {
                "category": "HARM_CATEGORY_SEXUALLY_EXPLICIT",
                "probability": "NEGLIGIBLE"
              },
              {
                "category": "HARM_CATEGORY_HATE_SPEECH",
                "probability": "NEGLIGIBLE"
              },
              {
                "category": "HARM_CATEGORY_HARASSMENT",
                "probability": "NEGLIGIBLE"
              },
              {
                "category": "HARM_CATEGORY_DANGEROUS_CONTENT",
                "probability": "NEGLIGIBLE"
              }
            ]
          }
        ]
      }
      ,
      {
        "candidates": [
          {
            "content": {
              "parts": [
                {
                  "text": " heart pounded with"
                }
              ],
              "role": "model"
            },
            "finishReason": "STOP",
            "index": 0,
            "safetyRatings": [
              {
                "category": "HARM_CATEGORY_SEXUALLY_EXPLICIT",
                "probability": "LOW"
              },
              {
                "category": "HARM_CATEGORY_HATE_SPEECH",
                "probability": "NEGLIGIBLE"
              },
              {
                "category": "HARM_CATEGORY_HARASSMENT",
                "probability": "NEGLIGIBLE"
              },
              {
                "category": "HARM_CATEGORY_DANGEROUS_CONTENT",
                "probability": "NEGLIGIBLE"
              }
            ]
          }
        ]
      }
      ,
      {
        "candidates": [
          {
            "finishReason": "SAFETY",
            "index": 0,
            "safetyRatings": [
              {
                "category": "HARM_CATEGORY_SEXUALLY_EXPLICIT",
                "probability": "HIGH"
              },
              {
                "category": "HARM_CATEGORY_HATE_SPEECH",
                "probability": "NEGLIGIBLE"
              },
              {
                "category": "HARM_CATEGORY_HARASSMENT",
                "probability": "NEGLIGIBLE"
              },
              {
                "category": "HARM_CATEGORY_DANGEROUS_CONTENT",
                "probability": "NEGLIGIBLE"
              }
            ]
          }
        ]
      }
      ]"#;
}
