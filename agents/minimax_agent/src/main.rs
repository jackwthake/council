use reqwest::Client;
use serde_json::json;
use std::fs::OpenOptions;
use std::fs;
use std::io::Write;
use tokio::time::{sleep, Duration};

const INPUT_FNAME: &str = "/tmp/council/minimax.in";
const OUTPUT_FNAME: &str = "/tmp/council/minimax.out";
const MODEL: &str = "minimax-m2.7:cloud";

fn get_api_key() -> String {
    fs::read_to_string("/home/jwt/Code/council/agents/ollama.key")
        .expect("Failed to read key file")
        .trim()
        .to_string()
}

async fn api_call(client: &Client, key: &str, msg : &str) -> String {
    let response = client
        .post("https://ollama.com/api/chat")
        .header("Authorization", format!("Bearer {}", key))
        .header("content-type", "application/json")
        .json(&json!({
            "model": MODEL,
            "messages": [
                {
                "role": "system",
                "content": fs::read_to_string("/home/jwt/Code/council/agents/system.txt")
                                .expect("Failed to read system file")
                                .trim()
                                .to_string()
                },
                {
                "role": "user",
                "content": msg
                }
            ]
        }))
        .send()
        .await
        .unwrap();

    response.text().await.unwrap()
}

#[tokio::main]
async fn main() {
    let client = Client::new();
    let key = get_api_key();

    let mut old_content = String::new();

    loop {
        if let Ok(contents) = fs::read_to_string(INPUT_FNAME) {
            if contents == old_content || contents.trim().is_empty() {
                continue;
            }

            let raw = api_call(&client, &key, &contents).await;
            let mut full_content = String::new();

            for line in raw.lines() {
                if line.is_empty() {
                    continue;
                }
                if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(line) {
                    if let Some(content) = parsed["message"]["content"].as_str() {
                        full_content.push_str(content);
                    }
                }
            }

            let mut file = OpenOptions::new()
                .create(true)
                .write(true)
                .append(true)
                .open(OUTPUT_FNAME)
                .unwrap();

            writeln!(file, "{}", full_content).unwrap();

            
            old_content = contents;
        }

        sleep(Duration::from_millis(100)).await;
    }
}