# CPAL backend cancellation bug — concurrent melodies overlapping

## Task Specification

The user reported that the CPAL backend does not enforce mutual
exclusion between melodies in the way the FreeBSD `/dev/speaker`
driver does: when the first HTTP client is interrupted (Ctrl-C
before its melody finishes), a second client's melody plays in
parallel with the first instead of waiting.

## Root cause

In the original `play_melody`, the lock was a
`tokio::sync::Mutex<()>` and the `MutexGuard` was held by the async
parent future across the `spawn_blocking(...).await`. When the
client disconnects, axum drops the parent future, which drops the
guard and releases the lock. But `spawn_blocking` cannot be aborted
by tokio — the blocking task continues running, owns its own
`Arc<CpalBackend>` clone, and keeps the live `cpal::Stream` playing
in CPAL's audio thread. The next request sees the lock free,
acquires it, builds a *second* `cpal::Stream`, and PulseAudio /
PipeWire mixes both streams — audible overlap.

Verified via per-event eprintln instrumentation (timestamps showed
the lock-released event firing before the audio actually stopped
when the parent was cancelled).

## Decisions

- Fix matches FreeBSD's `spkr.c` behaviour. There the per-tone
  `tsleep` is invoked with `PCATCH`, so a signal interrupts the
  in-progress melody and releases the `sx_xlock` mid-string. We
  reproduce this with an abort flag observed by the audio callback.
- Lock-follows-the-work: the `play_lock` is now acquired *inside*
  the `spawn_blocking` task. The lock's lifetime is tied to the
  blocking task's lifetime, not to the parent future's. If the
  parent future is dropped, the blocking task continues holding the
  lock until it finishes naturally (or is cut short by the abort
  flag), at which point the lock is released and the next request
  can proceed. This eliminates any window where the parent has
  released the lock but the audio thread is still playing.
- Cancellation is propagated via `Arc<AtomicBool>`: an
  `AbortOnDrop` guard in the async parent sets the flag in its
  `Drop` impl. The cpal callback observes the flag once per
  invocation; when set it writes zeros to the output and signals
  `done`, ending the wait in `run_stream`.
- `play_lock` switches from `tokio::sync::Mutex<()>` to
  `std::sync::Mutex<()>` because the lock is now held entirely
  inside synchronous (blocking) code. Holding a tokio mutex across
  blocking work is the wrong tool for the job.
- Cancellation does not surface as an HTTP error: by definition the
  client has already disconnected, so the response body is
  irrelevant. The blocking task may complete with `Ok(retries)` or
  with `SpeakerError::Timeout` if the lock was never acquired
  within retry_timeout.

## Files Modified

- `src/cpal_backend.rs`:
  - `play_lock` type: `AsyncMutex<()>` → `std::sync::Mutex<()>`.
  - `play_melody`: render the buffer in the async parent
    (synthesis is CPU-only, no need to spawn_blocking it); move
    the retry-poll lock acquisition into the blocking task; install
    an `AbortOnDrop` guard in the async parent.
  - New `AbortOnDrop` struct.
  - `run_stream` (and `play_buffer`): now take an
    `Arc<AtomicBool>` abort flag, wire it into the callback. The
    callback writes silence and signals `done` when the flag is
    set. The 50 ms tail sleep is skipped on abort to keep
    cancellation snappy.

## Verification

- `cargo build` (default features) → clean.
- `cargo build --no-default-features` → clean.
- `cargo test` and `cargo test --no-default-features` → green.
- Manual reproduction of the user's scenario: long melody on
  client 1, Ctrl-C client 1, immediate request from client 2 →
  should hear client 2 alone (with at most a small audio-system
  tail bleed, separate issue).

## Current Status

Done. Manual reproduction of the original scenario (long melody on
client 1, Ctrl-C, immediate request from client 2) is left to the
user since this requires real audio output that the development
environment can't validate without disturbing other audio.
