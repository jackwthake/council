use std::fs;
use tokio::io::{self, AsyncBufReadExt, BufReader};
use tokio::time::{sleep, Duration};
use std::fs::OpenOptions;
use std::io::Write;

#[tokio::main]
async fn main() {
    let fname = "/tmp/council/minimax.out";
    
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
                println!("minimax > {}", diff);
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
        
        let mut file = OpenOptions::new()
            .create(true)
            .write(true)
            .append(true)
            .open("/tmp/council/minimax.in")
            .unwrap();

        writeln!(file, "{}", line).unwrap();
    }
}