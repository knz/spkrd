# README documentation drift after `--bind`; split of INSTALL.md

## Task Specification

Original request: does `README.md` need updating following the recent
addition of the `--bind` flag (commits `6efe1ff` "Add --bind flag for
configurable server listen addresses" and `a7c6579` "Bind IPv6 listeners
v6-only so the default --bind spec works on Linux")?

Scope was widened twice by the user during the conversation:

1. First: "let's update README holistically" — fix all drift found in
   `README.md`, not just the `--bind` gaps.
2. Then: "let's split the installation instructions to a separate
   INSTALL.md and link README to it. also fix examples/README.md."
3. Mid-task: "please also update rc.d/spkrd after you're done."
4. Then: "also move the development doc to a separate file."
5. Finally: "let's have 'usage', 'logging' and 'troubleshooting' moved to
   a new USAGE.md as well."

Final scope: documentation only. Six files: `README.md` (reduced to an
entry point), `INSTALL.md` (new), `USAGE.md` (new), `DEVELOPMENT.md`
(new), `examples/README.md` (corrected), `rc.d/spkrd` (comment header
only).

What began as "does README need a `--bind` mention?" ended as a
restructuring of the documentation set. The incremental splits were each
requested separately rather than planned up front.

## Audit findings (before any edit)

`API.md` was updated by both `--bind` commits and documents the flag
correctly, including v6-only behaviour. `rc.d/spkrd` was also updated.
`README.md` and `examples/README.md` were touched by neither.

### README.md — `--bind` drift

- `--bind` absent from both flag lists: the rc.conf "Available
  configuration flags" (~line 148) and "Command Line Options" (~line 295).
- `--port` described as "Server port"; since `6efe1ff` it is the *default*
  port for `--bind` entries that omit one.
- IPv6 v6-only behaviour undocumented. `[::]` serves IPv6 only; `0.0.0.0`
  must be listed alongside to serve both. Non-obvious and the likeliest
  source of user confusion.
- Startup-log sample (~line 264) shows the old `port=1111` format. Actual
  format is now `Starting spkrd: bind=[...], retry_timeout=...,
  max_melody_length=..., output=... (resolved=...), device=...,
  daemon=..., pidfile=..., debug=...`.

### README.md — other flag drift (pre-existing, not from the --bind work)

- `--max-melody-length` (default 1000, range `1..=1048576`) documented in
  API.md but absent from both README flag lists. README states the
  1000-char limit as fixed at lines 33 and 391.
- Debug short flag documented as `-d`; the code uses `-D` (`-d` is bound
  to `--device`). Same error present in `rc.d/spkrd:20`.

### README.md — structural drift

- "Project Structure" tree lists a `speaker.rs` that no longer exists and
  omits `bind.rs`, `mml.rs`, `freebsd_speaker.rs`, `cpal_backend.rs`. The
  `examples/` subtree is stale, and `rc.d/`, `systemd/`, `Makefile` are
  absent entirely.
- No Linux/systemd installation documented at all, although `make install`
  auto-detects the OS via `uname` and on Linux installs a systemd **user**
  unit (chosen so the service can reach the per-user PulseAudio/PipeWire
  socket; needs `loginctl enable-linger` to start without login). README
  presents installation as FreeBSD-only.
- Makefile `FEATURES` auto-detection undocumented: when `FEATURES` is
  empty (the default) the recipe pkg-config-probes for `jack`,
  `libpulse`, and `libpipewire-0.3` and enables each feature found.
- The flag reference is duplicated in two places, which is what let the
  two lists drift apart in the first place.

### examples/README.md drift

- "Files" list omits `spkcmd`, `spkcmd-bash.sh`, `spkcmd-zsh.sh`,
  `tunes/`, `Cargo.toml`.
- **Documents a `BUILD` variable that does not exist.** The Makefile's
  variable is `PROFILE`. Every `make BUILD=debug` example in the file is
  wrong and fails to do what it claims.
- Shell-integration section tells users to source
  `/usr/local/share/spkrd/examples/spkcmd-{bash,zsh}.sh`, but the
  `install` target installs only `client`→`spkrc` and `spkcmd`. That path
  is never created.
- States a fixed 1000-character melody limit; now configurable via
  `--max-melody-length`. The `bach.mml` note mentions needing a longer
  limit without naming the flag.
- Typo: `bach.ml` should be `bach.mml`.
- Go client build/install is not covered by the Makefile; not stated.

## High-Level Decisions

**Split installation out of README.** README had grown to ~456 lines with
build, feature-selection, service-installation, and service-management
prose crowding out the usage and API material. Installation content moves
to `INSTALL.md`; README links to it.

**Single canonical flag reference.** The duplicated flag lists are the
mechanism by which the `--bind` drift happened — one list was updated,
the other forgotten. README keeps one "Command Line Options" section;
INSTALL.md's service-configuration sections carry service-specific
examples only and link back rather than restating flags.

**Waveform prose stays in README.** It describes runtime behaviour, not
installation; it currently sits under Service Configuration only by
accident of where the flags were listed.

## Files Modified

- `changelog/20260819-readme-bind-doc-drift.md` (this file, created)
- `INSTALL.md` (created) — prerequisites, building, build features and
  the Makefile's pkg-config auto-detection, system-wide installation
  split by OS (FreeBSD rc.d / Linux systemd user unit), service
  configuration and management, client-install pointer, verification
  steps, uninstall steps.
- `DEVELOPMENT.md` (created) — project structure tree, building, the two
  build configurations (`default` vs `--no-default-features`) and why the
  latter is easy to break, running tests with a per-location breakdown,
  manual testing against a mock device and against real audio, CI
  description, the `changelog/` convention, contribution workflow.
- `USAGE.md` (created) — starting the server, the canonical command line
  reference (with `--bind` and the corrected `--port`,
  `--max-melody-length`, and `-D` entries), "Listen addresses" covering
  the v6-only semantics and the shell-quoting gotcha, "Waveforms", the
  HTTP API summary and Python example, the full Logging section with the
  corrected startup-log sample, and Troubleshooting including new
  `EADDRINUSE` and over-length-melody entries. Section heading levels
  were promoted one level during the move so the file reads as a
  document rather than a transplanted subsection.
- `README.md` (reduced to an entry point, 456 → 116 lines) — retains
  overview, features, a short install snippet, the melody syntax quick
  reference, a two-command usage snippet, "How It Works", and links out
  to the four companion documents.
- `examples/README.md` (corrected) — `BUILD` → `PROFILE` throughout,
  completed file list, corrected shell-integration instructions, melody
  limit described as configurable, `bach.ml` typo, Go client build note,
  See Also section.
- `rc.d/spkrd` (comment header only) — `-d` → `-D` for `--debug`, added
  `--output` and `--max-melody-length`, added a pointer to USAGE.md for
  the full option list and an extra example.

## Rationales and Alternatives

Considered leaving installation in README and only fixing the drift.
Rejected at user request; the split also reduces the surface where the
same fact is stated twice.

The duplicated flag lists were the mechanism by which the `--bind` drift
happened — one list was updated by `6efe1ff`, the other forgotten.
Collapsing to a single list in README, with INSTALL.md and `rc.d/spkrd`
linking to it rather than restating flags, removes that failure mode.

`rc.d/spkrd` was initially held back as out of documentation scope, then
pulled in when the user asked for it mid-task. Only its comment header
was touched; the executable portion is unchanged.

## Obstacles and Solutions

- The `[` and `]` in IPv6 `--bind` entries are shell glob characters and
  zsh rejects them unquoted ("no matches found"). Hit while verifying the
  documented commands; added a quoting note to README's "Listen
  addresses" section and quoted the affected examples.
- An initial draft of INSTALL.md's verification section implied a file
  used as `--device` could produce audio. It cannot: an existing path
  makes `--output=auto` resolve to `freebsd-speaker`, which only writes
  the melody to the file. Rewritten as two separate cases (no sound
  hardware / with audio output).

## Verification

Documentation-only change, but every documented claim was checked
against the built binary rather than against the source alone:

- `cargo build --release` then `spkrd --help` — confirmed every
  documented flag, default, and short option, including `-D` for
  `--debug` and `-d` for `--device`.
- Ran the server with `--bind '127.0.0.1,[::1]'` — confirmed the startup
  log line and the per-listener "Server listening on ..." lines match the
  sample now in README verbatim.
- Ran the file-as-mock-device workflow from INSTALL.md end to end: PUT
  returned 200 and the melody appeared in the file.
- `cargo test` — 30 tests pass (25 lib + 5 integration).
  `cargo test --no-default-features` — 28 pass (the two `cpal_backend`
  tests are compiled out). Both counts are stated in DEVELOPMENT.md and
  were taken from these runs, not estimated.
- `sh -n rc.d/spkrd` — syntax intact after the comment edit.
- Checked all relative markdown links across the five documents resolve,
  and that every `#anchor` used has a matching heading. After the USAGE.md
  split, re-checked that no `README.md#...` reference survives anywhere
  (the flag-reference anchor moved to `USAGE.md#command-line-options`,
  and INSTALL.md, DEVELOPMENT.md, and `rc.d/spkrd` were updated).
- Content-preservation check across the split: grepped for distinctive
  strings from every moved section (`IPV6_V6ONLY`, `timer_spkr_setfreq`,
  `pw groupmod`, `journalctl --user`, `cpal-device`, "Testing Without
  Hardware", the Python example, …) and confirmed each still appears in
  the document set.

## Current Status

Complete. All six files written and verified. Not staged or committed —
per repository rules, staging is left to the user.

Resulting document set (README was 456 lines before this work, and was
the only user-facing document besides API.md):

| File | Lines | Covers |
|------|-------|--------|
| `README.md` | 116 | Overview, features, melody syntax, how it works, links |
| `USAGE.md` | 299 | Command line reference, listen addresses, waveforms, API, logging, troubleshooting |
| `INSTALL.md` | 299 | Prerequisites, building, features, service setup, verification, uninstall |
| `examples/README.md` | 302 | Clients, bundled tunes, spkcmd, shell integration |
| `DEVELOPMENT.md` | 183 | Layout, build configs, tests, manual testing, CI, contributing |

The canonical flag reference now lives in exactly one place,
`USAGE.md#command-line-options`; INSTALL.md, DEVELOPMENT.md, and
`rc.d/spkrd` link to it rather than restating flags. Keeping that
single-source property is what prevents a repeat of the drift this task
started from.

Known remaining item, not addressed: `examples/README.md` documents
sourcing the `spkcmd-{bash,zsh}.sh` shell integrations, but the
`examples/Makefile` `install` target does not install them. This was
resolved on the documentation side (users are told to source from the
checkout). Installing them to a share directory instead would be a
Makefile change and was left out of this documentation-only pass.
