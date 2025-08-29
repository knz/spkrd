// Speaker device handling with retry logic for FreeBSD /dev/speaker

use crate::error::SpeakerError;
use chrono::{DateTime, Utc};
use std::fs::OpenOptions;
use std::io::Write;
use std::net::SocketAddr;
use std::time::{Duration, Instant};
use tokio::time::sleep;

const RETRY_INTERVAL: Duration = Duration::from_secs(1);

pub async fn play_melody(
    melody: &str,
    client_addr: SocketAddr,
    retry_timeout: Duration,
    device_path: &str,
) -> Result<(), SpeakerError> {
    validate_melody(melody)?;
    log_request(client_addr, melody);

    let start_time = Instant::now();
    
    loop {
        match try_play_melody(melody, device_path) {
            Ok(()) => return Ok(()),
            Err(SpeakerError::DeviceBusy) => {
                if start_time.elapsed() >= retry_timeout {
                    return Err(SpeakerError::Timeout);
                }
                sleep(RETRY_INTERVAL).await;
                continue;
            }
            Err(e) => return Err(e),
        }
    }
}

fn validate_melody(melody: &str) -> Result<(), SpeakerError> {
    if melody.len() > 1000 {
        return Err(SpeakerError::InvalidMelody(
            "Melody exceeds 1000 characters".to_string(),
        ));
    }
    Ok(())
}

fn try_play_melody(melody: &str, device_path: &str) -> Result<(), SpeakerError> {
    let mut file = OpenOptions::new()
        .write(true)
        .open(device_path)?;
    
    file.write_all(melody.as_bytes())?;
    Ok(())
}

fn log_request(client_addr: SocketAddr, melody: &str) {
    let timestamp: DateTime<Utc> = Utc::now();
    let printable_melody: String = melody
        .chars()
        .filter(|c| c.is_ascii() && (c.is_alphanumeric() || c.is_ascii_punctuation() || c.is_whitespace()))
        .collect();
    
    println!("[{}] {} - Melody: {}", 
             timestamp.format("%Y-%m-%d %H:%M:%S UTC"),
             client_addr.ip(),
             printable_melody);
}