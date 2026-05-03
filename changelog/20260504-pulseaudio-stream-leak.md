# PulseAudio playback streams accumulating per melody

## Task Specification

User report: with the running `spkrd` service connected to PulseAudio,
every melody played causes a new PulseAudio playback source to appear
on the server, and the previous ones are not removed. Resource leak.

A previous agent attempted a fix by editing `~/src/cpal/`
(`src/host/pulseaudio/stream.rs`). The user observed no change in
behaviour and asked for a fresh independent analysis.

## Investigation

### Where the leak originates (cpal pulseaudio backend)

`cpal::host::pulseaudio::stream::Stream::new_playback` (in cpal's
master at commit 078787e — what spkrd actually uses) creates a
`pulseaudio::PlaybackStream` and spawns two threads:

1. **play_all thread**: `block_on(stream_clone.play_all())` —
   `play_all` is `source_eof().await?; drain().await?;`. `source_eof`
   only resolves when the cpal user-callback wrapper returns 0 from
   `poll_read`. The `CallbackWrapper` in `pulseaudio` crate (0.3.1)
   *always* returns `buf.len()` from the user callback, never 0. So
   `source_eof()` blocks forever; the play_all thread never exits.

2. **Latency monitor thread**: polls `timing_info()` periodically;
   exits when `LatencyHandle::cancel` is set.

Both threads hold a `pulseaudio::PlaybackStream` clone, which is
`Arc<InnerPlaybackStream>`. The PA server-side stream is only released
when `InnerPlaybackStream::drop` runs, which sends
`DeletePlaybackStream` via `now_or_never()`. That Drop only fires when
the Arc count reaches 0.

`cpal::Stream::drop` (unpatched) just calls `handle.cancel()`. That
unblocks the latency thread but leaves the play_all thread stuck on
`source_eof`. Result: every stream creation leaks one PA stream plus
one stuck OS thread.

### Why the previous agent's patch is not effecting the running binary

The patch to `~/src/cpal/src/host/pulseaudio/stream.rs` adds
`block_on(stream.clone().delete())` after `handle.cancel()`. That
sends `DeletePlaybackStream` and waits for the ack; when the ack
arrives, the reactor removes the stream state, dropping `eof_tx`,
which wakes the play_all thread's `source_eof` with cancellation, so
play_all returns `Err` and the thread exits.

That logic is sound. The patch isn't taking effect because spkrd's
`Cargo.toml` has:

    [patch.crates-io]
    cpal = { git = "https://github.com/RustAudio/cpal" }

This patches the dep against a **git URL**, not a local path. Cargo
ignores `~/src/cpal/` entirely; it fetches and uses
`~/.cargo/git/checkouts/cpal-476cd1dd23dbc279/078787e/`, which still
contains the unpatched `Stream::drop`. Confirmed by reading the actual
checkout file — it still has the original 4-line Drop impl.

So: the diagnosis "Stream::drop leaks the PA stream because
play_all's source_eof never resolves" is correct, the textual patch
that addresses it is correct, but the patch is not in any code path
that gets compiled.

## Decisions

- Fix lives in cpal, not in spkrd: the play_all driver thread is
  internal to cpal's PulseAudio backend; no spkrd-side workaround
  can reach it through the public cpal API.
- Use `now_or_never` rather than `block_on` in `Stream::drop` to
  match the pattern already used in `pulseaudio::PlaybackStream`'s
  own `Drop` and avoid blocking arbitrary user code in destructors.
  Cleanup still completes asynchronously: the queued
  `DeletePlaybackStream` causes the reactor to drop `eof_tx`, which
  wakes `play_all` with cancellation and lets the driver thread
  exit and release its `Arc` clone.
- Only the `Playback` arm needs the fix. The `Record` path has no
  `play_all`-equivalent thread, so its existing
  `InnerRecordStream::drop` (already `now_or_never`) is sufficient.
- Patch is carried on the user's fork (`knz/cpal`, branch
  `fix/pulseaudio-stream-leak-1188`) and consumed via
  `[patch.crates-io]` in `Cargo.toml` until the upstream PR for
  RustAudio/cpal#1188 merges.

## Files Modified

- `~/src/cpal/src/host/pulseaudio/stream.rs`: in `Stream::drop`,
  split the `Playback`/`Record` arms; for the playback arm queue a
  `delete()` via `now_or_never` after `handle.cancel()`. Add the
  `futures::FutureExt` import.
- `~/src/cpal/CHANGELOG.md`: new "Fixed" entry under the Unreleased
  PulseAudio bullets, referencing #1188.
- `Cargo.toml` (this repo): repoint `[patch.crates-io] cpal` from
  the upstream RustAudio/cpal git URL to the
  `knz/cpal#fix/pulseaudio-stream-leak-1188` branch, with a comment
  explaining why and when to revert.
- `changelog/20260504-pulseaudio-stream-leak.md` (this file).

Pushed to `git@github.com:knz/cpal.git` as branch
`fix/pulseaudio-stream-leak-1188`, commit `3b2475d`. PR for
RustAudio/cpal#1188 to be opened by the user.

## Verification

- `cargo check --features pulseaudio` in `~/src/cpal/`: clean (one
  pre-existing dead_code warning unrelated to this change).
- `cargo build --release --features pulseaudio` in spkrd: clean.
- `Cargo.lock` confirms cpal source resolves to
  `git+https://github.com/knz/cpal?branch=fix%2Fpulseaudio-stream-leak-1188#3b2475d9...`,
  i.e. the actually-compiled checkout carries the patched
  `Stream::drop`.

Manual runtime verification (running spkrd, sending several melodies
via HTTP, and watching `pactl list short sink-inputs` shrink back
between requests instead of accumulating) is left to the user.

## Current Status

Patch shipped on the fork; spkrd repointed; build green. Awaiting
manual confirmation that the leak is gone, then upstream PR.
