# 20260503 - Linux systemd support in Makefile

## Task Specification

Add Linux/systemd support to the Makefile's `install` target, alongside the
existing FreeBSD rc.d service installation.

## Current State

- `Makefile` installs binary + FreeBSD `rc.d/spkrd` script under `$(DSTDIR)`.
- No Linux-specific installation path exists.
- The binary supports `--daemon` / `--pidfile` flags for daemonization.

## High-Level Decisions

- **OS detection**: shell `uname -s` conditional inside the recipe, avoiding make-level conditionals (`ifeq`/`.if`) that differ between GNU make and BSD make.
- **Systemd unit type**: `Type=simple` — binary runs in foreground without `--daemon`; systemd owns the lifecycle and journald captures stderr logs.
- **Output backend**: `--output=auto` (default) — probes `/dev/speaker` and falls back to CPAL on Linux.
- **Unit file location**: `$(DSTDIR)/lib/systemd/system/` (i.e. `/usr/local/lib/systemd/system/` by default) — customary for source-built packages; supported since systemd v239.
- **Post-install**: print instructions to the user; do not run `systemctl daemon-reload` automatically.

## Files Modified

- `systemd/spkrd.service` — new systemd unit file (Type=simple, port 1111, output=auto)
- `Makefile` — added OS-detection shell branch in `install` recipe; Linux installs unit file and prints post-install instructions; BSD path unchanged

## Current Status

Complete.
