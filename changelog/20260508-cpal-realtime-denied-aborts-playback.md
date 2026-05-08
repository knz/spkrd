# CPAL: RealtimeDenied error aborts playback after first sample

## Task Specification

User reports: with the PipeWire backend, every melody plays only the
first sample (or a tiny chunk) and then goes silent. Server log:

```
[INFO  spkrd] Starting spkrd: ... output=Auto (resolved=Cpal), ...
[INFO  spkrd::cpal_backend] CPAL backend: device="output_default", sample_rate=48000, channels=2, format=F32, buffer_size=Fixed(480), waveform=PcSpeaker, volume=0.25
[INFO  spkrd::server] Server listening on 0.0.0.0:1111
[DEBUG spkrd::cpal_backend] Request from 127.0.0.1: melody=abc
[WARN  spkrd::cpal_backend] cpal stream error: Failed to promote audio thread to real-time priority: AudioThreadPriorityError: Thread promotion error ("Operation not permitted"): "Operation not permitted"
[DEBUG spkrd::server] Request from 127.0.0.1 completed successfully after 0 retries
```

The server reports success but no audio actually plays past the first
buffer chunk.

## Investigation

### Backend

`ldd target/release/spkrd` shows `libpipewire-0.3.so.0` linked — the
binary was built with `--features pipewire`. So the cpal host in use
is the native PipeWire host (`cpal::host::pipewire`), not pulseaudio
or alsa.

### Where the RT promotion happens

In `~/src/cpal/src/host/pipewire/device.rs:594` (and similarly at
:431 for input), inside the spawned `pw_out` thread, **after** the
mainloop is created and before `mainloop.run()` is called:

```rust
#[cfg(feature = "realtime")]
if let Err(e) = audio_thread_priority::promote_current_thread_to_real_time(
    device.quantum,
    device.rate,
) {
    emit_error(&error_callback_rt, Error::from(e));
}
mainloop.run();
```

The `audio_thread_priority` crate calls into RTKit over DBus on
Linux. RTKit denies promotion for processes that don't satisfy its
policy (often the case for command-line/daemon processes vs. desktop
sessions). The user's "Operation not permitted" matches that path.

In `~/src/cpal/src/error.rs:189`, the From impl translates this into
a cpal `Error` with `ErrorKind::RealtimeDenied`. The doc on that
variant explicitly states: *"Audio will still play, but may be
subject to increased latency or glitches under load."* — i.e.
non-fatal.

### The bug in spkrd

`src/cpal_backend.rs:370-376`:

```rust
move |err| {
    warn!("cpal stream error: {}", err);
    let (lock, cv) = &*err_done;
    let mut d = lock.lock().unwrap();
    *d = true;
    cv.notify_all();
},
```

The error callback unconditionally sets `done = true` and notifies
the condvar — for **every** error, including non-fatal ones. The main
thread in `run_stream` (line 386-389) is waiting on that condvar:

```rust
while !*d {
    d = cv.wait(d).unwrap();
}
drop(d);
// ... 50ms tail sleep ...
drop(stream);
```

When PipeWire fires the RealtimeDenied error during stream startup
(it happens just after the data callback is wired up but very early
in playback), the wait returns, we sleep 50ms, and drop the stream.
PipeWire only delivered the first ~10–50 ms of audio before being
torn down — exactly the "first sample then nothing" symptom.

### Summary of root cause

Spkrd treats every error from the cpal stream error callback as
end-of-stream. cpal 0.18 exposes a structured `ErrorKind` enum with
several non-fatal variants (`RealtimeDenied`, `Xrun`,
`DeviceChanged`). Treating those as fatal short-circuits playback.

## High-Level Decisions

(All confirmed by user 2026-05-08.)

* **Three-bucket classification by `cpal::ErrorKind`**, not by
  message substring. cpal 0.18 exposes structured `ErrorKind`; the
  Display-string heuristic ("disconnect") was fragile and could not
  distinguish RealtimeDenied (which says "playback continues") from
  StreamInvalidated (which doesn't). The buckets:
  - **Continues** = `RealtimeDenied`, `Xrun`, `DeviceChanged` —
    every variant whose docstring says playback continues.
  - **Disconnect** = `StreamInvalidated`, `DeviceNotAvailable`,
    `HostUnavailable` — the kinds the PA host maps disconnect/Io
    to, plus their PipeWire equivalents.
  - **Fatal** = everything else.
* **Replace substring disconnect detection with kind matching.** The
  prior `is_disconnect_error(&str)` is removed; `acquire_and_play`
  now matches on a new `SpeakerError::CpalDisconnect` variant.
* **Surface fatal callback errors back to `play_buffer`** via a
  shared `Mutex<Option<(ErrorClass, String)>>` slot. Today even a
  truly fatal error from the stream-error callback returned `Ok(())`
  to the caller because the wait completed and the error was never
  inspected; the new path checks the slot after the wait and
  converts the captured class to the right SpeakerError variant.

## Implementation

### Code changes

`src/error.rs`:
* New `SpeakerError::CpalDisconnect(String)` variant + Display +
  module header doc explaining the split.

`src/cpal_backend.rs`:
* New `enum ErrorClass { Continues, Disconnect, Fatal }`.
* New `fn classify_error(&cpal::Error) -> ErrorClass` matching on
  `err.kind()`.
* New `fn classify_to_speaker_error(&cpal::Error, ctx) ->
  SpeakerError` for synchronous build/play call sites
  (Continues-class errors are folded into Fatal here because the
  synchronous path can't choose to keep running).
* `run_stream` error callback rewritten:
  - Calls `classify_error` first.
  - On Continues: `warn!` only — does NOT touch `done` or the
    error slot. Lets the data callback drive completion.
  - On Disconnect/Fatal: stores `(class, msg)` into
    `Mutex<Option<...>>`, sets `done = true`, notifies cv.
* `run_stream` post-wait: takes the error slot before the flush
  tail; if present, drops the stream and returns the matching
  variant. Otherwise behaves as before (50ms tail + drop + Ok).
* `build_output_stream` and `stream.play()` map_err call sites
  switched from raw `SpeakerError::CpalError(format!(...))` to
  `classify_to_speaker_error(...)` so a host-down condition at
  build time can also retry via the same `acquire_and_play` loop.
* `acquire_and_play` retry arm: matches on
  `SpeakerError::CpalDisconnect(msg)` instead of the prior
  `SpeakerError::CpalError(msg) if is_disconnect_error(&msg)`.
* `is_disconnect_error` removed.
* Added unit test `classify_error_buckets` covering one variant
  per bucket (RealtimeDenied/Xrun/DeviceChanged for Continues;
  StreamInvalidated/DeviceNotAvailable/HostUnavailable for
  Disconnect; PermissionDenied/UnsupportedConfig/BackendError/Other
  for Fatal).
* File-header comment updated to describe the three-bucket
  classification and to call out the original RealtimeDenied bug
  for context.

`src/server.rs`:
* New match arm for `SpeakerError::CpalDisconnect` mirroring the
  existing `CpalError` arm (500 + "CPAL disconnect: ..." body).
  Reaching this case means `acquire_and_play` exhausted
  `--retry-timeout` rebuilding the device.

## Files Modified

* `src/error.rs` — new variant, header doc.
* `src/cpal_backend.rs` — primary edit. Classifier + reworked error
  callback + classify_to_speaker_error call sites + unit test.
  Module header comment updated.
* `src/server.rs` — new match arm.
* `changelog/20260508-cpal-realtime-denied-aborts-playback.md`
  (this file).

## Trade-offs / Risks

* **`BackendError` and `Other` are now always fatal.** Previously
  they could fall into the disconnect retry path *only if* their
  Display contained "disconnect". After this change they're always
  fatal. This is a strict tightening — no scenario that retried
  before stops retrying, because PA's actual disconnect path maps
  to `StreamInvalidated` (not `BackendError`).
* **First-error-wins in the slot.** If the stream emits multiple
  errors in flight (rare), only the first non-Continues one is
  reported to the caller. Subsequent ones are still logged via
  `warn!`. This matches the prior behavior of "first wakeup wins"
  at the condvar.
* **No upstream cpal change.** ErrorKind is already public in
  cpal 0.18 (the version pinned via the patch in Cargo.toml).
* **Continues-class errors at synchronous call sites** (build/play)
  are mapped to Fatal rather than swallowed. The synchronous path
  can't continue past a build failure; if cpal ever surfaces e.g.
  `Xrun` from `stream.play()` (it doesn't today), this would treat
  it as fatal. Acceptable given today's behavior of those sites.

## Verification

* `cargo build --release --features pipewire` — clean.
* `cargo test --release --features pipewire` — 12 unit tests
  (added 1: `classify_error_buckets`) + 4 integration tests pass.
* `cargo clippy --release --features pipewire -p spkrd --lib --bin
  spkrd` — 3 pre-existing warnings remain (mml.rs ×2,
  cpal_backend.rs:682 `repeat().take()` in synth_pcspeaker — all
  untouched code). No new warnings from this change.
* Runtime verification (re-run with PipeWire backend, confirm
  melodies play to completion despite the same RealtimeDenied
  warning still appearing in the log) is left to the user.

## Current Status

Implementation complete. Awaiting user runtime confirmation that
melodies now play to completion under PipeWire when RTKit denies
real-time priority promotion.
