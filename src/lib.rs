// Library interface for spkrd. Up to two output backends are exposed:
// freebsd_speaker (writes the raw melody string to /dev/speaker) is always
// compiled; cpal_backend (parses MML via the mml module and synthesises a
// waveform through the host's audio output) is gated behind the `cpal`
// Cargo feature, which is enabled by default. server::run dispatches to
// whichever backend has been selected at startup.

pub mod error;
pub mod server;
pub mod freebsd_speaker;
pub mod mml;
#[cfg(feature = "cpal")]
pub mod cpal_backend;
