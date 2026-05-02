# square waveform: kernel-faithful phase reset, no envelope

## Task Specification

User reports two perceptual mismatches between the FreeBSD-on-QEMU
output and our `--waveform=square` rendering of the same MML:

1. Notes feel slightly shorter on the emulation.
2. For dense fast melodies like `c32c32c32`, the notes are less
   clearly separated than under FreeBSD.

User's hypothesis (silence between notes in the FreeBSD spec) was
not the cause: re-verified `spkr.c::playtone` produces
`sound = 6, silence = 0` for `c32` at default tempo/fill, exactly
matching our Rust port. The total duration in seconds is identical.

## Root cause

FreeBSD's `tone()` calls `timer_spkr_setfreq()` at the start of
every note. That function writes the new divisor's high byte to
PIT port 0x42, which **resets the PIT's internal counter** —
producing a square-wave phase discontinuity at every note boundary
even when consecutive notes share a frequency. The brief gate
on/off cycle (one `outb` to port 0x61 each side) and the I/O
latency of the three PIT writes amount to ~5–10 µs round-trip,
which is sub-sample at typical audio rates and not audible as a
gap on its own — but the phase reset itself is what produces the
"plink" the ear locks onto as articulation.

Our `synth_generic` declared `phase` outside the per-event loop
and never reset it on `Tone`, only on `Rest`. So `c32c32c32`
rendered as one phase-continuous 180 ms square wave with no
boundary articulation. The 5 ms AR envelope's amplitude ramps
were too smooth and too low-frequency to be perceived as note
onsets — the ear heard gentle amplitude modulation on a single
sustained tone instead of three discrete notes. Without
articulation the brain's beat estimator can't lock on, which is
why the entire melody felt "faster" even though the absolute
duration was correct.

## Decisions

- **`Waveform::Square`** is now kernel-faithful: phase is reset to
  0 at every `Tone` event start (mirroring the PIT counter reset),
  and the AR envelope is disabled. This re-introduces the
  amplitude-step boundary clicks the user originally noticed —
  but those clicks are the *intended* sound character of the raw
  FreeBSD speaker driver. Users wanting click-suppressed playback
  have `square-bandlimited`, `sine`, `triangle`, `sawtooth`, and
  `pc-speaker`.
- **`Waveform::PcSpeaker`** also resets phase per `Tone`. The PIT
  reset is the authentic PC-speaker behaviour and the biquad
  filter chain will smooth the resulting transient into the
  mechanical-style click a real piezo would produce, instead of a
  sharp DAC click.
- **Other software waveforms** (`SquareBandlimited`, `Sine`,
  `Triangle`, `Sawtooth`) keep the AR envelope and phase
  continuity. They have no FreeBSD analog and benefit from
  click-free rendering.
- I/O latency / gate-off duration is **not** simulated as an
  explicit silent gap. It's microseconds — sub-sample at audio
  rates — and inserting any audible silence would deviate from the
  spec timing.

## Files Modified

- `src/cpal_backend.rs`:
  - `synth_generic`: per-waveform `reset_phase_per_tone` and
    `apply_envelope` flags. Square gets `reset=true,
    envelope=false`; others get `reset=false, envelope=true`.
  - `synth_pcspeaker`: phase reset on every `Tone` event.
- `README.md`: updated the paragraph on the waveform behaviour to
  reflect that `square` is now kernel-faithful (with clicks) and
  list which waveforms still have the envelope.

## Verification

- `cargo build` (default features) → clean.
- `cargo build --no-default-features` → clean.
- `cargo test` and `cargo test --no-default-features` → green.
- Manual A/B test on `c32c32c32` (`square` vs the previous build)
  is left to the user; expectation is three distinct articulated
  notes with FreeBSD-like character.

## Current Status

Done.
