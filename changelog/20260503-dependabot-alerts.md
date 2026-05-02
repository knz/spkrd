# Dependabot Alerts Remediation

## Task Specification

Address open Dependabot security alerts for the `knz/spkrd` repository
(https://github.com/knz/spkrd/security/dependabot).

## Open Alerts (snapshot)

13 open alerts across two lockfiles (`Cargo.lock`, `examples/Cargo.lock`),
all transitively pulled in. None of these crates are direct dependencies
of `spkrd` itself; they come in via dev-deps (notably `reqwest`) and
runtime deps (`tokio`, `chrono`, `syslog`, etc.).

| # | Package  | Current | Patched | Severity | Manifest             |
|---|----------|---------|---------|----------|----------------------|
| 13| openssl  | 0.10.73 | 0.10.78 | high     | examples/Cargo.lock  |
| 12| openssl  | 0.10.73 | 0.10.78 | high     | Cargo.lock           |
| 11| openssl  | 0.10.73 | 0.10.78 | low      | examples/Cargo.lock  |
| 10| openssl  | 0.10.73 | 0.10.78 | low      | Cargo.lock           |
|  9| openssl  | 0.10.73 | 0.10.78 | high     | examples/Cargo.lock  |
|  8| openssl  | 0.10.73 | 0.10.78 | high     | Cargo.lock           |
|  7| openssl  | 0.10.73 | 0.10.78 | high     | examples/Cargo.lock  |
|  6| openssl  | 0.10.73 | 0.10.78 | high     | Cargo.lock           |
|  5| openssl  | 0.10.73 | 0.10.78 | high     | examples/Cargo.lock  |
|  4| openssl  | 0.10.73 | 0.10.78 | high     | Cargo.lock           |
|  3| time     | 0.3.41  | 0.3.47  | medium   | Cargo.lock           |
|  2| bytes    | 1.10.1  | 1.11.1  | medium   | examples/Cargo.lock  |
|  1| bytes    | 1.10.1  | 1.11.1  | medium   | Cargo.lock           |

All advisories are within the existing semver-compatible range, so a
plain `cargo update -p <crate>` (or `--precise <version>`) should be
sufficient — no `Cargo.toml` changes anticipated.

## High-Level Decisions

(pending user clarification / approval)

## Files Modified

(pending)

## Current Status

- Inventoried open Dependabot alerts and current lockfile versions.
- Awaiting user confirmation on scope and approval of remediation plan.
