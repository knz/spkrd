// Rust client example for SPKRD server
// Supports verbose output mode via -v flag to show informational messages

use clap::Parser;
use std::fs;
use std::path::PathBuf;
use std::process;

#[derive(Parser)]
#[command(name = "spkrd-client")]
#[command(about = "A client for the SPKRD server")]
struct Args {
    /// Server URL (overrides config file)
    #[arg(short, long)]
    server: Option<String>,
    
    /// Enable verbose output
    #[arg(short, long)]
    verbose: bool,
    
    /// Melody to play
    melody: String,
}

fn get_config_file_path() -> PathBuf {
    let home = std::env::var("HOME").expect("HOME environment variable not set");
    PathBuf::from(home).join(".spkrc")
}

fn read_server_from_config() -> Option<String> {
    let config_path = get_config_file_path();
    match fs::read_to_string(&config_path) {
        Ok(content) => Some(content.trim().to_string()),
        Err(_) => None,
    }
}

fn get_server_url(args: &Args) -> Result<String, String> {
    if let Some(server) = &args.server {
        Ok(server.clone())
    } else if let Some(server) = read_server_from_config() {
        Ok(server)
    } else {
        Err(format!(
            "No server URL provided. Use --server option or create {}",
            get_config_file_path().display()
        ))
    }
}

#[tokio::main]
async fn main() {
    let args = Args::parse();
    
    let server_url = match get_server_url(&args) {
        Ok(url) => url,
        Err(err) => {
            eprintln!("Error: {}", err);
            eprintln!("Example: {} --server http://192.168.1.100:8080 \"cdefgab\"", env!("CARGO_PKG_NAME"));
            process::exit(1);
        }
    };
    
    let melody = &args.melody;
    
    let client = reqwest::Client::new();
    let url = format!("{}/play", server_url);
    
    if args.verbose {
        println!("Playing melody: {}", melody);
        println!("Server: {}", url);
    }
    
    match client.put(&url)
        .body(melody.clone())
        .send()
        .await
    {
        Ok(response) => {
            match response.status().as_u16() {
                200 => {
                    if args.verbose {
                        println!("✓ Melody played successfully");
                    }
                }
                400 => {
                    let error = response.text().await.unwrap_or_else(|_| "Bad request".to_string());
                    eprintln!("✗ Invalid melody: {}", error);
                    process::exit(1);
                }
                503 => {
                    let error = response.text().await.unwrap_or_else(|_| "Service unavailable".to_string());
                    eprintln!("✗ Device busy: {}", error);
                    process::exit(1);
                }
                500 => {
                    let error = response.text().await.unwrap_or_else(|_| "Internal server error".to_string());
                    eprintln!("✗ Server error: {}", error);
                    process::exit(1);
                }
                status => {
                    eprintln!("✗ Unexpected response: HTTP {}", status);
                    process::exit(1);
                }
            }
        }
        Err(e) => {
            eprintln!("✗ Connection error: {}", e);
            process::exit(1);
        }
    }
}