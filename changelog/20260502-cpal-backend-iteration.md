# CPAL backend iteration

## Task Specification

The user wants to iterate on the recently added local-melody-playback
feature. The relevant staged changes (not yet committed) are:

- `src/mml.rs` (new): Rust port of FreeBSD `spkr.c`'s MML
  interpreter. Produces `Event::Tone { freq_hz, centisecs }` /
  `Event::Rest { centisecs }` sequences from a melody string.
- `src/cpal_backend.rs` (new): synthesises an MML melody to PCM via
  one of several waveforms (Square, SquareBandlimited, Sine,
  Triangle, Sawtooth) and plays it through CPAL. Uses an async mutex
  to enforce one-melody-at-a-time and reuses the
  retry-on-busy/timeout convention. Wraps the blocking CPAL stream in
  `spawn_blocking`.
- `src/freebsd_speaker.rs` (new file but old behaviour): the original
  `/dev/speaker` writer extracted out of the previous `speaker.rs`.
- `src/lib.rs`: exposes the new modules.
- `src/main.rs`: adds `--output` (auto/freebsd-speaker/cpal),
  `--waveform`, `--volume`, `--sample-rate`, `--cpal-host`,
  `--cpal-device`. `auto` falls back to CPAL when the device path is
  missing. Warns about flags that don't apply to the resolved
  backend.
- `src/server.rs`: dispatches between backends through a `Backend`
  enum.
- `src/error.rs`: adds `CpalError(String)`.
- `Cargo.toml`: adds the `cpal = "0.16"` dependency.
- `tests/integration_tests.rs`: updated to use the new
  `Backend::FreebsdSpeaker` shape.

The user has not yet specified the direction of the iteration. Open
clarifying questions are recorded below, awaiting answers before any
implementation plan is drafted.

## Iteration scope (user-specified)

Three concrete deliverables for this round:

1. Add a history-attribution header to `src/mml.rs` pointing at the
   original FreeBSD `spkr.c` (sources are in `src/fbsd-speaker/`).
2. Put the CPAL backend behind a Cargo feature flag. The feature is
   on by default, but can be disabled at build time. Document this
   in README and recommend disabling on FreeBSD targets (where the
   real /dev/speaker is available).
3. Since the project is no longer FreeBSD-only, add a short
   description of the MML melody input language to README so users
   not familiar with the FreeBSD speaker driver can use it.

## Decisions

- Cargo feature is named `cpal`; default-on. `dep:cpal` makes the
  optional dep purely a feature gate (no implicit feature with the
  same name as the dep).
- Asymmetric gating: only `cpal_backend` and the cpal dependency are
  feature-gated. `freebsd_speaker` is always compiled — it has no
  platform-specific deps, so the savings of gating it would be
  negligible.
- `auto` resolution when `cpal` is disabled: if the device path is
  missing, fail at startup with a clear message (rather than
  resolving to `freebsd-speaker` and erroring per-request, which
  would be noisier and harder to diagnose).
- `--output=cpal` and the cpal-only flags are hidden from `--help`
  when the feature is disabled (via `#[cfg(feature = "cpal")]` on
  each `#[arg]` field and on the enum variant). The `--output` help
  text also varies based on the feature.
- README change scope: kept narrow. The MML "Quick Melody Syntax
  Reference" already exists, so we only updated the project
  framing (no longer FreeBSD-only), added the Backends paragraph,
  and a Build features section documenting the feature flag and
  recommending `--no-default-features` on FreeBSD targets.
- Attribution in `mml.rs`: short two-paragraph note crediting the
  `spkr.c` lineage (Raymond v1.4 1993, Chernov FreeBSD port),
  with a license-compatibility note (BSD-2-Clause both sides).

## Files Modified

- `Cargo.toml`: `cpal` becomes optional; new `[features]` table with
  `default = ["cpal"]` and `cpal = ["dep:cpal"]`.
- `src/lib.rs`: gate `pub mod cpal_backend;` behind the feature;
  refresh header comment.
- `src/error.rs`: gate `CpalError(String)` variant + Display arm.
- `src/server.rs`: gate `CpalBackend` import, `Backend::Cpal`
  variant, dispatch arm, error-mapping arm, and the now-conditional
  `Arc` import.
- `src/main.rs`: gate the CPAL imports, `WaveformArg`, the
  `OutputMode::Cpal` variant, the five cpal-only CLI flags, the
  cpal arms in `resolve_output`/`warn_unused_flags`/`build_backend`,
  the `Arc` import, and the `warn` import. New const `OUTPUT_HELP`
  switches the `--output` help text based on feature state. New
  startup check that fails loudly if `--output=auto` is used with
  no device and the cpal feature is off.
- `src/mml.rs`: replaced top-of-file comment with attribution +
  license note. Behaviour unchanged.
- `README.md`: retitled (no longer FreeBSD-only); added Backends
  paragraph; added Build features section documenting the `cpal`
  feature and the FreeBSD recommendation; expanded the flag lists
  with `--output` plus the cpal-only flags as a clearly-marked
  conditional group.

Not modified:
- `src/cpal_backend.rs`, `src/freebsd_speaker.rs`,
  `src/qemu_pcspeaker.md`, `src/fbsd-speaker/spkr.c`,
  `tests/integration_tests.rs`, `API.md`.

## Verification

- `cargo build` (default features) → clean.
- `cargo build --no-default-features` → clean (no warnings).
- `cargo test` (default features) → 14 tests pass (10 mml unit + 4
  integration).
- `cargo test --no-default-features` → 14 tests pass.
- `./spkrd --help` shows the cpal flags; `./spkrd --help` (no
  default features) hides them and the `--output` possible values
  list contains only `auto, freebsd-speaker`.

## Current Status

Done. Awaiting the user's call on whether to commit.

## High-Level Decisions

(none yet)

## Files Modified

(none yet — changelog only)

## Current Status

Awaiting user clarification on which direction to take.
