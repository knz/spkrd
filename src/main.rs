// FreeBSD speaker device network server
// Main entry point with CLI argument parsing and server initialization

use clap::Parser;
use std::time::Duration;
use daemonize::Daemonize;
use log::{info, error};
use syslog::{Formatter3164, BasicLogger, Facility};
use std::process;

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

    #[arg(long, help = "Run as daemon in background")]
    daemon: bool,

    #[arg(long, default_value = "/var/run/spkrd.pid", help = "Path to PID file (requires appropriate permissions for default path)")]
    pidfile: String,

    #[arg(short, long, help = "Enable debug logging including client request details")]
    debug: bool,
}

fn init_logging(daemon: bool, debug: bool) {
    if daemon {
        // Use syslog for daemon mode
        let formatter = Formatter3164 {
            facility: Facility::LOG_DAEMON,
            hostname: None,
            process: "spkrd".into(),
            pid: process::id(),
        };
        
        match syslog::unix(formatter) {
            Ok(logger) => {
                log::set_boxed_logger(Box::new(BasicLogger::new(logger)))
                    .map(|()| log::set_max_level(if debug { log::LevelFilter::Debug } else { log::LevelFilter::Info }))
                    .unwrap_or_else(|e| {
                        eprintln!("Failed to initialize syslog: {}", e);
                        process::exit(1);
                    });
            }
            Err(e) => {
                eprintln!("Failed to connect to syslog: {}", e);
                process::exit(1);
            }
        }
    } else {
        // Use stderr for non-daemon mode
        env_logger::Builder::new()
            .filter_level(if debug { log::LevelFilter::Debug } else { log::LevelFilter::Info })
            .init();
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();
    let retry_timeout = Duration::from_secs(args.retry_timeout);

    // Initialize logging before anything else
    init_logging(args.daemon, args.debug);

    // Log startup configuration (always)
    info!("Starting spkrd server: port={}, retry_timeout={}s, device={}, daemon={}, pidfile={}, debug={}", 
          args.port, args.retry_timeout, args.device, args.daemon, args.pidfile, args.debug);

    // Daemonize if requested
    if args.daemon {
        let daemonize = Daemonize::new()
            .pid_file(&args.pidfile)
            .working_directory("/");

        match daemonize.start() {
            Ok(_) => {
                // We are now in the daemon process - logging should still work through syslog
                info!("Daemon started successfully");
            },
            Err(e) => {
                error!("Error starting daemon: {}", e);
                process::exit(1);
            }
        }
    }

    // Create and run the Tokio runtime after daemonization
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .map_err(|e| format!("Failed to create Tokio runtime: {}", e))?;

    runtime.block_on(async move {
        match server::run(args.port, retry_timeout, args.device, args.debug).await {
            Ok(_) => {
                info!("Server shutdown completed");
                Ok(())
            }
            Err(e) => {
                error!("Server error: {}", e);
                Err(e)
            }
        }
    })
}