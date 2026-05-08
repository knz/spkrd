# CPAL/PulseAudio: stale client after suspend/resume

## Task Specification

User reports: starting `spkrd` works fine, but after suspending the
machine and resuming, all subsequent `PUT /play` requests fail with:

```
ERROR pulseaudio::client::reactor] Reactor error: I/O error
ERROR spkrd::server] CPAL error for request from 127.0.0.1:
       build_output_stream: PulseAudio client disconnected
```

The first error fires once at the resume moment from a background
thread inside the `pulseaudio` Rust crate (cpal's PA backend); every
subsequent `build_output_stream` then fails because the cached
`cpal::Device` still references the now-dead PA `Client`.

## Root cause

`CpalBackend` is constructed once in `main()` and stores a
`cpal::Device`. Under cpal's pulseaudio host, that `Device` carries
an `Arc<pulseaudio::Client>` — a long-lived async client whose
reactor task talks over the unix socket at
`/run/user/$UID/pulse/native`. When the machine suspends, the kernel
keeps the FD open, but on resume the socket transitions in a way
that makes the reactor read return an I/O error. The reactor logs
"Reactor error: I/O error" and exits. After that, every
`build_output_stream` call on the cached `Device` synchronously
errors with `Disconnected` because the request can't reach the dead
reactor. No reconnection logic exists in cpal's PA host today.

## High-Level Decisions

(All confirmed by user 2026-05-08.)

* **Lazy reconnect on disconnect-shaped error**, no background watcher.
  The first request after resume pays a one-time reconnect cost
  (~tens of ms); subsequent requests use the rebuilt Device. No risk
  of fighting PA when it's still genuinely down.
* **Within a single request, retry on the existing 1s cadence until
  `--retry-timeout` elapses.** Mirrors the busy-device retry pattern
  so a transient PA-down state during a request gets the same
  patience as a busy-device state.
* **String-match on "disconnect" (case-insensitive)** to detect
  recoverable errors. cpal's PA backend translates
  `pulseaudio::ClientError::Disconnected` into a `BackendSpecific`
  error with a string message. There is no structured variant to
  match. Other CPAL errors (invalid config, unsupported format)
  remain fatal — preserving fail-fast behavior for genuinely-broken
  configs.
* **General PA-disconnect recovery**, suspend is one trigger.
  pipewire-pulse restart, server crash, or socket-transient errors
  all hit the same path. Suspend/resume is documented as the
  observed trigger.
* **PipeWire/JACK behavior unverified.** The is_disconnect_error
  heuristic is generic, so if those backends emit similar messages,
  the recovery path will fire for them too. If not, no regression
  vs. today.

## Implementation

### Verification of load-bearing assumption

Confirmed in `~/src/cpal/src/host/pulseaudio/mod.rs`:
- `Host::new()` calls `pulseaudio::Client::from_env(...)` — fresh
  client per call, no caching.
- `cpal::host_from_id(...)` and `cpal::default_host()` (in
  `cpal::platform::mod.rs`) both call `<Host>::new()` directly
  without memoization.

So calling `cpal::default_host()` (or `cpal::host_from_id`) again
gives a fresh PA client. The dead reactor stays orphaned in the old
`Host`/`Device` (which we drop), and the new state hands out streams
from a live one.

### Code changes

`src/cpal_backend.rs`:

* `CpalConfig` gets `#[derive(Clone)]` so the backend can hold a
  copy for rebuild.
* New private `DeviceState` struct holds the
  `(device, config, sample_format)` trio that gets replaced on
  reconnect.
* `CpalBackend` restructured:
  - `state: Mutex<DeviceState>` — replaceable.
  - `cfg: CpalConfig` — immutable, used to re-run device selection.
  - `play_lock` unchanged. (Hoisted `volume`/`waveform` fields
    removed; reads come from `cfg` now.)
* New free fn `build_device_state(cfg: &CpalConfig) ->
  Result<DeviceState, _>` factors out host/device selection from
  the old `CpalBackend::new`. Same logic, including the
  `BufferSize::Fixed(sample_rate / 100)` workaround for
  pipewire-pulse (RustAudio/cpal#1190).
* New free fn `log_device_state(...)` for the startup/rebuild info!
  line — same format as before.
* New free fn `is_disconnect_error(msg: &str) -> bool` — matches
  case-insensitive substring "disconnect".
* New method `CpalBackend::rebuild_device()` calls
  `build_device_state` and replaces `*self.state.lock()`.
* `CpalBackend::new` now just constructs initial state and stores it.
* `play_melody` reads sample rate from state, renders the buffer,
  and passes `events` + `buffer` + `initial_sr` to
  `acquire_and_play`. The events are kept around so the buffer can
  be re-rendered if a rebuild changes the sample rate (rare).
* `acquire_and_play` keeps its existing lock-acquisition retry
  loop, then adds a play-attempt retry loop sharing the same
  `start`/`retry_timeout` window. On disconnect-shaped error:
  `warn!`, `rebuild_device()`, log result, sleep `RETRY_INTERVAL`,
  retry. On non-disconnect error: surface immediately.
* `play_buffer` and `run_stream` take a `&DeviceState` (locked once
  in `play_buffer`, held for the duration of run_stream) instead of
  reading `self.device`/`self.config`/`self.sample_format`. State
  lock contention is impossible: `play_lock` already serializes
  plays, and rebuild_device only runs from inside the same
  play_lock-guarded section after `play_buffer` released the state
  guard.
* `run_stream` takes `&[f32]` instead of `Vec<f32>` so retries don't
  consume the buffer; clones into the `Arc<Vec<f32>>` the callback
  needs.

File header comment updated to document PA-disconnect recovery,
including the rationale for keeping the device behind a Mutex.

## Files Modified

* `src/cpal_backend.rs` — primary edit (~140 lines added/changed).
* `changelog/20260508-cpal-pulseaudio-suspend-recovery.md` (this file).

## Trade-offs / Risks

* **String-match on error message is fragile.** If cpal upstream
  changes the disconnect error wording, this stops detecting it. Not
  worth defending against today; a structured cpal::ErrorKind variant
  would be a cleaner upstream fix. Filed in the same area as the
  existing #1188/#1190 conversations.
* **State mutex held across the entire stream playback** (potentially
  several seconds). Acceptable because `play_lock` already serializes
  plays — only one thread is inside `play_buffer` at any time, so the
  state lock is uncontended.
* **Sample-rate change across rebuild is handled defensively**
  (re-render the buffer) but unlikely in practice for same-sink
  reconnects.
* **PipeWire backend untested** for suspend recovery. If its error
  messages contain "disconnect", the same path will fire. If not,
  no regression vs. today (it just won't auto-recover).

## Verification

* `cargo build --release` — clean.
* `cargo test --release` — 11 unit + 4 integration tests pass.
* `cargo clippy --release -p spkrd --lib --bin spkrd` — no new
  warnings introduced (3 pre-existing warnings in mml.rs and the
  synth code remain).
* Runtime verification (suspend the machine, resume, send a PUT
  /play) is left to the user.

## Current Status

Implementation complete. Awaiting user runtime confirmation that
suspend/resume recovery works.
