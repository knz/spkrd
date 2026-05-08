// MML (Music Macro Language) interpreter — Rust port of FreeBSD's
// sys/dev/speaker/spkr.c (playstring/playtone/playinit). The original C
// driver is v1.4 by Eric S. Raymond <esr@snark.thyrsus.com> (Aug 1993),
// modified for FreeBSD by Andrew A. Chernov <ache@astral.msk.su>.
//
// License: the original spkr.c is part of the FreeBSD kernel and is
// distributed under the standard 2-clause BSD license used by FreeBSD,
// which is compatible with this repository's BSD-2-Clause license.
//
// This port preserves the kernel driver's semantics: same A440
// equal-tempered pitch table, same integer-arithmetic fill/silence
// split, same defaults (octave 4, tempo 120, value 4, fill NORMAL),
// same handling of accidentals, dotted notes, slur `_`, octave tracking
// (OL/ON/O<n>/>/</), numeric notes (N<n>), rests (P/~), tempo (T),
// length (L), and articulation (M[NLS]). Output is a sequence of
// Tone/Rest events with frequencies in Hz and durations in centiseconds.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Event {
    Tone { freq_hz: u32, centisecs: u32 },
    Rest { centisecs: u32 },
}

// FreeBSD spkr.c constants
const SECS_PER_MIN: i32 = 60;
const WHOLE_NOTE: i32 = 4;
const MIN_VALUE: i32 = 64;
const DFLT_VALUE: i32 = 4;
const FILLTIME: i32 = 8;
const STACCATO: i32 = 6;
const NORMAL: i32 = 7;
const LEGATO: i32 = 8;
const DFLT_OCTAVE: i32 = 4;
const MIN_TEMPO: i32 = 32;
const DFLT_TEMPO: i32 = 120;
const MAX_TEMPO: i32 = 255;
const NUM_MULT: i32 = 3;
const DENOM_MULT: i32 = 2;
const OCTAVE_NOTES: i32 = 12;

// Letter to half-tone offset:  A   B  C  D  E  F  G
const NOTETAB: [i32; 7] = [9, 11, 0, 2, 4, 5, 7];

// A440 equal-tempered, rounded to nearest integer; spkr.c's pitchtab.
// Octave 0 here is standard octave 2.
const PITCHTAB: [u32; 84] = [
    65, 69, 73, 78, 82, 87, 93, 98, 103, 110, 117, 123,
    131, 139, 147, 156, 165, 175, 185, 196, 208, 220, 233, 247,
    262, 277, 294, 311, 330, 349, 370, 392, 415, 440, 466, 494,
    523, 554, 587, 622, 659, 698, 740, 784, 831, 880, 932, 988,
    1047, 1109, 1175, 1245, 1319, 1397, 1480, 1568, 1661, 1760, 1865, 1975,
    2093, 2217, 2349, 2489, 2637, 2794, 2960, 3136, 3322, 3520, 3729, 3951,
    4186, 4435, 4698, 4978, 5274, 5588, 5920, 6272, 6644, 7040, 7459, 7902,
];
const PITCHTAB_OCTAVES: i32 = (PITCHTAB.len() as i32) / OCTAVE_NOTES; // 7

struct State {
    octave: i32,
    whole: i32,
    value: i32,
    fill: i32,
    octtrack: bool,
    octprefix: bool,
    events: Vec<Event>,
}

impl State {
    fn new() -> Self {
        Self {
            octave: DFLT_OCTAVE,
            whole: (100 * SECS_PER_MIN * WHOLE_NOTE) / DFLT_TEMPO,
            value: DFLT_VALUE,
            fill: NORMAL,
            octtrack: false,
            octprefix: true,
            events: Vec::new(),
        }
    }

    // Mirrors playtone() in spkr.c.
    fn playtone(&mut self, pitch: i32, value: i32, sustain: i32) {
        let mut snum: i32 = 1;
        let mut sdenom: i32 = 1;
        for _ in 0..sustain {
            snum *= NUM_MULT;
            sdenom *= DENOM_MULT;
        }

        if value == 0 || sdenom == 0 {
            return;
        }

        let whole = self.whole;
        let fill = self.fill;

        if pitch == -1 {
            let cs = whole * snum / (value * sdenom);
            if cs > 0 {
                self.events.push(Event::Rest { centisecs: cs as u32 });
            }
        } else {
            let sound = (whole * snum) / (value * sdenom)
                - (whole * (FILLTIME - fill)) / (value * FILLTIME);
            let silence =
                whole * (FILLTIME - fill) * snum / (FILLTIME * value * sdenom);
            let freq = PITCHTAB[pitch as usize];
            if sound > 0 {
                self.events.push(Event::Tone {
                    freq_hz: freq,
                    centisecs: sound as u32,
                });
            }
            if fill != LEGATO && silence > 0 {
                self.events.push(Event::Rest {
                    centisecs: silence as u32,
                });
            }
        }
    }
}

// Render an MML melody to events. Mirrors playstring() in spkr.c.
pub fn render(melody: &str) -> Vec<Event> {
    let bytes = melody.as_bytes();
    let mut st = State::new();
    let mut i: usize = 0;
    let mut lastpitch: i32 = OCTAVE_NOTES * DFLT_OCTAVE;

    // Helper: ascii-uppercase a single byte (matches toupper on ascii).
    fn up(b: u8) -> u8 {
        b.to_ascii_uppercase()
    }
    // GETNUM: while next byte is ascii digit, consume it into v.
    fn getnum(bytes: &[u8], i: &mut usize) -> i32 {
        let mut v: i32 = 0;
        while *i + 1 < bytes.len() && (bytes[*i + 1] as char).is_ascii_digit() {
            *i += 1;
            v = v.saturating_mul(10).saturating_add((bytes[*i] - b'0') as i32);
        }
        v
    }

    while i < bytes.len() {
        let c = up(bytes[i]);
        match c {
            b'A'..=b'G' => {
                let mut pitch =
                    NOTETAB[(c - b'A') as usize] + st.octave * OCTAVE_NOTES;

                if i + 1 < bytes.len() {
                    let n = bytes[i + 1];
                    if n == b'#' || n == b'+' {
                        pitch += 1;
                        i += 1;
                    } else if n == b'-' {
                        pitch -= 1;
                        i += 1;
                    }
                }

                if st.octtrack && !st.octprefix {
                    if (pitch - lastpitch).abs()
                        > (pitch + OCTAVE_NOTES - lastpitch).abs()
                    {
                        st.octave += 1;
                        pitch += OCTAVE_NOTES;
                    }
                    if (pitch - lastpitch).abs()
                        > ((pitch - OCTAVE_NOTES) - lastpitch).abs()
                    {
                        st.octave -= 1;
                        pitch -= OCTAVE_NOTES;
                    }
                }
                st.octprefix = false;
                lastpitch = pitch;

                let mut timeval = getnum(bytes, &mut i);
                if timeval <= 0 || timeval > MIN_VALUE {
                    timeval = st.value;
                }

                let mut sustain = 0;
                while i + 1 < bytes.len() && bytes[i + 1] == b'.' {
                    i += 1;
                    sustain += 1;
                }

                let oldfill = st.fill;
                if i + 1 < bytes.len() && bytes[i + 1] == b'_' {
                    st.fill = LEGATO;
                    i += 1;
                }

                // Bounds-check pitch against pitchtab length, matching the
                // implicit array access in the C code (which would index out
                // of bounds for very high notes); we clamp to avoid panics.
                if (0..PITCHTAB.len() as i32).contains(&pitch) {
                    st.playtone(pitch, timeval, sustain);
                }

                st.fill = oldfill;
            }
            b'O' => {
                if i + 1 < bytes.len() {
                    let n = up(bytes[i + 1]);
                    if n == b'N' {
                        st.octprefix = false;
                        st.octtrack = false;
                        i += 1;
                    } else if n == b'L' {
                        st.octtrack = true;
                        i += 1;
                    } else {
                        let v = getnum(bytes, &mut i);
                        st.octave = if v >= PITCHTAB_OCTAVES {
                            DFLT_OCTAVE
                        } else {
                            v
                        };
                        st.octprefix = true;
                    }
                } else {
                    let v = getnum(bytes, &mut i);
                    st.octave = if v >= PITCHTAB_OCTAVES {
                        DFLT_OCTAVE
                    } else {
                        v
                    };
                    st.octprefix = true;
                }
            }
            b'>' => {
                if st.octave < PITCHTAB_OCTAVES - 1 {
                    st.octave += 1;
                }
                st.octprefix = true;
            }
            b'<' => {
                if st.octave > 0 {
                    st.octave -= 1;
                }
                st.octprefix = true;
            }
            b'N' => {
                let pitch = getnum(bytes, &mut i);
                let mut sustain = 0;
                while i + 1 < bytes.len() && bytes[i + 1] == b'.' {
                    i += 1;
                    sustain += 1;
                }
                let oldfill = st.fill;
                if i + 1 < bytes.len() && bytes[i + 1] == b'_' {
                    st.fill = LEGATO;
                    i += 1;
                }
                let p = pitch - 1;
                if p == -1 || (0..PITCHTAB.len() as i32).contains(&p) {
                    st.playtone(p, st.value, sustain);
                }
                st.fill = oldfill;
            }
            b'L' => {
                let v = getnum(bytes, &mut i);
                st.value = if v <= 0 || v > MIN_VALUE { DFLT_VALUE } else { v };
            }
            b'P' | b'~' => {
                let mut timeval = getnum(bytes, &mut i);
                if timeval <= 0 || timeval > MIN_VALUE {
                    timeval = st.value;
                }
                let mut sustain = 0;
                while i + 1 < bytes.len() && bytes[i + 1] == b'.' {
                    i += 1;
                    sustain += 1;
                }
                st.playtone(-1, timeval, sustain);
            }
            b'T' => {
                let v = getnum(bytes, &mut i);
                let tempo = if !(MIN_TEMPO..=MAX_TEMPO).contains(&v) {
                    DFLT_TEMPO
                } else {
                    v
                };
                st.whole = (100 * SECS_PER_MIN * WHOLE_NOTE) / tempo;
            }
            b'M' if i + 1 < bytes.len() => {
                let n = up(bytes[i + 1]);
                if n == b'N' {
                    st.fill = NORMAL;
                    i += 1;
                } else if n == b'L' {
                    st.fill = LEGATO;
                    i += 1;
                } else if n == b'S' {
                    st.fill = STACCATO;
                    i += 1;
                }
            }
            _ => {}
        }
        i += 1;
    }

    st.events
}

#[cfg(test)]
mod tests {
    use super::*;

    // Defaults: tempo=120 → whole = 100*60*4/120 = 200 cs; value=4 (quarter).
    // Fill=NORMAL=7: sound = 200/4 - 200*(8-7)/(4*8) = 50 - 6 = 44; silence
    // = 200*(8-7)/(8*4) = 6.

    #[test]
    fn empty() {
        assert!(render("").is_empty());
    }

    #[test]
    fn single_c4_default() {
        let ev = render("c");
        // C in octave 4: pitch index = 0 + 4*12 = 48 → 1047 Hz
        assert_eq!(
            ev,
            vec![
                Event::Tone { freq_hz: 1047, centisecs: 44 },
                Event::Rest { centisecs: 6 },
            ]
        );
    }

    #[test]
    fn legato_no_silence() {
        // ML sets fill=LEGATO; silence is omitted regardless of value.
        let ev = render("MLc");
        assert_eq!(ev.len(), 1);
        match ev[0] {
            Event::Tone { freq_hz: 1047, centisecs } => {
                // sound = 200/4 - 200*(8-8)/(4*8) = 50
                assert_eq!(centisecs, 50);
            }
            _ => panic!("expected tone"),
        }
    }

    #[test]
    fn staccato_more_silence() {
        // MS: fill=STACCATO=6; sound = 50 - 200*2/32 = 50-12=38; silence
        // = 200*2/(8*4)=12.
        let ev = render("MSc");
        assert_eq!(
            ev,
            vec![
                Event::Tone { freq_hz: 1047, centisecs: 38 },
                Event::Rest { centisecs: 12 },
            ]
        );
    }

    #[test]
    fn rest_default() {
        // P uses value=4 → cs = 200/4 = 50.
        let ev = render("p");
        assert_eq!(ev, vec![Event::Rest { centisecs: 50 }]);
    }

    #[test]
    fn dotted_quarter() {
        // c. → sustain=1, snum=3, sdenom=2; sound=200*3/(4*2) - 200*1/(4*8)
        // = 75 - 6 = 69; silence=200*1*3/(8*4*2)=75/8=9 (int div).
        let ev = render("c.");
        assert_eq!(
            ev,
            vec![
                Event::Tone { freq_hz: 1047, centisecs: 69 },
                Event::Rest { centisecs: 9 },
            ]
        );
    }

    #[test]
    fn accidental_and_octave_change() {
        let ev = render("o5c#");
        // O5C#: octave 5, C# = pitch 60+1 = 61 → pitchtab[61]=2217
        match ev[0] {
            Event::Tone { freq_hz: 2217, .. } => {}
            _ => panic!("expected 2217 Hz tone, got {:?}", ev[0]),
        }
    }

    #[test]
    fn tempo_change() {
        // T240: whole = 100*60*4/240 = 100; sound=100/4-100/32=25-3=22
        // (int div), silence=100/(8*4)=3.
        let ev = render("T240c");
        assert_eq!(
            ev,
            vec![
                Event::Tone { freq_hz: 1047, centisecs: 22 },
                Event::Rest { centisecs: 3 },
            ]
        );
    }

    #[test]
    fn numeric_note() {
        // N49 → pitch=48 → 1047 Hz (same as c default octave).
        let ev = render("N49");
        assert_eq!(
            ev,
            vec![
                Event::Tone { freq_hz: 1047, centisecs: 44 },
                Event::Rest { centisecs: 6 },
            ]
        );
    }

    #[test]
    fn slur_only_affects_one_note() {
        // c_d: c is legato (no silence after), d is normal again.
        let ev = render("c_d");
        // c with LEGATO fill: sound=50, no silence
        // d with NORMAL fill: sound=44, silence=6
        assert_eq!(ev.len(), 3);
        assert_eq!(ev[0], Event::Tone { freq_hz: 1047, centisecs: 50 });
        // d4 → pitch 50 → pitchtab[50]=1175
        assert_eq!(ev[1], Event::Tone { freq_hz: 1175, centisecs: 44 });
        assert_eq!(ev[2], Event::Rest { centisecs: 6 });
    }
}
