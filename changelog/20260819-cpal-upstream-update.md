# Move off the cpal fork to upstream cpal

## Task Specification

The `[patch.crates-io]` entry in `Cargo.toml` pins `cpal` to the
user's fork `knz/cpal`, branch `fix/pulseaudio-stream-leak-1188`
(locked at `fb8131a`). The user believes that branch carries the
changes of upstream PR RustAudio/cpal#1189, which has since merged;
upstream cpal has evolved further.

Request: pick up the latest upstream cpal and check whether
everything still works.

User follow-ups during investigation:

- cpal now has an API to wait for playback completion (stream drain).
- The PR that introduced it is RustAudio/cpal#1258.

## Findings

### What the fork carries, and where it landed upstream

Fork branch = upstream `383eb60` + 3 commits:

| fork commit | subject | upstream status |
|---|---|---|
| `7a2b05e` | release server-side stream on `Stream::drop` | in `cc1d221` (PR #1189) |
| `3fecb31` | join worker threads on `Stream::drop` | in `cc1d221` (PR #1189) |
| `fb8131a` | suppress disconnect error during teardown | present upstream (`cancel_driver` check in `pulseaudio/stream.rs`) |

All three are present on upstream `stable-0.18` and on `master`.
The fork is therefore fully redundant and the `[patch.crates-io]`
entry can be dropped.

### Release status

- **cpal 0.18.2** published to crates.io on 2026-08-16 (three days
  ago). Contains all three fork fixes. No `[patch]` needed.
- **cpal 0.19.0** is unreleased — `master`/`develop` only. Consuming
  it means keeping a git dependency, just repointed from the fork to
  `RustAudio/cpal`.

### The drain API (PR #1258, merged 2026-07-10, in 0.19.0 only)

`StreamTrait` gains `start` / `pause` / `stop(timeout)`:

- `pause` halts immediately, possibly discarding buffered audio.
- `stop(Some(d))` **drains** — blocks the caller until the device has
  played out queued frames, or `d` elapses. `None` waits indefinitely.
- Dropping a stream still halts immediately without draining.
- `play` is deprecated, forwards to `start`.

PR #1258 explicitly lists **issue #1190 as fixed**. #1190 is the
pipewire-pulse truncation bug that spkrd works around today with
`BufferSize::Fixed(sample_rate / 100)` in `build_device_state` plus
the unconditional 50 ms `thread::sleep` tail in `run_stream`.

### 0.19.0 breaking changes that touch spkrd

From upstream `UPGRADING.md` / `CHANGELOG.md`:

1. `ErrorKind::Xrun` **removed**; xruns now surface via
   `CallbackInfo::xrun()` in the data callback. `classify_error` in
   `cpal_backend.rs` matches on `ErrorKind::Xrun`, and
   `classify_error_buckets` asserts on it — both must change.
2. `InputCallbackInfo`/`OutputCallbackInfo` merged into
   `CallbackInfo`. The data callback closure is typed
   `&cpal::OutputCallbackInfo` today.
3. `StreamTrait::play` → `start` (deprecated alias still compiles).
4. `DeviceTrait`/`StreamTrait` now require `Send + Sync` — affects
   custom host implementors only, not spkrd.
5. Rust edition 2024, MSRV 1.85. Local toolchain is 1.95; CI uses
   `stable`. Not a blocker.

Feature names spkrd forwards (`cpal/jack`, `cpal/pulseaudio`,
`cpal/pipewire`) still exist on master.

## Decision

Two routes were put to the user:

- **A — crates.io 0.18.2, drop the patch.** Delete
  `[patch.crates-io]`, require `cpal 0.18.2`. No source changes.
  Keeps the `BufferSize::Fixed` + 50 ms sleep workaround for #1190.
  No git dependency.
- **B — upstream master (0.19.0-dev), git dependency.** Gets the
  drain API; lets `run_stream` replace the 50 ms sleep with
  `stream.stop(Some(..))`, and possibly revert `BufferSize::Fixed`
  back to `BufferSize::Default`. Costs: unreleased dependency, plus
  the breaking-change edits listed above.

**User chose A.** Rationale: 0.19.0 is unreleased, so B would only
trade one git dependency for another. The drain migration is
deferred until 0.19 ships, at which point the `BufferSize::Fixed`
workaround and the sleep tail should be revisited together.

A third option ("0.18.2 now, 0.19 later") was offered but is the same
code change as A; it only differed in stated follow-up intent.

## Files Modified

- `Cargo.toml`: removed the `[patch.crates-io]` block pointing at
  `knz/cpal`; changed the `cpal` requirement from the open `>=0.17`
  bound (which only existed so the patch could win) to `0.18.2`, the
  first crates.io release carrying #1189.
- `src/cpal_backend.rs`: updated the stale comment in
  `build_device_state` that described #1190 as merely "tracked
  upstream". It is now closed by #1258, but that fix is 0.19.0-only,
  so the workaround and the matching flush sleep in `run_stream`
  both stay until 0.19 ships.
- `Cargo.lock`: regenerated via `cargo update -p cpal`.
- `changelog/20260819-cpal-upstream-update.md` (this file).

## Obstacles and side effects

### Dropped transitive dependencies — `realtime` is no longer default

The fork branched from `383eb60`, which was after upstream `dccd6b4`
made `realtime` a default feature. Upstream later reverted that
(`f0f3df0`, no rationale in the commit message), so cpal 0.18.2 ships
`default = []` where the fork had `default = ["realtime-dbus"]`.

Consequence: spkrd was implicitly building cpal with `realtime-dbus`
and is no longer. `Cargo.lock` confirms `audio_thread_priority`,
`dbus`, and `libdbus-sys` are gone.

Effects, in both directions:

- The audio callback thread is no longer promoted to real-time
  priority via rtkit. Under load this makes xruns more likely.
- `libdbus-1` is no longer a build or runtime dependency. CI installs
  `libdbus-1-dev` (`.github/workflows/rust.yml`) purely for this and
  could drop it.
- `ErrorKind::RealtimeDenied` should no longer fire at all, since no
  RT promotion is attempted. That was the trigger for the earlier bug
  fixed in `20260508-cpal-realtime-denied-aborts-playback.md`; the
  `Continues` classification for it stays in place regardless.

Left as-is pending the user's decision on whether to re-enable
`cpal/realtime-dbus` explicitly.

### PipeWire build initially failed

`cargo build --features pipewire` failed in `libspa-sys`'s build
script — `libpipewire-0.3.pc` absent. Verified pre-existing by
stashing the change and reproducing the identical failure at `HEAD`.
The user then installed the dev package (libpipewire 1.6.2) and the
build succeeded.

Note `cargo update -p cpal` also moved `pipewire` 0.9.2 -> 0.10.1
(and `libspa` likewise), pulled in by cpal 0.18.2.

## Verification

Build / lint / test, all clean:

- `cargo build --release` (default features)
- `cargo build --release --features pulseaudio`
- `cargo build --release --features pipewire`
- `cargo build --release --features jack`
- `cargo build --release --features pulseaudio,pipewire`
- `cargo build --no-default-features`
- `cargo test`: 25 unit + 4 integration tests pass
- `cargo clippy --all-targets` with default / `pulseaudio` /
  `pipewire`: no warnings in `cpal_backend.rs`. The only warnings are
  6 pre-existing `redundant_field_names` in the `client` example.
- `Cargo.lock` resolves `cpal 0.18.2` from
  `registry+https://github.com/rust-lang/crates.io-index`; no `git+`
  sources remain anywhere in the lockfile.

Runtime smoke tests on the developer machine (pipewire-pulse,
48 kHz), all three Linux hosts, `--waveform pc-speaker`:

| host | device | format | result |
|---|---|---|---|
| PulseAudio | Ryzen HD Audio Controller Speaker | I32 | audible, HTTP 200 |
| PipeWire | default_output | F32 | audible, HTTP 200 |
| ALSA | Default Audio Device | F32 | audible, HTTP 200 |

Regression checks against the bugs the fork existed to fix:

- **Stream leak (#1189):** `pactl list short sink-inputs` held flat
  at its baseline of 2 across 14 consecutive plays on the PulseAudio
  host. No accumulation.
- **Stuck driver threads:** process thread count went 18 -> 19 after
  the first batch of plays and then stayed at 19 across 11 more —
  a one-off tokio blocking-pool thread, not a per-play leak.
- **Spurious teardown warnings (fork commit `fb8131a`):** no `WARN`
  or `ERROR` lines in any of the three hosts' logs. In particular no
  "PulseAudio client disconnected" on clean teardown.
- Request wall time (~2.45 s for `T200 O4 cdefgab>c`) matches the
  melody duration, so playback is not being truncated.

Not verified: suspend/resume and pipewire-pulse-restart recovery
(the `rebuild_device` path from
`20260508-cpal-pulseaudio-suspend-recovery.md`). That needs a real
suspend cycle.

## Current Status

Change complete and verified. Two follow-ups left open for the user:

1. Whether to re-enable `cpal/realtime-dbus` explicitly, or accept
   upstream's new default of no RT promotion (and correspondingly
   drop `libdbus-1-dev` from CI).
2. Revisit the drain migration (`stop(timeout)`, `BufferSize`
   workaround, sleep tail) once cpal 0.19.0 is released.
