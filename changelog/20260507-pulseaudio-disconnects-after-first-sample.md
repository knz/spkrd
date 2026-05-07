# PulseAudio backend: client disconnects after first sample

## Task Specification

User reports: a week ago, running `spkrd` with the PulseAudio backend on
this machine "worked fine" — but the user did not realise at the time
that the PulseAudio socket was not being served by a real PulseAudio
daemon, but by `pipewire-pulse` (PipeWire's PA-protocol shim).

This week the user changed sound packages (does not remember which),
and now:

* Running spkrd with `--cpal-host PipeWire`: works fine.
* Running spkrd with `--cpal-host PulseAudio`: sound output fails
  after the first sample, with the following log:
    `[WARN spkrd::cpal_backend] cpal stream error: PulseAudio client disconnected`

Investigate root cause.

## Investigation (so far)

### System state

* PulseAudio socket is provided by `pipewire-pulse 1.0.5-1ubuntu3.2`,
  running at `/run/user/1000/pulse/native`. `pactl info` confirms:
  *Server Name: PulseAudio (on PipeWire 1.0.5)*. The actual PulseAudio
  daemon never ran in the recent journal history (back to May 2);
  `pipewire-pulse[2699]` was already serving the socket on May 2/3.
* Recent apt history (`/var/log/apt/history.log`):
  - 2026-05-01 06:12: large unattended upgrade (kernel 6.17.0-23,
    systemd 255.4-1ubuntu8.15, etc.). No pipewire/pulseaudio packages
    upgraded.
  - 2026-05-03: dev libs installed (`libpulse-dev`,
    `libpipewire-0.3-dev`, `libjack-jackd2-dev`, `clang`).
  - **2026-05-06 17:53: `apt install pipewire-alsa` removed the
    `pulseaudio` package** (status went from `ii` → `rc`). This is
    the most likely candidate for "I changed sound packages".
  - 2026-05-06 19:05: system rebooted.
* `pulseaudio` package removal did **not** purge config; `/etc/pulse/`
  still has `client.conf`, `daemon.conf`, `default.pa`, `system.pa`
  (timestamps from Dec 2024). `~/.config/pulse/cookie` still present
  (from 2016). So cookies/configs aren't missing.
* The `pulseaudio` daemon was already not running before the package
  removal — pipewire-pulse was. So removing the package shouldn't
  have changed the socket-side behaviour. But it timed with the
  reboot.

### spkrd binary

* Cargo.toml pins cpal to `knz/cpal#fix/pulseaudio-stream-leak-1188`
  (commit `3b2475d`). This carries the May 4 leak-fix patch:
  `Stream::drop` now sends `DeletePlaybackStream` via
  `now_or_never()` after `handle.cancel()`. (See changelog
  `20260504-pulseaudio-stream-leak.md`.)
* The cpal pulseaudio backend uses the native Rust `pulseaudio`
  crate `0.3.1` (wire-protocol implementation), not libpulse.

### Log forensics

From `journalctl --user --since "1 hour ago"`:

```
15:15:04 spkrd[186888] ERROR build_output_stream: PulseAudio client disconnected
15:15:11 systemd  Stopped/Started spkrd.service
15:15:11 spkrd[288030] INFO  connecting to PulseAudio server at /run/user/1000/pulse/native
15:15:11 spkrd[288030] INFO  CPAL backend: device="alsa_output.pci-0000_00_1f.3.analog-stereo",
                              sample_rate=48000, channels=2, format=I32, waveform=PcSpeaker
15:15:11 spkrd[288030] INFO  Server listening on 0.0.0.0:1111
15:15:12 spkrd[288030] WARN  cpal stream error: PulseAudio client disconnected
15:15:21..15:16:10 spkrd[288030] WARN  cpal stream error: PulseAudio client disconnected (×14)
```

Observations:

* `default_output_config()` *succeeded* at startup — that requires a
  working PA-protocol round-trip (Auth + SetClientName + GetSinkInfo).
  So the initial socket and handshake are fine.
* The `WARN cpal stream error` line is fired by spkrd's
  `build_output_stream` error callback (cpal_backend.rs:333). That
  callback runs from cpal's pulseaudio backend's spawned threads
  (`play_all` driver thread or latency monitor) when they observe a
  `ClientError::Disconnected` — i.e. the reactor's socket read
  returned `Ok(0)`, meaning **pipewire-pulse closed the connection**.
* Format reported by the device: `I32` (S32Le). `pactl info` reports
  `float32le 2ch 48000Hz` as default sample spec — but
  `default_output_config` queries the *sink's* sample spec, not the
  server default. The Intel HDA sink probably runs S32Le natively;
  pipewire-pulse reports it as such.

### Hypotheses to test

1. **pipewire-pulse rejects `CreatePlaybackStream` and closes the
   client socket** as a fatal protocol response. Most likely trigger:
   the `PlaybackStreamParams` cpal sends — `start_corked: true`,
   `adjust_latency: ?`, buffer_attr values, or sample_spec/channel_map
   combination — is not accepted by pipewire-pulse's PA shim.
2. **The cpal stream-leak patch (3b2475d) tickles a server-side bug
   in pipewire-pulse**: a fire-and-forget `DeletePlaybackStream` sent
   without awaiting the ack might be confusing pipewire-pulse on the
   *next* stream creation. (User's wording "after the first sample"
   suggests the very first play succeeded, and only subsequent plays
   fail — which would fit.)
3. **Stale per-user state from old machine-id**: there's a dangling
   `~/.config/pulse/8e9fd86b...-runtime` symlink to a non-existent
   `/tmp/pulse-PKdhtXMmr18n/`. Unlikely to cause this, but worth
   ruling out.

### What we need from the user

Whether (1), (2), or (3) is correct hinges on:

* Whether the very first PUT /play after a fresh spkrd start
  succeeds, or fails.
* What pipewire-pulse logs at the moment of disconnection (this would
  pinpoint the rejected command). Reproducing with `PIPEWIRE_DEBUG=3`
  on pipewire-pulse and `RUST_LOG=debug` on spkrd would show the
  exact protocol exchange.
* Whether reverting Cargo.toml to the upstream cpal (pre-leak-fix)
  changes the behaviour.

## Round 1 — protocol trace (RUST_LOG=debug, --debug)

User captured a full PA-protocol trace running spkrd manually:

```
CLIENT [1025]: CreatePlaybackStream(... S32Le 2ch 48000Hz, BufferAttr all u32::MAX, start_corked: true, adjust_latency: false ...)
SERVER [1025]: Reply
CLIENT [1026]: CorkPlaybackStream(channel:0, cork:false)
CLIENT [1027]: GetPlaybackLatency
SERVER [1026]: Reply (uncork ack)
SERVER [1027]: Reply
CLIENT [1028]: GetPlaybackLatency
SERVER [1028]: Reply
SERVER [-1]: Started(0)
SERVER [-1]: Request(channel:0, length:16384)   ← server asks for 16384 bytes (~43 ms)
CLIENT [1029]: GetPlaybackLatency
SERVER [1029]: Reply
CLIENT [1030]: DeletePlaybackStream(0)           ← !!! client deletes stream early
SERVER [1030]: Reply
WARN cpal stream error: PulseAudio client disconnected
DEBUG spkrd::server: Request from 127.0.0.1 completed successfully after 0 retries
CLIENT [1031]: DeletePlaybackStream(0)           ← second delete from InnerPlaybackStream::drop
SERVER [1031]: Error(NoEntity)
```

Key conclusions:

* **PA wire protocol is healthy.** Server replies correctly to every
  command. pipewire-pulse never logs an error during the test. The
  cpal/pulseaudio crate's reactor is still alive; no real socket
  disconnect occurs.
* **The "PulseAudio client disconnected" WARN is misleading.** It is
  fired by cpal's `play_all` driver thread when its `source_eof()`
  resolves with `Err(Cancelled)` after the server acks the FIRST
  delete and the reactor drops `eof_tx`. That maps to
  `ClientError::Disconnected` even though the server is fine.
* **The two deletes are by-design redundancy.** [1030] is from cpal's
  patched `Stream::drop` (`stream.clone().delete().now_or_never()`).
  [1031] is from `pulseaudio::InnerPlaybackStream::Drop` firing when
  the last `Arc<InnerPlaybackStream>` clone (held by `play_all` /
  latency threads after they exit) goes away. The second one returns
  `NoEntity`; harmless.

* **The actual bug: spkrd dropped the cpal Stream after only ~43 ms
  of audio (one Request worth of writes).** The rendered "abc"
  buffer is ~72,000 mono samples (1.5 s @ 48 kHz); only one
  `Request(16384)` was processed before the client deleted the
  stream. The server never had a chance to ask for the rest.

User confirmed audibly: a brief click then silence — consistent with
a single 43 ms write reaching the speakers before the stream was
deleted.

## Why did spkrd drop the stream early?

`run_stream` waits on a condvar `done` (cpal_backend.rs:347–351) and
only proceeds to `drop(stream)` after `done` is set. `done` is set in
exactly two places (cpal_backend.rs:323–330 and 332–338):

1. **data_callback** when `aborted || *idx >= total`.
2. **error_callback** unconditionally when fired by cpal.

For (1) to trigger after one Request, `*idx` is only ~2048 (out of
72000) — so `*idx >= total` cannot be true yet, leaving only
`aborted == true`. `abort` is set by `AbortOnDrop::drop`
(cpal_backend.rs:107) which fires when the `play_melody` future is
dropped — i.e. when **axum cancels the request handler**, typically
because the HTTP client closed the connection.

For (2), the error_callback's WARN appears in the trace **after**
[1030], but log ordering across threads is not strictly
real-time-preserving with env_logger. So we cannot rule out the
error_callback path from the log alone.

User uses `examples/client.rs` (Rust spkrc), which uses
`reqwest::Client::new()` with no explicit timeout. That client
*should* hold the connection open for the full duration. So if
`abort` did get set, something else is dropping the future.

## Decisions

* Add temporary diagnostic logging in spkrd's cpal_backend.rs to
  identify the exact path that signals `done`. User approved
  (2026-05-07). Specifically:
  - `AbortOnDrop::drop` — log when abort is set.
  - data_callback — log every invocation with idx/total/aborted, and
    explicitly log which condition signaled done.
  - error_callback — log when it fires (existing WARN already does).
  - run_stream — log build/play/wait-exit/drop boundaries.

## Round 2 — additional data point from user

User reported (2026-05-07): with `--cpal-host pipewire` (libpipewire
backend) the same Rust client and same melody plays correctly. So:

* The HTTP client (`examples/client.rs` / reqwest) is **not**
  closing the connection prematurely (otherwise pipewire backend
  would also fail).
* Therefore axum is not cancelling the handler future, and
  `AbortOnDrop::drop` should not have set `abort=true` during
  `run_stream`.
* The `done` condvar must have been signalled by the
  **error_callback** path — but the error_callback's WARN appears
  in the trace *after* the [1030] DeletePlaybackStream. Either the
  log lines are out of real-time order across threads (env_logger
  doesn't guarantee that), or there is another path I haven't
  identified.

This points to a circular puzzle:
  error_callback → done → drop(stream) → DeletePlaybackStream →
  source_eof errors → error_callback. Something must error first.

## Files Modified

* `changelog/20260507-pulseaudio-disconnects-after-first-sample.md` (this file).
* `src/cpal_backend.rs` — diagnostic logging (DIAG 20260507):
  - `AbortOnDrop::drop` now logs at debug! when abort is set.
  - data_callback logs every invocation with `out_samples`,
    `idx_before/after`, `total`, `aborted`, `signal_done`. When done
    is signalled, an explicit reason (aborted vs idx>=total) is
    logged.
  - error_callback adds a debug! after setting done, recording
    whether done was already set.
  - run_stream logs build/play/wait-exit/about-to-drop boundaries.

## Round 3 — diagnosis confirmed

Trace from instrumented build (2026-05-07T14:00:57Z):

```
run_stream: building stream, total=72000 mono samples, channels=2
CLIENT [1025]: CreatePlaybackStream(... BufferAttr all u32::MAX ...)
SERVER [1025]: Reply
run_stream: build_output_stream OK, calling stream.play()
data_callback: out_samples=192000 idx=0->72000 total=72000 aborted=false signal_done=true
data_callback: setting done=true (reason: idx>=total)
CLIENT [1026]: CorkPlaybackStream(cork:false)
... etc
```

Smoking gun: the very first `data_callback` is invoked with
`out_samples=192000` (= 96000 stereo frames = **2 seconds of audio**)
*before* the stream is uncorked. A single huge call exhausts the
72000-sample melody buffer (idx 0→72000) plus 24000 frames of
trailing silence and signals `done=true` via the legitimate
`idx>=total` path. `run_stream` then sleeps 50 ms and drops the
stream; `Stream::drop` sends `DeletePlaybackStream`, which races
the actual ALSA-side playback. pipewire-pulse discards everything
it hasn't already shipped to the kernel — the audible "click" is
just the small fraction (~one Request worth, 16384 bytes ≈ 43 ms)
that already made it.

Root cause located in the `pulseaudio` crate's reactor
(`client/reactor.rs:127`): when `CreatePlaybackStreamReply` arrives,
the server-supplied `requested_bytes` is seeded into
`PlaybackStreamState.requested_bytes` and the reactor's
`write_streams` immediately asks the source for that many bytes.
With `BufferSize::Default`, cpal's pulseaudio backend sends
`BufferAttr { all u32::MAX }` (= "server pick"), and pipewire-pulse
picks a 2-second initial pre-buffer. Real PulseAudio happens to
pick a much smaller default which is why this stayed latent there.

## Why "a week ago worked"

The user's perception was approximately right — the prior runs
likely had longer melodies where the cutoff was less audibly
obvious — but the 50 ms tail in `run_stream` was always wrong for
the cpal+pulseaudio path on pipewire-pulse. Removing the
`pulseaudio` package and rebooting on 2026-05-06 didn't change
anything functionally (pipewire-pulse was already serving the
socket). The bug class has been latent the whole time.

## Upstream issue filed

Filed as RustAudio/cpal#1190:
*"PulseAudio backend: short / fixed-length playbacks truncated
under pipewire-pulse"*. Body covers mechanism, reproduction, the
`BufferSize::Fixed` workaround, and three suggested fixes
(default sensible BufferAttr / expose Stream::drain() /
drain-on-drop). Author offered to PR fix (1).

## Fix applied

User chose Option 1 (BufferSize::Fixed in spkrd, the smallest
change with the best risk/reward profile of the four discussed).

`CpalBackend::new` now sets
`stream_cfg.buffer_size = BufferSize::Fixed(sample_rate / 100)` —
~10 ms callback period, which `make_playback_buffer_attr` (cpal
pulseaudio mod.rs:557+) translates to a `BufferAttr` with
`target_length = 2 * frame_count` (~20 ms drain) and
`minimum_request_length = frame_count` (~10 ms request size).
pipewire-pulse honors the explicit BufferAttr and delivers
`Request` commands in 10 ms chunks rather than as one ~2 s
pre-buffer, restoring the periodic-callback pattern other cpal
backends already follow. The existing 50 ms tail in `run_stream`
covers the ~20 ms drain comfortably.

Diagnostic logging from round 2 (DIAG 20260507) was removed in the
same edit — `AbortOnDrop::drop`, the data_callback log lines, the
error_callback debug log, and the run_stream boundary logs. The
unused `trace` import was dropped.

## Files Modified (final)

* `changelog/20260507-pulseaudio-disconnects-after-first-sample.md`
* `src/cpal_backend.rs`:
  - Imports: `BufferSize` added; `trace` removed.
  - `CpalBackend::new`: explicit `stream_cfg.buffer_size =
    BufferSize::Fixed(sample_rate / 100)` with comment explaining
    why and pointing to RustAudio/cpal#1190.
  - `info!` startup line now includes `buffer_size` field.
  - All round-2 diagnostic logging removed.

## Verification

* `cargo build --release` clean.
* `cargo test --release` — 4/4 integration tests pass.
* Manual runtime verification with `--cpal-host pulseaudio` and a
  multi-second melody is left to the user.

## Round 4 — false-positive WARN during cleanup

After Round 3 fix, playback worked correctly but
`[WARN spkrd::cpal_backend] cpal stream error: PulseAudio client
disconnected` still appeared on every successful playback.

Root cause: cpal's pulseaudio backend has two spawned threads.
The latency thread checks `handle.cancel` before each timing_info
poll and exits cleanly when set. The `play_all` driver thread
does not — it parks in `source_eof().await`, and when
`Stream::drop` queues `DeletePlaybackStream` the reactor drops
the source's `eof_tx`, surfacing as `ClientError::Disconnected`.
That gets translated to "PulseAudio client disconnected" by the
cpal pulseaudio backend's `From<ClientError>` impl and fires the
user's error_callback. So every clean teardown looks like a
disconnect.

User chose Option 2 (patch cpal) over Option 1 (suppress in
spkrd). Patch added in the fork:

* `~/src/cpal/src/host/pulseaudio/stream.rs`: clone
  `handle.cancel` before the `play_all` spawn; after `play_all`
  returns Err, skip `emit_error` if cancel is set. Mirrors the
  latency thread's existing pre-poll cancel check.
* `~/src/cpal/CHANGELOG.md`: new "Fixed" bullet under PulseAudio.
* Committed on `fix/pulseaudio-stream-leak-1188` as `2c8009c`,
  pushed to `knz/cpal`.
* spkrd: `cargo update -p cpal` repointed Cargo.lock from
  `9a2c03c1` → `2c8009c1`. Build clean, 4/4 integration tests
  pass.

## Current Status

Both spkrd-side (BufferSize::Fixed in CpalBackend::new) and
cpal-side (suppress cancel-induced disconnect WARN) fixes shipped.
Awaiting user runtime confirmation that the WARN no longer fires
on successful playback.

Upstream trackers:
- RustAudio/cpal#1188 — stream leak (existing, leak-fix patch).
- RustAudio/cpal#1190 — short-playback truncation under
  pipewire-pulse (filed by user 2026-05-07).
- The new cancel-suppress patch is currently only on the fork; a
  separate upstream PR (or an addition to #1188's PR) would be
  needed to land it. User's call.
