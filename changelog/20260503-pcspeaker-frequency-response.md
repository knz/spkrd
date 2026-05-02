# PC speaker frequency-response simulation

## Task Specification

The user is interested in simulating the PC speaker's frequency
response in software so the CPAL backend output more closely matches
what a real PC speaker would produce. Context: in the previous
discussion (changelog/20260502-cpal-backend-iteration.md and chat) we
established that the boundary clicks heard from the CPAL backend are
caused by step discontinuities at note boundaries that a real PC
speaker would smooth out via its mechanical bandwidth limitation. A
filter that mimics the PC speaker response would both kill the clicks
and produce a more authentic timbre.

Background reference: `src/qemu_pcspeaker.md` documents how QEMU
emulates the PC speaker — it generates a raw square wave at 32 kHz
with no filtering, so QEMU output also exhibits the same modern-DAC
clicks the CPAL backend has.

## Decisions

- Authentic-target simulation, tier 4 (HP + peak + LP biquad chain
  plus tanh saturation). Single-flag UX as a new `pc-speaker`
  waveform value, no extra CLI knobs.
- Modern piezoelectric disc preset. Constants chosen for the
  characteristic small-piezo response: HP @ 800 Hz / Q 0.707, peak
  @ 3 kHz / Q 3.0 / +9 dB, LP @ 6 kHz / Q 0.707, drive 2.0.
- 5 ms linear AR envelope applied to the other waveforms (Square /
  SquareBandlimited / Sine / Triangle / Sawtooth) to suppress the
  note-boundary clicks; capped at n/4 so very short staccato notes
  still get a proportional ramp without losing their body.
- PIT frequency quantisation (`PIT_FREQ / divisor`, integer
  divisor, with `divisor = round(PIT_FREQ / freq)`) is applied
  **only** on the pc-speaker path. Other waveforms keep the MML
  pitch table's A440 equal-tempered frequencies, since picking
  e.g. sine deliberately steps off the "match the kernel driver"
  use case.
- Biquad implementation uses RBJ-cookbook formulas with
  direct-form-2 transposed processing — five mul-adds per sample,
  no extra branching, coefficients pre-normalised by a0 at
  construction time.
- Filter state on the pc-speaker path persists across the entire
  event sequence including rests. Rests feed zeros into the
  filter chain so the stored energy decays naturally — this is
  what makes the speaker "ring out" on note-off rather than
  cutting silent, matching real-hardware behaviour.

## Files Modified

- `src/cpal_backend.rs`:
  - New `Waveform::PcSpeaker` variant + `FromStr` cases
    (`pc-speaker`, `pcspeaker`, `pc`).
  - New constants: `ENVELOPE_MS`, `PIT_FREQ`, the seven `PIEZO_*`
    preset values.
  - Refactored `synth()` into a dispatcher; new `synth_generic()`
    (existing oscillators + AR envelope) and `synth_pcspeaker()`
    (square at quantised frequency → HP → peak → LP → tanh →
    volume); shared `total_samples()` preallocation helper.
  - New `Biquad` struct with DF2T `process()` and three
    constructors (`lowpass`, `highpass`, `peak`) using RBJ-cookbook
    coefficients.
  - New `pit_quantize()` helper.
  - One new unit test for `pit_quantize()`.
- `src/main.rs`: `WaveformArg::PcSpeaker` variant and matching
  `From<WaveformArg>` arm.
- `README.md`: extended the waveform list and added a paragraph
  describing what `pc-speaker` does (frequency quantisation, biquad
  chain, soft-clip) plus a note about the AR envelope on the other
  waveforms.

Not modified:
- `src/mml.rs` — quantisation lives in the cpal backend, the MML
  interpreter keeps the kernel driver's pitch table.
- `src/freebsd_speaker.rs`, `src/server.rs`, `src/error.rs`,
  `src/lib.rs`, `Cargo.toml`, `tests/integration_tests.rs`.

## Verification

- `cargo build` (default features) → clean.
- `cargo build --no-default-features` → clean.
- `cargo test` (default features) → 11 unit (10 mml + 1 new
  pit_quantize) + 4 integration, all passing.
- `cargo test --no-default-features` → clean.
- `./spkrd --help` shows `pc-speaker` in the `--waveform` possible
  values list.

## Current Status

Done. Awaiting the user's call on whether to commit (this iteration
plus the previous CPAL-feature-flag iteration are both ready).
