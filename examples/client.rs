// Rust client example for SPKRD server

use std::env;
use std::process;

#[tokio::main]
async fn main() {
    let args: Vec<String> = env::args().collect();
    
    if args.len() != 3 {
        eprintln!("Usage: {} <server_url> <melody>", args[0]);
        eprintln!("Example: {} http://192.168.1.100:8080 \"cdefgab\"", args[0]);
        process::exit(1);
    }
    
    let server_url = &args[1];
    let melody = &args[2];
    
    let client = reqwest::Client::new();
    let url = format!("{}/play", server_url);
    
    println!("Playing melody: {}", melody);
    println!("Server: {}", url);
    
    match client.put(&url)
        .body(melody.clone())
        .send()
        .await
    {
        Ok(response) => {
            match response.status().as_u16() {
                200 => println!("✓ Melody played successfully"),
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