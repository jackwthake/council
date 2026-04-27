use std::fs;
use tokio::io::{self, AsyncBufReadExt, BufReader};
use tokio::time::{sleep, Duration};

#[tokio::main]
async fn main() {
    let fname = "/tmp/council/test.out";
    
    tokio::select! {
        _ = watch_file(fname) => {},
        _ = read_keyboard() => {},
    }
}

async fn watch_file(fname: &str) {
    let mut old_content = String::new();
    loop {
        if let Ok(contents) = fs::read_to_string(fname) {
            if contents != old_content {
                let diff = &contents[old_content.len()..];
                println!("tester > {}", diff);
                old_content = contents;
            }
        }
        sleep(Duration::from_millis(100)).await;
    }
}

async fn read_keyboard() {
    let stdin = io::stdin();
    let mut reader = BufReader::new(stdin).lines();
    while let Ok(Some(line)) = reader.next_line().await {
        if line.trim() == "/exit" {
            break;
        }
        println!("input: {}", line);
    }
}