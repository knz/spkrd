// Error types for speaker device operations.
//
// CpalError vs CpalDisconnect: CpalError is fatal (unsupported config,
// permission denied, generic backend error) — surfaced verbatim to the
// HTTP client. CpalDisconnect is the "transient, retryable" subset
// (host/device went away, stream invalidated) — the cpal backend's
// acquire_and_play retry loop matches on this variant and rebuilds the
// device on the same 1s cadence as the busy-device retry, sharing the
// request's --retry-timeout window. Only after the timeout elapses does
// a CpalDisconnect propagate up to the HTTP layer.

use std::fmt;

#[derive(Debug)]
pub enum SpeakerError {
    DeviceBusy,
    DeviceError(std::io::Error),
    InvalidMelody(String),
    Timeout,
    #[cfg(feature = "cpal")]
    CpalError(String),
    #[cfg(feature = "cpal")]
    CpalDisconnect(String),
}

impl fmt::Display for SpeakerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SpeakerError::DeviceBusy => write!(f, "Speaker device is busy"),
            SpeakerError::DeviceError(e) => write!(f, "Device error: {}", e),
            SpeakerError::InvalidMelody(msg) => write!(f, "Invalid melody: {}", msg),
            SpeakerError::Timeout => write!(f, "Operation timed out"),
            #[cfg(feature = "cpal")]
            SpeakerError::CpalError(msg) => write!(f, "CPAL error: {}", msg),
            #[cfg(feature = "cpal")]
            SpeakerError::CpalDisconnect(msg) => write!(f, "CPAL disconnect: {}", msg),
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