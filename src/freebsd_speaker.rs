// FreeBSD /dev/speaker backend: writes the raw melody string to a character
// device with retry-on-busy logic. Mirrors the original behaviour of this
// program before the CPAL backend was added.

use crate::error::SpeakerError;
use std::fs::OpenOptions;
use std::io::Write;
use std::net::SocketAddr;
use std::time::{Duration, Instant};
use tokio::time::sleep;
use log::debug;

const RETRY_INTERVAL: Duration = Duration::from_secs(1);

pub async fn play_melody(
    melody: &str,
    client_addr: SocketAddr,
    retry_timeout: Duration,
    device_path: &str,
    max_melody_length: usize,
    debug: bool,
) -> Result<u32, SpeakerError> {
    validate_melody(melody, max_melody_length)?;

    if debug {
        log_request(client_addr, melody);
    }

    let start_time = Instant::now();
    let mut retries = 0;
    
    loop {
        match try_play_melody(melody, device_path) {
            Ok(()) => return Ok(retries),
            Err(SpeakerError::DeviceBusy) => {
                if start_time.elapsed() >= retry_timeout {
                    return Err(SpeakerError::Timeout);
                }
                retries += 1;
                sleep(RETRY_INTERVAL).await;
                continue;
            }
            Err(e) => return Err(e),
        }
    }
}

fn validate_melody(melody: &str, max_melody_length: usize) -> Result<(), SpeakerError> {
    if melody.len() > max_melody_length {
        return Err(SpeakerError::InvalidMelody(
            format!("Melody exceeds {} bytes", max_melody_length),
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
    let printable_melody: String = melody
        .chars()
        .filter(|c| c.is_ascii() && (c.is_alphanumeric() || c.is_ascii_punctuation() || c.is_whitespace()))
        .collect();
    
    debug!("Request from {}: melody={}", client_addr.ip(), printable_melody);
}