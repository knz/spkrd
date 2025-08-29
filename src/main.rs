// FreeBSD speaker device network server
// Main entry point with CLI argument parsing and server initialization

use clap::Parser;
use std::time::Duration;

mod error;
mod server;
mod speaker;

#[derive(Parser)]
#[command(author, version, about = "FreeBSD speaker device network server", long_about = None)]
struct Args {
    #[arg(short, long, default_value = "8080")]
    port: u16,

    #[arg(short, long, default_value = "30", help = "Retry timeout in seconds")]
    retry_timeout: u64,

    #[arg(short, long, default_value = "/dev/speaker", help = "Path to speaker device")]
    device: String,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();
    let retry_timeout = Duration::from_secs(args.retry_timeout);

    println!("Starting spkrd server on port {} with {}s retry timeout using device {}", 
             args.port, args.retry_timeout, args.device);

    server::run(args.port, retry_timeout, args.device).await?;
    Ok(())
}