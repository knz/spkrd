# MML Tunes Organization

## Task Specification

User has placed MML (Music Macro Language) tune files as `.txt` files at the
repository root and wants them:

1. Moved into a sub-directory.
2. Renamed with more descriptive file names and a `.mml` extension.
3. Documented for use with the bundled `spkrc` client.

## Files Identified

The following `.txt` files are at the repo root:

- `dc.txt` — short F-major melody, `t104 l4 ...` (identity unclear, possibly
  shares melody with `m1.txt`)
- `elise.txt` — Beethoven's "Für Elise" (opening figure `E D# E D# E ...`)
- `hb.txt` — "Happy Birthday to You"
- `jb.txt` — "Jingle Bells"
- `m1.txt` — same melody as `dc.txt`, formatted one phrase per line
- `mario.txt` — Super Mario Bros. theme
- `mary.txt` — "Mary Had a Little Lamb"
- `oj.txt` — Beethoven's "Ode to Joy"
- `twinkle.txt` — "Twinkle, Twinkle, Little Star"

## High-Level Decisions (from user clarification)

- `dc.txt` is "Dance of the Cuckoos"; `m1.txt` is a duplicate of the same
  melody and will be discarded.
- Destination directory: `examples/tunes/`.
- Naming convention: kebab-case full titles, `.mml` extension.
- Move (not copy) — originals at the repo root are removed.
- Documentation lives inside the existing `examples/README.md`.

## Planned Renames

| From          | To                                              |
|---------------|-------------------------------------------------|
| `dc.txt`      | `examples/tunes/dance-of-the-cuckoos.mml`       |
| `elise.txt`   | `examples/tunes/fur-elise.mml`                  |
| `hb.txt`      | `examples/tunes/happy-birthday.mml`             |
| `jb.txt`      | `examples/tunes/jingle-bells.mml`               |
| `mario.txt`   | `examples/tunes/super-mario-bros.mml`           |
| `mary.txt`    | `examples/tunes/mary-had-a-little-lamb.mml`     |
| `oj.txt`      | `examples/tunes/ode-to-joy.mml`                 |
| `twinkle.txt` | `examples/tunes/twinkle-twinkle-little-star.mml`|
| `m1.txt`      | (deleted — duplicate of dc.txt)                 |

## Files Modified

- Created `examples/tunes/` directory.
- Moved/renamed eight `.txt` melody files from the repo root into
  `examples/tunes/` with kebab-case `.mml` names (see "Planned Renames"
  table above).
- Deleted `m1.txt` (duplicate of `dc.txt`).
- Updated `examples/README.md` with a new "Bundled Tunes" section that
  lists the included tunes, shows how to play them with `spkrc` via
  `"$(cat ...)"` command substitution, gives a loop one-liner to audition
  them all, and notes the 1000-char melody limit plus the `~` caveat for
  `super-mario-bros.mml`.
- Created/updated `changelog/20260503-mml-tunes-organization.md` (this file).

## Obstacles and Solutions

- `mario.txt` uses `~` (tie/sustain) which is not part of the FreeBSD
  `speaker(4)` MML grammar — flagged in the README rather than rewriting
  the file.

## Current Status

Complete. No source code changes; build/test untouched.
