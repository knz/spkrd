# Developing SPKRD

Notes for working on the SPKRD server itself. For installing it, see
[INSTALL.md](INSTALL.md); for running it, see [USAGE.md](USAGE.md); for
the HTTP API, see [API.md](API.md).

## Project structure

```
spkrd/
├── src/
│   ├── main.rs              # CLI entry point, flag parsing, daemonization
│   ├── lib.rs               # Library interface
│   ├── bind.rs              # --bind listen-address spec parsing
│   ├── server.rs            # HTTP server, routing, listener setup
│   ├── freebsd_speaker.rs   # /dev/speaker backend and retry logic
│   ├── cpal_backend.rs      # CPAL audio backend (feature `cpal`)
│   ├── mml.rs               # MML melody parser (port of FreeBSD spkr.c)
│   └── error.rs             # Error types
├── tests/
│   └── integration_tests.rs # Integration tests
├── examples/
│   ├── client.rs            # Rust client
│   ├── client.go            # Go client
│   ├── spkcmd               # Exit-status audio feedback wrapper
│   ├── spkcmd-bash.sh       # Bash shell integration
│   ├── spkcmd-zsh.sh        # Zsh shell integration
│   ├── tunes/               # Bundled .mml melodies
│   ├── Makefile             # Client build and install
│   └── Cargo.toml           # Client dependencies
├── rc.d/spkrd               # FreeBSD rc.d service script
├── systemd/spkrd.service    # Linux systemd user unit
├── changelog/               # Per-task design and decision notes
├── .github/workflows/       # CI
├── Makefile                 # Build and system installation
├── INSTALL.md               # Build and installation instructions
├── USAGE.md                 # Command line reference, logging, troubleshooting
├── DEVELOPMENT.md           # This file
├── API.md                   # Detailed API documentation
└── README.md                # Overview and entry point
```

Each source file carries a header comment summarising what it defines;
keep it current when changing the file.

## Building

```bash
# Debug build
cargo build

# Release build
cargo build --release
```

The `Makefile` wraps Cargo and additionally auto-detects optional audio
backends via `pkg-config` — see
[INSTALL.md](INSTALL.md#build-features) for the feature matrix and how
the detection works. For day-to-day development, calling `cargo`
directly is usually what you want.

### The two build configurations

The crate must build and pass tests both with and without the `cpal`
feature, and CI checks both:

```bash
# Default: includes the CPAL backend
cargo build

# FreeBSD kernel backend only; drops the cpal dependency entirely
cargo build --no-default-features
```

The `--no-default-features` path is easy to break, because
`cpal_backend`, several `main.rs` flags, and parts of `server.rs` are
behind `#[cfg(feature = "cpal")]`. Build it before sending a change.

## Running tests

```bash
# Full suite
cargo test

# With test output shown
cargo test -- --nocapture

# A single test by name
cargo test test_dual_stack_wildcard_bind

# The other build configuration
cargo test --no-default-features
```

The suite currently comprises:

| Location | Count (default features) | Covers |
|----------|--------------------------|--------|
| `src/bind.rs` | 13 | `--bind` spec parsing and its rejection cases |
| `src/mml.rs` | 10 | MML parsing |
| `src/cpal_backend.rs` | 2 | CPAL backend internals (compiled only with `cpal`) |
| `tests/integration_tests.rs` | 5 | End-to-end HTTP behaviour |

That is 30 tests with default features and 28 with
`--no-default-features` (the two `cpal_backend` tests are compiled out).

The integration tests use temporary files as mock speaker devices, so
they run on any platform and need neither a real `/dev/speaker` nor
audio hardware. `test_dual_stack_wildcard_bind` is the regression test
for the `IPV6_V6ONLY` behaviour described in `src/server.rs`'s header
comment — it binds the default `0.0.0.0,[::]` pair and would fail with
`EADDRINUSE` on Linux without the explicit socket option.

## Manual testing

### Against a mock device

Point `--device` at a regular file. Because the path exists,
`--output=auto` resolves to the `freebsd-speaker` backend, which writes
the melody to the file instead of producing sound:

```bash
touch /tmp/test-speaker
cargo run -- --device /tmp/test-speaker --port 18111 --bind 127.0.0.1 --debug
```

From another terminal:

```bash
curl -X PUT http://127.0.0.1:18111/play -d "cdefgab"
cat /tmp/test-speaker      # cdefgab
```

### Against real audio output

Point `--device` at a nonexistent path so `auto` falls back to CPAL, or
select the backend explicitly:

```bash
cargo run -- --output cpal --waveform square-bandlimited --debug
```

Use `--debug` throughout: it logs each request's client IP, melody, and
retry count.

Note that `[` and `]` are shell glob characters, so quote any `--bind`
argument containing an IPv6 entry: `--bind '127.0.0.1,[::1]'`.

## Continuous integration

`.github/workflows/rust.yml` runs on pushes and pull requests against
`master`, in two jobs:

- **build** — `cargo build` and `cargo test` with default features, on
  `ubuntu-latest`, after installing `libasound2-dev` and `libdbus-1-dev`
  (ALSA and D-Bus headers needed by cpal).
- **build-no-default-features** — `cargo build --no-default-features`
  and `cargo test --no-default-features`, which needs no system audio
  headers.

CI does not currently run `cargo fmt --check` or `cargo clippy`.

## Changelog notes

This repository keeps per-task notes under `changelog/`, named
`YYYYMMDD-topic.md`. They record the specification, the decisions taken
and why, obstacles hit, and what was verified — not code diffs. Add one
when starting a piece of work and update it as the work proceeds; commit
messages reference the relevant file. See `CLAUDE.md` for the full
convention.

## Contributing

1. Fork the repository
2. Create a feature branch
3. Add tests for new functionality
4. Ensure both build configurations pass: `cargo test` and
   `cargo test --no-default-features`
5. Update the affected documentation (`USAGE.md`, `INSTALL.md`,
   `API.md`, `rc.d/spkrd`'s comment header) in the same change — the
   flag reference lives in [USAGE.md](USAGE.md#command-line-options)
   and everything else links to it
6. Submit a pull request
