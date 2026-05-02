// CPAL audio output backend. Renders an MML melody to PCM via the mml module,
// synthesises the chosen waveform at the device's configured sample rate, and
// plays it through cpal's default (or selected) host/device. A global mutex
// enforces one-melody-at-a-time semantics matching FreeBSD spkr.c's exclusive
// sx lock; busy callers retry on the same schedule as the freebsd-speaker
// backend until --retry-timeout elapses.
//
// The lock is held *inside* the spawn_blocking task that owns the live
// cpal::Stream — not in the async parent — so that an HTTP-client disconnect
// (which drops the parent future) cannot release the lock while audio is
// still playing in CPAL's audio thread. An abort flag installed by the parent
// is observed by the audio callback, mirroring FreeBSD spkr.c's PCATCH-aware
// tsleep that lets a signal interrupt playback mid-string.

use crate::error::SpeakerError;
use crate::mml::{self, Event};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{FromSample, SampleFormat, SizedSample, StreamConfig};
use log::{debug, info, warn};
use std::f32::consts::PI;
use std::net::SocketAddr;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

const RETRY_INTERVAL: Duration = Duration::from_secs(1);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Waveform {
    Square,
    SquareBandlimited,
    Sine,
    Triangle,
    Sawtooth,
    // Modern piezoelectric PC-speaker simulation: square wave generated at a
    // PIT-quantised frequency, processed through a 3-stage biquad chain
    // (HP/peak/LP) tuned to a small piezo disc, then soft-clipped via tanh.
    // Filter state persists across the entire event sequence so rests ring
    // out naturally instead of cutting off abruptly.
    PcSpeaker,
}

impl std::str::FromStr for Waveform {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_ascii_lowercase().as_str() {
            "square" => Ok(Waveform::Square),
            "square-bandlimited" | "squarebandlimited" | "bl-square" => {
                Ok(Waveform::SquareBandlimited)
            }
            "sine" => Ok(Waveform::Sine),
            "triangle" => Ok(Waveform::Triangle),
            "sawtooth" | "saw" => Ok(Waveform::Sawtooth),
            "pc-speaker" | "pcspeaker" | "pc" => Ok(Waveform::PcSpeaker),
            other => Err(format!("unknown waveform: {}", other)),
        }
    }
}

// Attack/release envelope length applied to the non-PC-speaker waveforms to
// suppress amplitude-step clicks at note boundaries. 5 ms is short enough to
// be inaudible as a fade and long enough to push the boundary transient
// below the audible click threshold.
const ENVELOPE_MS: f32 = 5.0;

// Intel 8254 PIT clock — used by real PC-speaker hardware. Note frequencies
// in the simulation are quantised to PIT_FREQ / divisor for an integer
// divisor, matching what the kernel driver actually programs.
const PIT_FREQ: u32 = 1_193_182;

// Modern piezo PC-speaker preset: steep HP to kill sub-bass the disc can't
// move, peaking boost in the resonant midrange for the buzzy character, LP
// for cone roll-off, and a tanh saturator for the driver-clip edge.
const PIEZO_HP_HZ: f32 = 800.0;
const PIEZO_HP_Q: f32 = 0.707;
const PIEZO_PEAK_HZ: f32 = 3000.0;
const PIEZO_PEAK_Q: f32 = 3.0;
const PIEZO_PEAK_DB: f32 = 9.0;
const PIEZO_LP_HZ: f32 = 6000.0;
const PIEZO_LP_Q: f32 = 0.707;
const PIEZO_DRIVE: f32 = 2.0;

pub struct CpalConfig {
    pub host: Option<String>,
    pub device: Option<String>,
    pub sample_rate: Option<u32>,
    pub volume: f32,
    pub waveform: Waveform,
}

pub struct CpalBackend {
    device: cpal::Device,
    config: StreamConfig,
    sample_format: SampleFormat,
    volume: f32,
    waveform: Waveform,
    // Held by the spawn_blocking task that owns the live cpal::Stream, not by
    // the async parent. See module-level comment on cancellation safety.
    play_lock: Mutex<()>,
}

// Sets the inner abort flag on drop. Installed in the async parent so that
// when axum drops the request handler future (e.g. on client disconnect),
// the in-progress audio playback is signalled to stop early.
struct AbortOnDrop(Arc<AtomicBool>);

impl Drop for AbortOnDrop {
    fn drop(&mut self) {
        self.0.store(true, Ordering::SeqCst);
    }
}

impl CpalBackend {
    pub fn new(cfg: &CpalConfig) -> Result<Self, SpeakerError> {
        let host = match &cfg.host {
            Some(name) => {
                let id = cpal::available_hosts()
                    .into_iter()
                    .find(|h| h.name().eq_ignore_ascii_case(name))
                    .ok_or_else(|| {
                        SpeakerError::CpalError(format!("unknown cpal host: {}", name))
                    })?;
                cpal::host_from_id(id)
                    .map_err(|e| SpeakerError::CpalError(format!("host_from_id: {}", e)))?
            }
            None => cpal::default_host(),
        };

        let device = match &cfg.device {
            Some(name) => {
                let mut found: Option<cpal::Device> = None;
                let devs = host
                    .output_devices()
                    .map_err(|e| SpeakerError::CpalError(format!("output_devices: {}", e)))?;
                for d in devs {
                    if let Ok(n) = d.name() {
                        if n == *name {
                            found = Some(d);
                            break;
                        }
                    }
                }
                found.ok_or_else(|| {
                    SpeakerError::CpalError(format!("output device not found: {}", name))
                })?
            }
            None => host
                .default_output_device()
                .ok_or_else(|| SpeakerError::CpalError("no default output device".into()))?,
        };

        let default_cfg = device
            .default_output_config()
            .map_err(|e| SpeakerError::CpalError(format!("default_output_config: {}", e)))?;
        let sample_format = default_cfg.sample_format();
        let mut stream_cfg: StreamConfig = default_cfg.into();
        if let Some(sr) = cfg.sample_rate {
            stream_cfg.sample_rate = cpal::SampleRate(sr);
        }

        info!(
            "CPAL backend: device={:?}, sample_rate={}, channels={}, format={:?}, waveform={:?}, volume={}",
            device.name().unwrap_or_else(|_| "<unknown>".into()),
            stream_cfg.sample_rate.0,
            stream_cfg.channels,
            sample_format,
            cfg.waveform,
            cfg.volume,
        );

        Ok(Self {
            device,
            config: stream_cfg,
            sample_format,
            volume: cfg.volume,
            waveform: cfg.waveform,
            play_lock: Mutex::new(()),
        })
    }

    pub async fn play_melody(
        self: &Arc<Self>,
        melody: &str,
        client_addr: SocketAddr,
        retry_timeout: Duration,
        debug: bool,
    ) -> Result<u32, SpeakerError> {
        validate_melody(melody)?;
        if debug {
            log_request(client_addr, melody);
        }

        // Synthesis is pure CPU work — render in the async parent. (Even a
        // 1000-char melody is well under a millisecond at typical sample
        // rates.)
        let events = mml::render(melody);
        let sr = self.config.sample_rate.0;
        let buffer = synth(&events, sr, self.waveform, self.volume);

        if buffer.is_empty() {
            return Ok(0);
        }

        // Cancellation channel: setting `abort` makes the audio callback stop
        // emitting samples and signal end-of-stream. The guard installs a
        // Drop hook on the async parent — when axum drops this future on
        // client disconnect, the flag is set, the cpal callback observes it,
        // and the blocking task ends and releases play_lock.
        let abort = Arc::new(AtomicBool::new(false));
        let _abort_on_drop = AbortOnDrop(Arc::clone(&abort));

        // Move the lock acquisition + audio playback into the blocking task.
        // The lock is held by this task (synchronously) for the entire
        // duration of the audio, so the next request cannot start a parallel
        // stream even if our parent future is dropped before we return.
        let backend = Arc::clone(self);
        let task_abort = Arc::clone(&abort);
        let join = tokio::task::spawn_blocking(move || {
            backend.acquire_and_play(buffer, retry_timeout, task_abort)
        });

        match join.await {
            Ok(result) => result,
            Err(e) => Err(SpeakerError::CpalError(format!("join error: {}", e))),
        }
    }

    // Synchronous: acquire play_lock with retry-poll, then play. Runs on a
    // tokio blocking thread. Holds the lock for the entire audio duration.
    fn acquire_and_play(
        &self,
        buffer: Vec<f32>,
        retry_timeout: Duration,
        abort: Arc<AtomicBool>,
    ) -> Result<u32, SpeakerError> {
        let start = Instant::now();
        let mut retries: u32 = 0;
        let _guard = loop {
            // If the client already disconnected before we even got to
            // acquire the lock, drop out cleanly without playing anything.
            if abort.load(Ordering::SeqCst) {
                return Ok(retries);
            }
            match self.play_lock.try_lock() {
                Ok(g) => break g,
                Err(_) => {
                    if start.elapsed() >= retry_timeout {
                        return Err(SpeakerError::Timeout);
                    }
                    retries += 1;
                    std::thread::sleep(RETRY_INTERVAL);
                }
            }
        };

        // Lock held — play the audio. Returns when the audio finishes or
        // when `abort` is set.
        self.play_buffer(buffer, abort)?;
        Ok(retries)
    }

    fn play_buffer(&self, buffer: Vec<f32>, abort: Arc<AtomicBool>) -> Result<(), SpeakerError> {
        match self.sample_format {
            SampleFormat::F32 => self.run_stream::<f32>(buffer, abort),
            SampleFormat::F64 => self.run_stream::<f64>(buffer, abort),
            SampleFormat::I16 => self.run_stream::<i16>(buffer, abort),
            SampleFormat::I32 => self.run_stream::<i32>(buffer, abort),
            SampleFormat::U16 => self.run_stream::<u16>(buffer, abort),
            SampleFormat::I8 => self.run_stream::<i8>(buffer, abort),
            SampleFormat::U8 => self.run_stream::<u8>(buffer, abort),
            other => Err(SpeakerError::CpalError(format!(
                "unsupported sample format: {:?}",
                other
            ))),
        }
    }

    fn run_stream<T>(
        &self,
        buffer: Vec<f32>,
        abort: Arc<AtomicBool>,
    ) -> Result<(), SpeakerError>
    where
        T: SizedSample + FromSample<f32> + Send + 'static,
    {
        let channels = self.config.channels as usize;
        let total = buffer.len();
        let cursor = Arc::new(Mutex::new(0usize));
        let done = Arc::new((Mutex::new(false), std::sync::Condvar::new()));
        let err_done = Arc::clone(&done);

        let buf = Arc::new(buffer);
        let cb_buf = Arc::clone(&buf);
        let cb_cursor = Arc::clone(&cursor);
        let cb_done = Arc::clone(&done);
        let cb_abort = Arc::clone(&abort);

        let stream = self
            .device
            .build_output_stream(
                &self.config,
                move |out: &mut [T], _info: &cpal::OutputCallbackInfo| {
                    // If the parent future was dropped, write zeros for the
                    // remainder of this callback and signal end-of-buffer.
                    // Mirrors FreeBSD spkr.c's PCATCH-aware tsleep that
                    // shorts a melody on signal interrupt.
                    let aborted = cb_abort.load(Ordering::SeqCst);
                    let mut idx = cb_cursor.lock().unwrap();
                    for frame in out.chunks_mut(channels) {
                        let v: f32 = if !aborted && *idx < total {
                            cb_buf[*idx]
                        } else {
                            0.0
                        };
                        let s: T = T::from_sample(v);
                        for ch in frame.iter_mut() {
                            *ch = s;
                        }
                        if !aborted && *idx < total {
                            *idx += 1;
                        }
                    }
                    if aborted || *idx >= total {
                        let (lock, cv) = &*cb_done;
                        let mut d = lock.lock().unwrap();
                        if !*d {
                            *d = true;
                            cv.notify_all();
                        }
                    }
                },
                move |err| {
                    warn!("cpal stream error: {}", err);
                    let (lock, cv) = &*err_done;
                    let mut d = lock.lock().unwrap();
                    *d = true;
                    cv.notify_all();
                },
                None,
            )
            .map_err(|e| SpeakerError::CpalError(format!("build_output_stream: {}", e)))?;

        stream
            .play()
            .map_err(|e| SpeakerError::CpalError(format!("stream.play: {}", e)))?;

        let (lock, cv) = &*done;
        let mut d = lock.lock().unwrap();
        while !*d {
            d = cv.wait(d).unwrap();
        }
        drop(d);

        // Add a small tail so the device can flush buffered samples before we
        // drop the stream — except when we're aborting, where we want
        // cancellation to be snappy.
        if !abort.load(Ordering::SeqCst) {
            std::thread::sleep(Duration::from_millis(50));
        }
        drop(stream);
        Ok(())
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

fn log_request(client_addr: SocketAddr, melody: &str) {
    let printable: String = melody
        .chars()
        .filter(|c| {
            c.is_ascii() && (c.is_alphanumeric() || c.is_ascii_punctuation() || c.is_whitespace())
        })
        .collect();
    debug!("Request from {}: melody={}", client_addr.ip(), printable);
}

// Synthesise the event sequence into a mono f32 PCM buffer. Dispatches to a
// dedicated PC-speaker path (square + biquad chain + saturation) or to the
// generic oscillator path (Square / SquareBandlimited / Sine / Triangle /
// Sawtooth) which adds an AR envelope to suppress note-boundary clicks.
fn synth(events: &[Event], sr: u32, wf: Waveform, volume: f32) -> Vec<f32> {
    match wf {
        Waveform::PcSpeaker => synth_pcspeaker(events, sr, volume),
        _ => synth_generic(events, sr, wf, volume),
    }
}

// Precompute the total sample count for buffer preallocation.
fn total_samples(events: &[Event], sr: u32) -> usize {
    let total_cs: u64 = events
        .iter()
        .map(|e| match *e {
            Event::Tone { centisecs, .. } => centisecs as u64,
            Event::Rest { centisecs } => centisecs as u64,
        })
        .sum();
    (total_cs as usize) * (sr as usize) / 100
}

// Generic oscillator path. Behaviour is per-waveform:
//
//   * Waveform::Square is the kernel-faithful raw output: phase is reset to
//     0 at every Tone event start (mirroring spkr.c's timer_spkr_setfreq()
//     resetting the PIT counter), and no AR envelope is applied. This
//     reproduces the FreeBSD driver's hard-edged amplitude-step transients
//     at note boundaries — the "plink" the ear locks onto as articulation
//     even when consecutive notes share a frequency. Boundary clicks are
//     intentional here.
//
//   * The remaining software-only waveforms (SquareBandlimited, Sine,
//     Triangle, Sawtooth) have no FreeBSD analog. They keep phase continuity
//     across consecutive tones and apply a 5 ms linear AR envelope (capped
//     at n/4 each side so attack+release can't exceed half a short staccato
//     note) to suppress the amplitude-step clicks that would otherwise be
//     audible at every Tone boundary.
fn synth_generic(events: &[Event], sr: u32, wf: Waveform, volume: f32) -> Vec<f32> {
    let sr_f = sr as f32;
    let mut out: Vec<f32> = Vec::with_capacity(total_samples(events, sr));
    let default_ramp = (sr_f * ENVELOPE_MS / 1000.0) as usize;

    let kernel_faithful = matches!(wf, Waveform::Square);

    let mut phase: f32 = 0.0;

    for ev in events {
        match *ev {
            Event::Rest { centisecs } => {
                let n = (centisecs as u64 * sr as u64 / 100) as usize;
                out.extend(std::iter::repeat(0.0).take(n));
                phase = 0.0;
            }
            Event::Tone { freq_hz, centisecs } => {
                let n = (centisecs as u64 * sr as u64 / 100) as usize;
                if n == 0 {
                    continue;
                }
                if kernel_faithful {
                    phase = 0.0;
                }
                let f = freq_hz as f32;
                let dphase = f / sr_f;
                let ramp = default_ramp.min(n / 4).max(1);
                for i in 0..n {
                    let s = match wf {
                        Waveform::Square => {
                            if phase < 0.5 { 1.0 } else { -1.0 }
                        }
                        Waveform::SquareBandlimited => {
                            // PolyBLEP: sawtooth + shifted sawtooth.
                            let saw1 = 2.0 * phase - 1.0;
                            let phase2 = (phase + 0.5).fract();
                            let saw2 = 2.0 * phase2 - 1.0;
                            let sq = saw1 - saw2;
                            sq - poly_blep(phase, dphase)
                                + poly_blep(phase2, dphase)
                        }
                        Waveform::Sine => (2.0 * PI * phase).sin(),
                        Waveform::Triangle => {
                            if phase < 0.25 {
                                4.0 * phase
                            } else if phase < 0.75 {
                                2.0 - 4.0 * phase
                            } else {
                                -4.0 + 4.0 * phase
                            }
                        }
                        Waveform::Sawtooth => 2.0 * phase - 1.0,
                        Waveform::PcSpeaker => unreachable!(),
                    };
                    let gain = if kernel_faithful {
                        1.0
                    } else if i < ramp {
                        i as f32 / ramp as f32
                    } else if i + ramp >= n {
                        (n - 1 - i) as f32 / ramp as f32
                    } else {
                        1.0
                    };
                    out.push(s * gain * volume);
                    phase += dphase;
                    if phase >= 1.0 {
                        phase -= phase.floor();
                    }
                }
            }
        }
    }
    out
}

// PC-speaker simulation path. The note frequency is rounded to the nearest
// PIT-achievable value (PIT_FREQ / divisor) before sample generation —
// matching what real hardware would actually play. A ±1 square at that
// frequency is fed through HP -> peaking -> LP biquads (Modern piezo disc
// preset) and through a tanh saturator. Filter state persists across all
// events including rests, so the speaker "rings out" naturally on note-off.
//
// Phase is reset to 0 at every Tone event start to mirror the PIT counter
// reset that timer_spkr_setfreq() performs in the FreeBSD kernel. The
// resulting amplitude-step transient is shaped by the biquad chain into a
// mechanical-style "plink" — what a real piezo would produce when the gate
// reopens at a fresh PIT count, rather than the sharp DAC click you'd get
// from feeding the same raw signal to a modern audio output.
fn synth_pcspeaker(events: &[Event], sr: u32, volume: f32) -> Vec<f32> {
    let sr_f = sr as f32;
    let mut out: Vec<f32> = Vec::with_capacity(total_samples(events, sr));

    let mut hp = Biquad::highpass(sr, PIEZO_HP_HZ, PIEZO_HP_Q);
    let mut pk = Biquad::peak(sr, PIEZO_PEAK_HZ, PIEZO_PEAK_Q, PIEZO_PEAK_DB);
    let mut lp = Biquad::lowpass(sr, PIEZO_LP_HZ, PIEZO_LP_Q);

    for ev in events {
        match *ev {
            Event::Rest { centisecs } => {
                let n = (centisecs as u64 * sr as u64 / 100) as usize;
                for _ in 0..n {
                    let y = lp.process(pk.process(hp.process(0.0)));
                    out.push((PIEZO_DRIVE * y).tanh() * volume);
                }
            }
            Event::Tone { freq_hz, centisecs } => {
                let n = (centisecs as u64 * sr as u64 / 100) as usize;
                if n == 0 {
                    continue;
                }
                let q_freq = pit_quantize(freq_hz);
                let dphase = q_freq as f32 / sr_f;
                let mut phase: f32 = 0.0;
                for _ in 0..n {
                    let raw = if phase < 0.5 { 1.0 } else { -1.0 };
                    let y = lp.process(pk.process(hp.process(raw)));
                    out.push((PIEZO_DRIVE * y).tanh() * volume);
                    phase += dphase;
                    if phase >= 1.0 {
                        phase -= phase.floor();
                    }
                }
            }
        }
    }
    out
}

// Round a desired frequency to the nearest frequency the PIT can actually
// produce: divisor = round(PIT_FREQ / freq), achievable = PIT_FREQ / divisor.
fn pit_quantize(freq_hz: u32) -> u32 {
    if freq_hz == 0 {
        return 0;
    }
    let divisor = ((PIT_FREQ + freq_hz / 2) / freq_hz).max(1);
    PIT_FREQ / divisor
}

// Direct-form-2 transposed biquad. Coefficients are pre-normalised by a0 at
// construction time so `process` is just five mul-adds.
struct Biquad {
    b0: f32,
    b1: f32,
    b2: f32,
    a1: f32,
    a2: f32,
    s1: f32,
    s2: f32,
}

impl Biquad {
    fn process(&mut self, x: f32) -> f32 {
        let y = self.b0 * x + self.s1;
        self.s1 = self.b1 * x - self.a1 * y + self.s2;
        self.s2 = self.b2 * x - self.a2 * y;
        y
    }

    // RBJ audio cookbook: lowpass biquad.
    fn lowpass(sr: u32, hz: f32, q: f32) -> Self {
        let omega = 2.0 * PI * hz / sr as f32;
        let alpha = omega.sin() / (2.0 * q);
        let cos_w = omega.cos();
        let b0 = (1.0 - cos_w) * 0.5;
        let b1 = 1.0 - cos_w;
        let b2 = (1.0 - cos_w) * 0.5;
        let a0 = 1.0 + alpha;
        let a1 = -2.0 * cos_w;
        let a2 = 1.0 - alpha;
        Self::normalise(a0, b0, b1, b2, a1, a2)
    }

    // RBJ audio cookbook: highpass biquad.
    fn highpass(sr: u32, hz: f32, q: f32) -> Self {
        let omega = 2.0 * PI * hz / sr as f32;
        let alpha = omega.sin() / (2.0 * q);
        let cos_w = omega.cos();
        let b0 = (1.0 + cos_w) * 0.5;
        let b1 = -(1.0 + cos_w);
        let b2 = (1.0 + cos_w) * 0.5;
        let a0 = 1.0 + alpha;
        let a1 = -2.0 * cos_w;
        let a2 = 1.0 - alpha;
        Self::normalise(a0, b0, b1, b2, a1, a2)
    }

    // RBJ audio cookbook: peaking EQ biquad.
    fn peak(sr: u32, hz: f32, q: f32, gain_db: f32) -> Self {
        let a_amp = 10f32.powf(gain_db / 40.0);
        let omega = 2.0 * PI * hz / sr as f32;
        let alpha = omega.sin() / (2.0 * q);
        let cos_w = omega.cos();
        let b0 = 1.0 + alpha * a_amp;
        let b1 = -2.0 * cos_w;
        let b2 = 1.0 - alpha * a_amp;
        let a0 = 1.0 + alpha / a_amp;
        let a1 = -2.0 * cos_w;
        let a2 = 1.0 - alpha / a_amp;
        Self::normalise(a0, b0, b1, b2, a1, a2)
    }

    fn normalise(a0: f32, b0: f32, b1: f32, b2: f32, a1: f32, a2: f32) -> Self {
        Self {
            b0: b0 / a0,
            b1: b1 / a0,
            b2: b2 / a0,
            a1: a1 / a0,
            a2: a2 / a0,
            s1: 0.0,
            s2: 0.0,
        }
    }
}

// PolyBLEP correction for band-limited oscillators. `t` is phase in [0,1),
// `dt` is per-sample phase increment.
fn poly_blep(t: f32, dt: f32) -> f32 {
    if t < dt {
        let x = t / dt;
        x + x - x * x - 1.0
    } else if t > 1.0 - dt {
        let x = (t - 1.0) / dt;
        x * x + x + x + 1.0
    } else {
        0.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pit_quantize_round_trip() {
        // PIT_FREQ / 1140 = 1046.65... → rounds to divisor 1140 → 1046 Hz.
        assert_eq!(pit_quantize(1047), 1193182 / 1140);
        // 440 Hz: divisor = round(1193182/440) = 2712 → PIT/2712 = 439 Hz.
        assert_eq!(pit_quantize(440), 1193182 / 2712);
        // Zero-input guard.
        assert_eq!(pit_quantize(0), 0);
    }
}
