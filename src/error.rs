// Error types for speaker device operations

use std::fmt;

#[derive(Debug)]
pub enum SpeakerError {
    DeviceBusy,
    DeviceError(std::io::Error),
    InvalidMelody(String),
    Timeout,
}

impl fmt::Display for SpeakerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SpeakerError::DeviceBusy => write!(f, "Speaker device is busy"),
            SpeakerError::DeviceError(e) => write!(f, "Device error: {}", e),
            SpeakerError::InvalidMelody(msg) => write!(f, "Invalid melody: {}", msg),
            SpeakerError::Timeout => write!(f, "Operation timed out"),
        }
    }
}

impl std::error::Error for SpeakerError {}

impl From<std::io::Error> for SpeakerError {
    fn from(err: std::io::Error) -> Self {
        use std::io::ErrorKind;
        match err.kind() {
            ErrorKind::WouldBlock | ErrorKind::AddrInUse => SpeakerError::DeviceBusy,
            _ => SpeakerError::DeviceError(err),
        }
    }
}