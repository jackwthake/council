use reqwest::Client;
use serde::Deserialize;
use serde_json::json;
use std::fs::{self, OpenOptions};
use std::io::Write;
use tokio::time::{sleep, Duration};

// Configuration structures to match your JSON
#[derive(Deserialize, Clone)]
struct Config {
    agents: Vec<AgentConfig>,
}

#[derive(Deserialize, Clone)]
struct AgentConfig {
    name: String,
    in_file: String,
    out_file: String,
    personal_prompt: String,
}

const MODEL: &str = "gemini-3-flash-preview:cloud";
const MASTER_PROMPT_PATH: &str = "/home/jwt/Code/council/agents/system.txt";

fn get_api_key() -> String {
    fs::read_to_string("/home/jwt/Code/council/agents/ollama.key")
        .expect("Failed to read key file")
        .trim()
        .to_string()
}

async fn api_call(client: &Client, key: &str, msg: &str, personal_prompt_path: &str) -> String {
    // Read prompts from disk
    let master_prompt = fs::read_to_string(MASTER_PROMPT_PATH)
        .expect("Failed to read master system file");
    let personal_prompt = fs::read_to_string(personal_prompt_path)
        .expect("Failed to read personal system file");

    // Combine prompts to anchor identity
    let combined_system = format!("{}\n\nYOUR SPECIFIC IDENTITY:\n{}", master_prompt, personal_prompt);

    let response = client
        .post("https://ollama.com/api/chat")
        .header("Authorization", format!("Bearer {}", key))
        .header("content-type", "application/json")
        .json(&json!({
            "model": MODEL,
            "messages": [
                {
                    "role": "system",
                    "content": combined_system
                },
                {
                    "role": "user",
                    "content": msg
                }
            ]
        }))
        .send()
        .await
        .expect("API request failed");

    let raw = response.text().await.expect("Failed to get response text");
    
    // Parse the streaming-style JSON lines from Ollama
    let mut full_content = String::new();
    for line in raw.lines() {
        if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(line) {
            if let Some(content) = parsed["message"]["content"].as_str() {
                full_content.push_str(content);
            }
        }
    }
    full_content
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = Client::new();
    let key = get_api_key();

    // Load the council configuration
    let config_data = fs::read_to_string("config.json")?;
    let config: Config = serde_json::from_str(&config_data)?;

    let mut handles = vec![];

    for agent in config.agents {
        let client = client.clone();
        let key = key.clone();
        
        // Spawn a unique watcher for each agent
        let handle = tokio::spawn(async move {
            let mut old_content = String::new();
            // println!("Started watcher for agent: {}", agent.name);

            loop {
                if let Ok(contents) = fs::read_to_string(&agent.in_file) {
                    // Check for new input
                    if contents != old_content && !contents.trim().is_empty() {
                        // println!("{} is thinking...", agent.name);
                        
                        let response = api_call(&client, &key, &contents, &agent.personal_prompt).await;

                        if !response.trim().is_empty() {
                            let mut file = OpenOptions::new()
                                .create(true)
                                .write(true)
                                .append(true)
                                .open(&agent.out_file)
                                .expect("Failed to open output file");

                            writeln!(file, "{}", response).expect("Failed to write output");
                        }
                        
                        old_content = contents;
                    }
                }
                sleep(Duration::from_millis(500)).await;
            }
        });
        handles.push(handle);
    }

    // Wait for all agents (blocks until app is closed)
    futures::future::join_all(handles).await;
    Ok(())
}