# 20260503 — Configurable Melody Maximum Length

## Task Specification

Original request: "make the melody max length configurable server-side".

The melody length limit is currently hard-coded to 1000 bytes in three
places in the source tree:

- `src/freebsd_speaker.rs:48` (active path)
- `src/cpal_backend.rs:366` (active path, gated behind the `cpal` feature)
- `src/speaker.rs:46` (appears to be an older / unused module — needs
  verification)

The error message ("Melody exceeds 1000 characters") and the integration
test in `tests/integration_tests.rs:63,78` also reference the constant.
The goal is to expose this as a runtime configuration option on the
server CLI so operators can raise or lower the bound without recompiling.

## High-Level Decisions

Confirmed with the user 2026-05-03:

1. CLI flag is `--max-melody-length`, value in bytes, default `1000`
   (preserves current behaviour).
2. The limit is always active — `0` is rejected at startup; there is no
   "unlimited" mode.
3. The server enforces a hard ceiling of 1 MiB (1,048,576 bytes) at
   startup; values above that are rejected before the runtime starts.
4. The validation error message reports the configured limit
   dynamically (e.g. "Melody exceeds 4096 bytes").
5. `src/speaker.rs` is dead code (not declared in `lib.rs`, not
   referenced anywhere) and will be deleted as part of this change.
6. Scope is server-only; example clients are not modified.
7. Validation error message changes wording from "characters" to "bytes"
   (more accurate since the count is UTF-8 byte length).
8. Stale Emacs autosave file `src/#cpal_backend.rs#` is also deleted.

## Current Status

Plan approved 2026-05-03. Implementing.

## Files Modified

- `src/main.rs` — added `--max-melody-length <BYTES>` flag (default
  1000), startup validation rejecting `0` and values above the
  `MAX_MELODY_LENGTH_CEILING` constant (1 MiB), threaded into
  `server::run`, and included in the startup `info!` log line.
- `src/server.rs` — added `max_melody_length: usize` to `AppState`,
  extended `run(...)` signature, passed value to both backends.
- `src/freebsd_speaker.rs` — `play_melody` now takes
  `max_melody_length: usize`; `validate_melody` uses it and produces
  the dynamic message `"Melody exceeds {N} bytes"`.
- `src/cpal_backend.rs` — same parameter and dynamic message.
- `src/speaker.rs` — **deleted** (dead code: not declared in
  `lib.rs`, no callers).
- `src/#cpal_backend.rs#` — **deleted** (stale Emacs autosave).
- `tests/integration_tests.rs` — updated all `server::run` callsites
  to pass the new arg (1000), updated `test_melody_validation`'s
  assertion to look for `"exceeds 1000 bytes"`.
- `API.md` — described the configurable limit, updated the example
  error string, added flag to the configuration section.

## Verification

- `cargo build` (default features) — clean.
- `cargo build --no-default-features` — clean.
- `cargo test` — 11 lib + 4 integration tests pass.
- CLI smoke test:
  - `--help` lists the new flag with default and range.
  - `--max-melody-length 0` exits 1 with a clear message.
  - `--max-melody-length 1048577` exits 1 with a clear message.

