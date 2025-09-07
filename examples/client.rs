// Rust client example for SPKRD server
// Supports multiple servers via CLI args or config file, with concurrent broadcast
// and verbose output mode via -v flag to show per-server results

use clap::Parser;
use std::fs;
use std::path::PathBuf;
use std::process;

#[derive(Debug)]
struct ServerResult {
    server_url: String,
    success: bool,
    error_message: Option<String>,
}

#[derive(Parser)]
#[command(name = "spkrd-client")]
#[command(about = "A client for the SPKRD server")]
struct Args {
    /// Server URL (overrides config file, can be specified multiple times)
    #[arg(short, long, action = clap::ArgAction::Append)]
    server: Vec<String>,
    
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

fn read_servers_from_config() -> Vec<String> {
    let config_path = get_config_file_path();
    match fs::read_to_string(&config_path) {
        Ok(content) => {
            content
                .lines()
                .map(|line| line.trim())
                .filter(|line| !line.is_empty() && !line.starts_with('#'))
                .map(|line| line.to_string())
                .collect()
        }
        Err(_) => Vec::new(),
    }
}

fn normalize_server_url(url: &str) -> String {
    let mut normalized = url.to_string();
    
    // Add http:// scheme if no scheme is present
    if !normalized.starts_with("http://") && !normalized.starts_with("https://") {
        normalized = format!("http://{}", normalized);
    }
    
    // Remove trailing slashes
    normalized = normalized.trim_end_matches('/').to_string();
    
    // Add default port if not present
    // Find the scheme separator and check if there's a port after the hostname
    if let Some(scheme_pos) = normalized.find("://") {
        let after_scheme = &normalized[scheme_pos + 3..];
        // Check if there's already a port (contains : after the hostname)
        // We need to be careful not to confuse IPv6 addresses, but for basic string manipulation
        // we'll assume no IPv6 and just check for a single colon
        if !after_scheme.contains(':') {
            normalized = format!("{}:1111", normalized);
        }
    }
    
    normalized
}

fn get_server_urls(args: &Args) -> Result<Vec<String>, String> {
    let raw_urls = if !args.server.is_empty() {
        args.server.clone()
    } else {
        let config_servers = read_servers_from_config();
        if config_servers.is_empty() {
            return Err(format!(
                "No server URLs provided. Use --server option or create {}",
                get_config_file_path().display()
            ));
        }
        config_servers
    };
    
    let normalized_urls = raw_urls
        .iter()
        .map(|url| normalize_server_url(url))
        .collect();
    
    Ok(normalized_urls)
}

async fn send_melody_to_server(server_url: String, melody: String, verbose: bool) -> ServerResult {
    let client = reqwest::Client::new();
    let url = format!("{}/play", server_url);
    
    if verbose {
        println!("Sending to: {}", url);
    }
    
    match client.put(&url)
        .body(melody)
        .send()
        .await
    {
        Ok(response) => {
            match response.status().as_u16() {
                200 => ServerResult {
                    server_url: server_url,
                    success: true,
                    error_message: None,
                },
                400 => {
                    let error = response.text().await.unwrap_or_else(|_| "Bad request".to_string());
                    ServerResult {
                        server_url: server_url,
                        success: false,
                        error_message: Some(format!("Invalid melody: {}", error)),
                    }
                }
                503 => {
                    let error = response.text().await.unwrap_or_else(|_| "Service unavailable".to_string());
                    ServerResult {
                        server_url: server_url,
                        success: false,
                        error_message: Some(format!("Device busy: {}", error)),
                    }
                }
                500 => {
                    let error = response.text().await.unwrap_or_else(|_| "Internal server error".to_string());
                    ServerResult {
                        server_url: server_url,
                        success: false,
                        error_message: Some(format!("Server error: {}", error)),
                    }
                }
                status => {
                    ServerResult {
                        server_url: server_url,
                        success: false,
                        error_message: Some(format!("Unexpected response: HTTP {}", status)),
                    }
                }
            }
        }
        Err(e) => {
            ServerResult {
                server_url: server_url,
                success: false,
                error_message: Some(format!("Connection error: {}", e)),
            }
        }
    }
}

#[tokio::main]
async fn main() {
    let args = Args::parse();
    
    let server_urls = match get_server_urls(&args) {
        Ok(urls) => urls,
        Err(err) => {
            eprintln!("Error: {}", err);
            eprintln!("Example: {} --server http://192.168.1.100:1111 --server http://192.168.1.101:1111 \"cdefgab\"", env!("CARGO_PKG_NAME"));
            process::exit(1);
        }
    };
    
    let melody = &args.melody;
    
    if args.verbose {
        println!("Playing melody: {}", melody);
        println!("Sending to {} server(s)", server_urls.len());
    }
    
    // Send melody to all servers concurrently and collect results
    let mut handles = Vec::new();
    for server_url in server_urls {
        let melody = melody.clone();
        let handle = tokio::spawn(send_melody_to_server(server_url, melody, args.verbose));
        handles.push(handle);
    }
    
    // Wait for all requests to complete
    let mut results = Vec::new();
    for handle in handles {
        results.push(handle.await);
    }
    
    let mut success_count = 0;
    let mut total_count = 0;
    
    for task_result in results {
        match task_result {
            Ok(server_result) => {
                total_count += 1;
                if server_result.success {
                    success_count += 1;
                    if args.verbose {
                        println!("✓ {} - Melody played successfully", server_result.server_url);
                    }
                } else {
                    if let Some(error) = server_result.error_message {
                        eprintln!("✗ {} - {}", server_result.server_url, error);
                    }
                }
            }
            Err(e) => {
                total_count += 1;
                eprintln!("✗ Task failed: {}", e);
            }
        }
    }
    
    if args.verbose {
        println!("Results: {}/{} servers succeeded", success_count, total_count);
    }
    
    if success_count > 0 {
        process::exit(0);
    } else {
        process::exit(1);
    }
}