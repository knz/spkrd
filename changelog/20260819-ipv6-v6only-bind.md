# Default --bind spec fails to start on Linux (EADDRINUSE on [::])

## Task Specification

User report: after `make install`, `systemctl --user start spkrd` fails.
Initially suspected to be fallout from the cpal fork removal earlier in
the same session (see `20260819-cpal-upstream-update.md`).

## Diagnosis

Not related to the cpal change. The installed unit runs

    /usr/local/bin/spkrd --port 1111 --cpal-host=PulseAudio

with no `--bind`, so the default spec `0.0.0.0,[::]` from commit
`6efe1ff` applies. `server::run` binds one listener per entry, in order:

    Server listening on 0.0.0.0:1111
    Server error: failed to bind [::]:1111: Address already in use (98)

Root cause: `tokio::net::TcpListener::bind` does not set
`IPV6_V6ONLY`, so the socket inherits the host default. On Linux
`net.ipv6.bindv6only` defaults to 0, which makes a `[::]` socket
dual-stack — it accepts IPv4 as well. It therefore overlaps the
already-bound `0.0.0.0` socket on the same port and the second bind
fails.

Reproduced deterministically on a scratch port with a locally built
binary; `cat /proc/sys/net/ipv6/bindv6only` on the reporting machine
confirms 0.

Why it was not caught when `--bind` landed: on FreeBSD, the project's
primary target, `net.inet6.ip6.v6only` defaults to **1**, so the two
wildcard sockets are genuinely independent there and the default spec
works. The breakage is Linux-specific. The `--bind` commit also added
no test that binds more than one address — all four existing
`server::run` call sites in the integration tests pass a single IPv4
address.

Interim workaround given to the user: a `systemctl --user edit spkrd`
drop-in using `--bind '[::]:1111'`, relying on dual-stack to cover both
families. Verified serving IPv4 and IPv6 clients. **This must be
reverted now that the real fix has landed** — with `IPV6_V6ONLY` set,
`[::]` alone no longer accepts IPv4.

## Decisions

Three options were put to the user:

1. Set `IPV6_V6ONLY` on IPv6 listeners so the two wildcard entries are
   independent everywhere.
2. Change the default spec to `[::]` alone, relying on dual-stack.
3. Skip a bind that collides with an already-bound address.

**User chose 1.** Rationale: it makes the documented default mean the
same thing on every host — two listeners, one per family — rather than
depending on a sysctl, and it makes Linux behave the way FreeBSD
already does. Option 2 would silently lose IPv4 where
`bindv6only=1` and fail outright where IPv6 is disabled; option 3
would mask genuine port conflicts.

Trade-off accepted: `--bind '[::]'` on its own is now genuinely
IPv6-only. Documented in `API.md` and `rc.d/spkrd`.

Implementation notes:

- `socket2` is used because the option must be set between `socket()`
  and `bind()`, and tokio's `TcpListener::bind` exposes no hook there.
  It was already in the lockfile as a tokio transitive dependency, so
  this adds no new code to the tree.
- `SO_REUSEADDR` is set explicitly on Unix. `std::net::TcpListener::bind`
  sets it and `socket2::Socket::new` does not; omitting it would have
  regressed restart-while-TIME_WAIT.
- Backlog 128, matching what `std` uses.

## Files Modified

- `Cargo.toml`: add `socket2 = { version = "0.6", features = ["all"] }`.
- `src/server.rs`: new `bind_listener` replaces the direct
  `TcpListener::bind` call; sets `IPV6_V6ONLY` for IPv6 addresses and
  `SO_REUSEADDR` on Unix. Module comment explains the FreeBSD/Linux
  sysctl difference and the v6-only consequence.
- `tests/integration_tests.rs`: new `test_dual_stack_wildcard_bind`.
- `API.md`, `rc.d/spkrd`: document that IPv6 entries are v6-only.
- `changelog/20260819-ipv6-v6only-bind.md` (this file).

`src/bind.rs` is unchanged — parsing was never the problem.

## Verification

- `cargo build`, `cargo build --release --features pulseaudio`,
  `cargo build --no-default-features`: clean.
- `cargo test`: 25 unit + 5 integration tests pass.
- `cargo clippy --all-targets`: no warnings in `server.rs` or the tests.
- **Negative control**: flipping `set_only_v6(true)` to `false` makes
  `test_dual_stack_wildcard_bind` fail with exactly the reported error
  (`failed to bind [::]:36113: Address already in use (os error 98)`),
  confirming the test covers the actual regression rather than passing
  vacuously.
- End-to-end with the real default spec: both listener lines appear at
  startup with no error, and an IPv4 client (`127.0.0.1`) and an IPv6
  client (`[::1]`) each get HTTP 200.

Not verified: behaviour on FreeBSD, and on a host with IPv6 disabled
entirely (where the default spec's `[::]` entry would fail to bind for
an unrelated reason — pre-existing, not introduced here).

## Current Status

Fix complete and verified on Linux. Outstanding for the user:

1. Remove the `--bind '[::]:1111'` drop-in workaround, since `[::]`
   alone is now IPv6-only.
2. Re-run `make install` to pick up the fixed binary.
