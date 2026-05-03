# CPAL Audio Backend Support (JACK, PipeWire, PulseAudio)

## Task Specification

Add support for JACK, PipeWire, and PulseAudio audio backends in the spkrd server,
using the CPAL (Cross-Platform Audio Library) crate as the foundation. The reference
implementation to study is the `beep` example in ~/src/cpal.

## Status: Complete

## High-Level Decisions

- JACK, PulseAudio, PipeWire added as **optional** Cargo features (not in `default`),
  each requiring the corresponding system library at build time.
- `--cpal-host` kept as-is (accepts strings); help text updated to enumerate valid
  values and document which feature each requires.
- cpal bumped from 0.16 to `>=0.17` with a `[patch.crates-io]` git override pointing
  to the upstream GitHub repo. Reason: PipeWire and PulseAudio are only in cpal 0.18
  (unreleased); the git patch provides 0.18.0 without a machine-specific path.
  The patch section carries a removal note for when 0.18 lands on crates.io.
- cpal 0.18 had three breaking API changes fixed in `cpal_backend.rs`:
  `SampleRate` is now `type alias = u32` (no `.0` / no constructor);
  `build_output_stream` takes `StreamConfig` by value; `device.name()` deprecated →
  `device.description().name()`.

## Requirements Changes

- User clarified: features optional, document `--cpal-host` strings, let cpal
  default_host() handle auto-selection (it already does PipeWire > PulseAudio > ALSA).

## Files Modified

- `Cargo.toml`: bumped cpal to `>=0.17`, added `[patch.crates-io]` git override,
  added `jack`, `pulseaudio`, `pipewire` feature entries with comments.
- `src/cpal_backend.rs`: fixed three cpal 0.18 API changes (SampleRate, StreamConfig,
  device description).
- `src/main.rs`: updated `--cpal-host` help text to enumerate valid strings and
  document feature requirements.
- `Makefile`: added `FEATURES ?=` variable and auto-detection logic in the build
  recipe; probes jack/libpulse/libpipewire-0.3 via pkg-config and enables the
  corresponding Cargo features when the libraries are present. Override with
  `make FEATURES=jack,pulseaudio` to skip auto-detection.
- `README.md`: updated Prerequisites, Build features section (added feature table
  with system library requirements), and `--cpal-host` flag docs.

## Rationales and Alternatives

- **git patch vs path**: `path = "../cpal"` is machine-specific; git URL is portable
  and pins a commit SHA in Cargo.lock for reproducibility.
- **`>=0.17` version req**: allows both 0.17 (crates.io) and 0.18 (git patch) to
  satisfy the requirement. The git patch wins because it provides the newer version.

## Obstacles and Solutions

- cpal 0.17 on crates.io lacks pipewire/pulseaudio features → patched with git URL.
- cpal 0.18 has breaking API changes → fixed in cpal_backend.rs.
- `--features pipewire` build fails with `stdbool.h not found` → system missing
  `libclang-dev`; install with `sudo apt install libclang-dev`. JACK and PulseAudio
  build cleanly without it.

## Current Status

- [x] Explore cpal beep example
- [x] Explore current spkrd server structure
- [x] Ask clarifying questions
- [x] Present implementation plan
- [x] Await approval
- [x] Implement
