---
title: CI build fails: missing ALSA dependency for CPAL
date: 2026-05-03
---

# CI build failure: missing ALSA dev headers

## Task Specification

User reports that the GitHub Actions CI build fails because the runner
doesn't have the ALSA development headers installed. The recent
addition of CPAL (default feature) requires `libasound2-dev` on Linux
to compile.

## Context

- CI workflow: `.github/workflows/rust.yml` runs on `ubuntu-latest`
- Cargo.toml: `cpal = { version = "0.16", optional = true }` is a default feature
- CPAL on Linux links against ALSA, which requires the `libasound2-dev`
  package at build time.
- The project's primary target is FreeBSD; Linux CI is just a sanity
  check, but we still want it to pass.

## Options Considered

1. Install `libasound2-dev` in the CI workflow before `cargo build`.
2. Disable the `cpal` default feature in CI via `--no-default-features`.
3. Both: install the dep AND add a `--no-default-features` job to keep
   that build path tested too.

## Decision

User selected option 3: install ALSA for the main job AND add a second
job that builds with `--no-default-features`.

Rationale: CPAL is the default feature for Linux developers, but the
production target is FreeBSD which uses the no-CPAL build path. Both
paths should be exercised on every PR. Keeping the existing job named
`build` preserves any branch-protection check that pins to the job
name; the new job is `build-no-default-features`.

## Files Modified

- `.github/workflows/rust.yml`: added `libasound2-dev` install step to
  the existing `build` job; added a new `build-no-default-features`
  job that runs `cargo build` and `cargo test` with
  `--no-default-features` (no ALSA needed).

## Current Status

Done. Awaiting user to commit and push to verify CI passes.

