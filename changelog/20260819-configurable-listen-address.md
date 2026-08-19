# Configurable listen address

## Task Specification

User request: add a flag to make the server's listen address configurable,
not just the port. Currently `src/server.rs` hardcodes the bind address to
`0.0.0.0` (all interfaces) and only `--port` (in `src/main.rs`) is
configurable.

## High-Level Decisions

- New `--bind <SPEC>` flag replaces the hardcoded `0.0.0.0` bind address.
  `SPEC` is a comma-separated list of `addr` or `addr:port` entries.
  - IPv4 entries are bare, unbracketed literals (e.g. `0.0.0.0`,
    `127.0.0.1:9000`). Brackets are not accepted for IPv4.
  - IPv6 entries must be bracketed (e.g. `[::]`, `[::1]:9000`) — required
    both to disambiguate from the bare-colon port syntax and, per user
    clarification, brackets are reserved exclusively for IPv6 (never valid
    around an IPv4 literal).
  - An entry without `:port` falls back to the value of `--port`, which is
    kept as a flag (dual role: legacy single-port default, and the
    default-port source for `--bind` entries).
  - Default: `--bind 0.0.0.0,[::]` (dual-stack, preserves prior behavior on
    IPv4 and adds IPv6-any).
- Server now binds and serves on N listeners concurrently via
  `tokio::task::JoinSet`, using the same `axum::serve` app for each. No new
  dependency needed (`JoinSet` is part of tokio's `full` feature, already
  enabled).
- Parsing lives in a new `src/bind.rs` module with its own unit tests, kept
  separate from `main.rs`'s CLI wiring and `server.rs`'s networking code.

## Requirements Changes

- Initial ask was just "configurable listen address" (one address). Refined
  via clarifying questions to: a `--bind` flag taking a *comma-separated
  list* of `addr[:port]` entries, defaulting to `0.0.0.0,[::]`, with
  brackets required for (and exclusive to) IPv6.

## Files Modified

- `src/bind.rs` (new): `parse_bind_spec` + unit tests.
- `src/lib.rs`: register `pub mod bind;`.
- `src/main.rs`: add `--bind` flag, parse it, exit(1) on parse error, pass
  `Vec<SocketAddr>` to `server::run`, update startup log line.
- `src/server.rs`: `run()` takes `Vec<SocketAddr>` instead of `port: u16`;
  binds one `TcpListener` per address; serves all concurrently via
  `JoinSet`.
- `tests/integration_tests.rs`: update the 4 `server::run(port, ...)` call
  sites to pass `vec![SocketAddr::from(([0, 0, 0, 0], port))]`.
- `API.md`: document `--bind`.
- `rc.d/spkrd`: add a commented `--bind` example.

## Rationales and Alternatives

- Considered a `--listen host:port` flag that would replace `--port`
  outright; rejected in favor of keeping `--port` as the default-port
  source for `--bind` entries, per user's explicit request.
- Considered allowing bracketed IPv4 (`[1.2.3.4]`) for syntactic
  uniformity; rejected per user clarification — brackets mean IPv6,
  unconditionally.

## Obstacles and Solutions

- `cargo build --all-features` fails on this machine because the `jack`
  feature needs system libjack via pkg-config, which isn't installed here.
  Unrelated to this change — verified with `cargo build` (default
  features) instead, which succeeds.

## Current Status

Implemented and verified:

- `cargo build` (default features) succeeds; `cargo clippy --all-targets`
  shows no new warnings (pre-existing warnings are in `examples/client.rs`,
  unrelated to this change).
- `cargo test`: all 25 unit tests (13 new, in `src/bind.rs`) and all 4
  integration tests pass.
- Manual smoke test: `--bind 127.0.0.1:PORT` and `--bind "addr1,[::1]:addr2"`
  both bind and serve concurrently; a request to each listener reaches the
  shared backend. `--bind "[127.0.0.1]"` (bracketed IPv4) is correctly
  rejected at startup with exit code 1.

Done — no further steps planned.
