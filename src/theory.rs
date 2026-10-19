//! Music Theory, Microtonal Tuning Systems, Parsimonious Voice-Leading,
//! and Cthulhu/Scaler-Style Multiline Chord Progression Sequencer.

use crate::math_engine::PHI;
use crate::scala::ScalaScale;

#[derive(Debug, Clone)]
pub enum TuningSystem {
    Equal12 { a4_hz: f64 },
    JustIntonation { root_pc: u8, a4_hz: f64 },
    Pythagorean { root_pc: u8, a4_hz: f64 },
    HarmonicSeries { base_hz: f64 },
    GoldenRatio { a4_hz: f64 },
    Scala { scale: ScalaScale, tonic_midi: f64, a4_hz: f64 },
}

impl Default for TuningSystem {
    fn default() -> Self {
        TuningSystem::Equal12 { a4_hz: 440.0 }
    }
}

impl TuningSystem {
    pub fn from_str(s: &str, root_pc: u8) -> Self {
        match s.trim().to_ascii_lowercase().as_str() {
            "just" | "ji" | "just_intonation" | "pure" => TuningSystem::JustIntonation {
                root_pc,
                a4_hz: 440.0,
            },
            "432" | "equal432" => TuningSystem::Equal12 { a4_hz: 432.0 },
            "pyth" | "pythagorean" => TuningSystem::Pythagorean {
                root_pc,
                a4_hz: 440.0,
            },
            "harm" | "harmonic" | "overtone" => TuningSystem::HarmonicSeries { base_hz: 55.0 },
            "phi" | "golden" | "golden_ratio" => TuningSystem::GoldenRatio { a4_hz: 440.0 },
            other => {
                if let Some(scale) = crate::scala::find_builtin_scala(other) {
                    TuningSystem::Scala {
                        scale,
                        tonic_midi: 69.0,
                        a4_hz: 440.0,
                    }
                } else if other.starts_with("edo") {
                    if let Ok(n) = other.trim_start_matches("edo").parse::<usize>() {
                        TuningSystem::Scala {
                            scale: crate::scala::equal_temperament(n, 2.0),
                            tonic_midi: 69.0,
                            a4_hz: 440.0,
                        }
                    } else {
                        TuningSystem::Equal12 { a4_hz: 440.0 }
                    }
                } else {
                    TuningSystem::Equal12 { a4_hz: 440.0 }
                }
            }
        }
    }

    pub fn label(&self) -> String {
        match self {
            TuningSystem::Equal12 { a4_hz } if (*a4_hz - 432.0).abs() < 1.0 => {
                "12-TET (A4 = 432 Hz)".into()
            }
            TuningSystem::Equal12 { .. } => "12-Tone Equal Temperament (A4 = 440 Hz)".into(),
            TuningSystem::JustIntonation { .. } => "Pure 5/7-Limit Just Intonation".into(),
            TuningSystem::Pythagorean { .. } => "Pythagorean Pure Fifths (3:2)".into(),
            TuningSystem::HarmonicSeries { .. } => "Pure Spectral Overtone Series".into(),
            TuningSystem::GoldenRatio { .. } => "Golden Ratio (Phi = 1.618) Microtonal".into(),
            TuningSystem::Scala { scale, .. } => format!("Scala: {}", scale.description),
        }
    }

    /// Convert a MIDI note number (with optional chord root pitch class for pure adaptive Just Intonation) to Hz.
    pub fn midi_to_hz(&self, midi_note: f64, chord_root_pc: Option<u8>) -> f64 {
        match self {
            TuningSystem::Equal12 { a4_hz } => *a4_hz * (2.0_f64).powf((midi_note - 69.0) / 12.0),
            TuningSystem::JustIntonation { root_pc, a4_hz } => {
                let ref_pc = chord_root_pc.unwrap_or(*root_pc) % 12;
                let rounded = midi_note.round() as i32;
                let pc = rounded.rem_euclid(12) as u8;
                let interval = (pc + 12 - ref_pc) % 12;
                // Pure 5-limit/7-limit harmonic ratios
                let ratios: [f64; 12] = [
                    1.0,          // Unison 1:1
                    16.0 / 15.0,  // Minor 2nd 16:15
                    9.0 / 8.0,    // Major 2nd 9:8
                    6.0 / 5.0,    // Minor 3rd 6:5
                    5.0 / 4.0,    // Major 3rd 5:4
                    4.0 / 3.0,    // Perfect 4th 4:3
                    7.0 / 5.0,    // Septimal Tritone 7:5
                    3.0 / 2.0,    // Perfect 5th 3:2
                    8.0 / 5.0,    // Minor 6th 8:5
                    5.0 / 3.0,    // Major 6th 5:3
                    7.0 / 4.0,    // Harmonic 7th 7:4 (or 9:5)
                    15.0 / 8.0,   // Major 7th 15:8
                ];
                let root_midi_same_oct = rounded - (interval as i32);
                let root_hz = *a4_hz * (2.0_f64).powf((root_midi_same_oct as f64 - 69.0) / 12.0);
                root_hz * ratios[interval as usize]
            }
            TuningSystem::Pythagorean { root_pc, a4_hz } => {
                let rounded = midi_note.round() as i32;
                let pc = rounded.rem_euclid(12) as u8;
                let interval = (pc + 12 - (*root_pc % 12)) % 12;
                let pyth_ratios: [f64; 12] = [
                    1.0,
                    256.0 / 243.0,
                    9.0 / 8.0,
                    32.0 / 27.0,
                    81.0 / 64.0,
                    4.0 / 3.0,
                    729.0 / 512.0,
                    3.0 / 2.0,
                    128.0 / 81.0,
                    27.0 / 16.0,
                    16.0 / 9.0,
                    243.0 / 128.0,
                ];
                let root_midi_same_oct = rounded - (interval as i32);
                let root_hz = *a4_hz * (2.0_f64).powf((root_midi_same_oct as f64 - 69.0) / 12.0);
                root_hz * pyth_ratios[interval as usize]
            }
            TuningSystem::HarmonicSeries { base_hz } => {
                // Map MIDI notes around C2 (36) to harmonics 1..32
                let step = (midi_note - 36.0).max(0.0);
                let octave = (step / 12.0).floor();
                let degree = (step % 12.0).round() as usize;
                let partial_map: [f64; 12] =
                    [1.0, 17.0 / 16.0, 9.0 / 8.0, 19.0 / 16.0, 5.0 / 4.0, 21.0 / 16.0, 11.0 / 8.0, 3.0 / 2.0, 13.0 / 8.0, 27.0 / 16.0, 7.0 / 4.0, 15.0 / 8.0];
                *base_hz * (2.0_f64).powf(octave) * partial_map[degree.min(11)]
            }
            TuningSystem::GoldenRatio { a4_hz } => {
                let oct = (midi_note - 69.0) / 12.0;
                let semi = (midi_note - 69.0).rem_euclid(12.0);
                let phi_blend = 0.85 * (2.0_f64).powf(oct)
                    + 0.15 * PHI.powf(oct * 1.4404) * (1.0 + 0.008 * semi);
                *a4_hz * phi_blend
            }
            TuningSystem::Scala {
                scale,
                tonic_midi,
                a4_hz,
            } => {
                let tonic_hz = *a4_hz * (2.0_f64).powf((*tonic_midi - 69.0) / 12.0);
                let degree = (midi_note - tonic_midi).round() as i32;
                tonic_hz * scale.degree_ratio(degree)
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MultilinePattern {
    /// Lush sustained voice-led chords + deep pedal bass + slow counter-melody
    SustainedCathedral,
    /// J.S. Bach WTC Prelude in C style interlocking 16th-note multiline figure + sustained bass/tenor
    BachPrelude,
    /// Xfer Cthulhu style multi-octave arpeggio + counter-voices
    CthulhuCascade,
    /// 1980s Synth-Classical driving 16th pulse + soaring brass-string chords
    StrangerPulse,
    /// Philip Glass / Modern Minimalist 3-against-4 polyrhythmic voice weave
    Polyrhythm3v4,
    /// Four independent contrapuntal voices with smooth portamento pitch-glide
    CounterpointFourVoice,
}

impl MultilinePattern {
    pub fn from_str(s: &str) -> Self {
        match s.trim().to_ascii_lowercase().as_str() {
            "sustained" | "cathedral" | "pad" | "sustained_cathedral" => {
                MultilinePattern::SustainedCathedral
            }
            "bach" | "prelude" | "bach_prelude" | "baroque" => MultilinePattern::BachPrelude,
            "cthulhu" | "cascade" | "cthulhu_cascade" | "arp" => MultilinePattern::CthulhuCascade,
            "stranger" | "pulse" | "stranger_pulse" | "synthwave" => MultilinePattern::StrangerPulse,
            "poly" | "polyrhythm" | "3v4" | "glass" | "polyrhythm_3v4" => {
                MultilinePattern::Polyrhythm3v4
            }
            "counterpoint" | "four_voice" | "glide" | "counterpoint_four_voice" => {
                MultilinePattern::CounterpointFourVoice
            }
            _ => MultilinePattern::CthulhuCascade,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            MultilinePattern::SustainedCathedral => "sustained_cathedral",
            MultilinePattern::BachPrelude => "bach_prelude",
            MultilinePattern::CthulhuCascade => "cthulhu_cascade",
            MultilinePattern::StrangerPulse => "stranger_pulse",
            MultilinePattern::Polyrhythm3v4 => "polyrhythm_3v4",
            MultilinePattern::CounterpointFourVoice => "counterpoint_four_voice",
        }
    }
}

#[derive(Debug, Clone)]
pub struct ParsedChord {
    pub symbol: String,
    pub root_pc: u8,
    pub bass_pc: u8,
    /// Relative semitone intervals above root (e.g. `[0, 3, 7, 10, 14]`)
    pub intervals: Vec<i32>,
    /// Optional explicit frequencies in Hz (when user writes `[220, 275, 330, 412.5]`)
    pub explicit_hz: Option<Vec<f64>>,
    /// Duration in beats (default 4.0 = 1 bar in 4/4)
    pub beats: f64,
}

#[derive(Debug, Clone)]
pub struct NoteEvent {
    pub start_sec: f64,
    pub duration_sec: f64,
    pub start_freq_hz: f64,
    pub target_freq_hz: f64,
    pub velocity: f64,
    pub voice_line: usize,
}

/// Parse a progression string like `"Cm9 -> Abmaj7#11 -> Fm11 -> G7b9"` or `"Am9 | Fmaj9 | Cmaj9 | E7alt"`.
pub fn parse_progression(prog_str: &str, default_beats_per_chord: f64) -> Vec<ParsedChord> {
    let normalized = prog_str
        .replace("->", "|")
        .replace('→', "|")
        .replace(';', "|");

    let parts: Vec<&str> = if normalized.contains('|') {
        normalized.split('|').collect()
    } else if normalized.contains(',') && !normalized.contains('[') {
        normalized.split(',').collect()
    } else {
        normalized.split_whitespace().collect()
    };

    let mut chords = Vec::new();
    for raw in parts {
        let trimmed = raw.trim();
        if trimmed.is_empty() {
            continue;
        }
        if let Some(chord) = parse_single_chord(trimmed, default_beats_per_chord) {
            chords.push(chord);
        }
    }

    if chords.is_empty() {
        // Fallback lush Cm9 -> Abmaj9 -> Fm11 -> G7b9
        return parse_progression("Cm9 -> Abmaj9#11 -> Fm11 -> G7b9", default_beats_per_chord);
    }

    chords
}

pub fn parse_single_chord(token: &str, default_beats: f64) -> Option<ParsedChord> {
    // Support optional `*2` or `:2` beat length suffix (e.g. `Cm9*4`)
    let (chord_part, beats) = if let Some((c, b)) = token.rsplit_once('*') {
        (c.trim(), b.trim().parse::<f64>().unwrap_or(default_beats))
    } else {
        (token.trim(), default_beats)
    };

    // Check if explicit frequency list `[220, 275, 330, 412.5]`
    if chord_part.starts_with('[') && chord_part.ends_with(']') {
        let inner = &chord_part[1..chord_part.len() - 1];
        let mut hz_list = Vec::new();
        for item in inner.split(|c: char| c == ',' || c.is_whitespace()) {
            let s = item.trim();
            if !s.is_empty() {
                if let Ok(hz) = s.parse::<f64>() {
                    if hz > 10.0 && hz < 24000.0 {
                        hz_list.push(hz);
                    }
                }
            }
        }
        if !hz_list.is_empty() {
            return Some(ParsedChord {
                symbol: chord_part.to_string(),
                root_pc: 0,
                bass_pc: 0,
                intervals: vec![0, 4, 7, 11],
                explicit_hz: Some(hz_list),
                beats,
            });
        }
    }

    // Handle slash bass note e.g. `Cmaj9/E`
    let (main_sym, slash_bass) = if let Some((m, b)) = chord_part.split_once('/') {
        (m.trim(), Some(b.trim()))
    } else {
        (chord_part, None)
    };

    let (root_pc, rest) = parse_note_pitch_class(main_sym)?;
    let bass_pc = slash_bass
        .and_then(|b| parse_note_pitch_class(b).map(|(pc, _)| pc))
        .unwrap_or(root_pc);

    let q = rest.trim();
    let ql = q.to_ascii_lowercase();

    let is_minor = (ql.starts_with('m') && !ql.starts_with("maj")) || ql.starts_with("min") || ql.starts_with('-');
    let is_dim = ql.contains("dim") || ql.contains("m7b5");
    let is_aug = ql.contains("aug") || ql.contains('+');
    let is_sus2 = ql.contains("sus2");
    let is_sus4 = ql.contains("sus4") || (ql.contains("sus") && !is_sus2);
    let is_maj7 = ql.contains("maj") || q.contains('M');

    let third = if is_sus2 {
        2
    } else if is_sus4 {
        5
    } else if is_minor || is_dim {
        3
    } else {
        4
    };

    let fifth = if is_dim || ql.contains("b5") {
        6
    } else if is_aug || ql.contains("#5") {
        8
    } else {
        7
    };

    let mut intervals = vec![0, third, fifth];

    // 6th / 7th extensions
    if ql.contains("dim7") {
        intervals.push(9);
    } else if is_maj7 || ql.contains("maj9") || ql.contains("maj11") || ql.contains("maj13") {
        intervals.push(11);
    } else if ql.contains('7') || ql.contains('9') || ql.contains("11") || ql.contains("13") || ql.contains("alt") {
        if !ql.contains("add9") && !ql.contains("add11") {
            intervals.push(10);
        }
    } else if ql.contains('6') {
        intervals.push(9);
    }

    // 9th extensions
    if ql.contains("b9") || ql.contains("alt") {
        intervals.push(13);
    } else if ql.contains("#9") {
        intervals.push(15);
    } else if ql.contains('9') || ql.contains("11") || ql.contains("13") {
        intervals.push(14);
    }

    // 11th extensions
    if ql.contains("#11") {
        intervals.push(18);
    } else if ql.contains("11") {
        intervals.push(17);
    }

    // 13th extensions
    if ql.contains("b13") || ql.contains("alt") {
        intervals.push(20);
    } else if ql.contains("13") {
        intervals.push(21);
    }

    // Ensure at least 4 rich voices by adding octave/9th color if needed
    if intervals.len() < 4 {
        intervals.push(12);
    }

    Some(ParsedChord {
        symbol: token.to_string(),
        root_pc,
        bass_pc,
        intervals,
        explicit_hz: None,
        beats,
    })
}

fn parse_note_pitch_class(s: &str) -> Option<(u8, &str)> {
    let mut chars = s.chars();
    let first = chars.next()?.to_ascii_uppercase();
    let base_pc: i32 = match first {
        'C' => 0,
        'D' => 2,
        'E' => 4,
        'F' => 5,
        'G' => 7,
        'A' => 9,
        'B' => 11,
        _ => return None,
    };

    let rest = &s[first.len_utf8()..];
    if let Some(second) = rest.chars().next() {
        match second {
            '#' | '♯' => {
                let pc = (base_pc + 1).rem_euclid(12) as u8;
                return Some((pc, &rest[second.len_utf8()..]));
            }
            'b' | '♭' => {
                let pc = (base_pc - 1).rem_euclid(12) as u8;
                return Some((pc, &rest[second.len_utf8()..]));
            }
            _ => {}
        }
    }

    Some((base_pc as u8, rest))
}

/// Build a voice-led multiline sequence of `NoteEvent`s across the chord progression!
/// Decomposes each chord into:
/// - Line 0: Deep Pedal / Sub Bass (octave 2)
/// - Lines 1..=4: Parsimoniously voice-led inner SATB harmonic voices (octaves 3..4)
/// - Line 5..=6: Multiline Arpeggiated / Contrapuntal Upper Weave (octaves 4..5)
pub fn generate_multiline_events(
    chords: &[ParsedChord],
    bpm: f64,
    pattern: MultilinePattern,
    tuning: TuningSystem,
) -> (Vec<NoteEvent>, f64) {
    let sec_per_beat = 60.0 / bpm.clamp(30.0, 260.0);
    let mut events = Vec::new();
    let mut cursor_sec = 0.0_f64;

    // Track previous chord's inner voice MIDI notes for parsimonious SATB voice-leading & glide!
    let mut prev_voicing_midi: Option<Vec<f64>> = None;
    let mut prev_bass_hz: Option<f64> = None;

    for (chord_idx, chord) in chords.iter().enumerate() {
        let chord_dur_sec = chord.beats * sec_per_beat;

        // Resolve voice frequencies for this chord
        let (bass_hz, fifth_hz, inner_hz, arp_hz_pool) =
            if let Some(ref explicit) = chord.explicit_hz {
                let b = explicit[0];
                let f = explicit.get(1).copied().unwrap_or(b * 1.5);
                let inner: Vec<f64> = explicit.clone();
                let mut pool = explicit.clone();
                for &hz in explicit {
                    pool.push(hz * 2.0);
                }
                (b, f, inner, pool)
            } else {
                // Bass in octave 2 (MIDI 36..47)
                let mut bass_midi = 36.0 + chord.bass_pc as f64;
                if bass_midi > 45.0 {
                    bass_midi -= 12.0;
                }
                let bass_hz = tuning.midi_to_hz(bass_midi, Some(chord.root_pc));
                let fifth_hz = tuning.midi_to_hz(bass_midi + 7.0, Some(chord.root_pc));

                // Target 4 inner voices around MIDI 55..74 (G3..D5)
                let mut raw_pitches: Vec<f64> = chord
                    .intervals
                    .iter()
                    .take(5)
                    .map(|&iv| 48.0 + chord.root_pc as f64 + iv as f64)
                    .collect();

                // Voice-leading optimizer: shift individual voices by +/- 12 semitones to stay close to previous chord's voicing
                if let Some(ref prev) = prev_voicing_midi {
                    for (vi, pitch) in raw_pitches.iter_mut().enumerate() {
                        let target_ref = prev.get(vi).copied().unwrap_or(60.0);
                        while *pitch - target_ref > 6.5 && *pitch >= 58.0 {
                            *pitch -= 12.0;
                        }
                        while target_ref - *pitch > 6.5 && *pitch <= 68.0 {
                            *pitch += 12.0;
                        }
                    }
                    raw_pitches.sort_by(|a, b| a.partial_cmp(b).unwrap());
                } else {
                    for pitch in &mut raw_pitches {
                        while *pitch < 55.0 {
                            *pitch += 12.0;
                        }
                        while *pitch > 76.0 {
                            *pitch -= 12.0;
                        }
                    }
                    raw_pitches.sort_by(|a, b| a.partial_cmp(b).unwrap());
                }

                let inner_hz: Vec<f64> = raw_pitches
                    .iter()
                    .map(|&m| tuning.midi_to_hz(m, Some(chord.root_pc)))
                    .collect();

                // Extended multi-octave pool for Cthulhu/Bach arpeggio weavers
                let mut arp_pool = Vec::new();
                for &m in &raw_pitches {
                    arp_pool.push(tuning.midi_to_hz(m, Some(chord.root_pc)));
                }
                for &m in &raw_pitches {
                    arp_pool.push(tuning.midi_to_hz(m + 12.0, Some(chord.root_pc)));
                }
                prev_voicing_midi = Some(raw_pitches);
                (bass_hz, fifth_hz, inner_hz, arp_pool)
            };

        let start_bass_hz = prev_bass_hz.unwrap_or(bass_hz);
        prev_bass_hz = Some(bass_hz);

        match pattern {
            MultilinePattern::SustainedCathedral => {
                // Line 0: Deep sustained bass pedal
                events.push(NoteEvent {
                    start_sec: cursor_sec,
                    duration_sec: chord_dur_sec * 0.96,
                    start_freq_hz: start_bass_hz,
                    target_freq_hz: bass_hz,
                    velocity: 0.88,
                    voice_line: 0,
                });
                // Line 1: Fifth anchor entering slightly after root
                events.push(NoteEvent {
                    start_sec: cursor_sec + sec_per_beat * 0.25,
                    duration_sec: chord_dur_sec * 0.88,
                    start_freq_hz: fifth_hz,
                    target_freq_hz: fifth_hz,
                    velocity: 0.65,
                    voice_line: 1,
                });
                // Lines 2..: Voice-led inner pad voices with gentle staggered entrances
                for (vi, &hz) in inner_hz.iter().enumerate() {
                    let stagger = (vi as f64) * 0.04;
                    events.push(NoteEvent {
                        start_sec: cursor_sec + stagger,
                        duration_sec: (chord_dur_sec - stagger) * 0.95,
                        start_freq_hz: hz,
                        target_freq_hz: hz,
                        velocity: 0.72 - (vi as f64 * 0.04),
                        voice_line: vi + 2,
                    });
                }
                // Slow lyrical upper counter-melody (half-note motion across the top of the chord)
                let half_dur = chord_dur_sec * 0.5;
                let top_a = arp_hz_pool[arp_hz_pool.len().saturating_sub(2)];
                let top_b = arp_hz_pool[arp_hz_pool.len().saturating_sub(1)];
                events.push(NoteEvent {
                    start_sec: cursor_sec,
                    duration_sec: half_dur * 0.92,
                    start_freq_hz: top_a,
                    target_freq_hz: top_a,
                    velocity: 0.68,
                    voice_line: 6,
                });
                events.push(NoteEvent {
                    start_sec: cursor_sec + half_dur,
                    duration_sec: half_dur * 0.92,
                    start_freq_hz: top_a,
                    target_freq_hz: top_b,
                    velocity: 0.74,
                    voice_line: 6,
                });
            }

            MultilinePattern::BachPrelude => {
                // Sustained bass root + tenor inner voice (baroque pedal)
                events.push(NoteEvent {
                    start_sec: cursor_sec,
                    duration_sec: chord_dur_sec * 0.96,
                    start_freq_hz: start_bass_hz,
                    target_freq_hz: bass_hz,
                    velocity: 0.86,
                    voice_line: 0,
                });
                if let Some(&tenor_hz) = inner_hz.first() {
                    events.push(NoteEvent {
                        start_sec: cursor_sec + sec_per_beat * 0.25,
                        duration_sec: chord_dur_sec * 0.88,
                        start_freq_hz: tenor_hz,
                        target_freq_hz: tenor_hz,
                        velocity: 0.68,
                        voice_line: 1,
                    });
                }

                // Bach WTC Prelude 16th-note figure: [0, 1, 2, 3, 4, 2, 3, 4] repeated
                let step_sec = sec_per_beat * 0.25; // 16th notes
                let num_steps = (chord.beats * 4.0).round() as usize;
                let seq = [0usize, 1, 2, 3, 4, 2, 3, 4];
                for s in 0..num_steps {
                    let deg = seq[s % seq.len()].min(arp_hz_pool.len().saturating_sub(1));
                    let hz = arp_hz_pool[deg];
                    // Let each note ring overlaps slightly (legato harp/clav resonance)
                    let hold = if s % 8 == 0 {
                        step_sec * 3.2
                    } else {
                        step_sec * 1.65
                    };
                    let vel = if s % 4 == 0 { 0.80 } else { 0.62 };
                    events.push(NoteEvent {
                        start_sec: cursor_sec + s as f64 * step_sec,
                        duration_sec: hold,
                        start_freq_hz: hz,
                        target_freq_hz: hz,
                        velocity: vel,
                        voice_line: 2 + (s % 3),
                    });
                }
            }

            MultilinePattern::CthulhuCascade => {
                // Layer 1: Warm sustained SATB voice-led FM pad bed
                events.push(NoteEvent {
                    start_sec: cursor_sec,
                    duration_sec: chord_dur_sec * 0.95,
                    start_freq_hz: start_bass_hz,
                    target_freq_hz: bass_hz,
                    velocity: 0.84,
                    voice_line: 0,
                });
                for (vi, &hz) in inner_hz.iter().take(4).enumerate() {
                    events.push(NoteEvent {
                        start_sec: cursor_sec,
                        duration_sec: chord_dur_sec * 0.92,
                        start_freq_hz: hz,
                        target_freq_hz: hz,
                        velocity: 0.54,
                        voice_line: vi + 1,
                    });
                }

                // Layer 2: Cthulhu-style multiline cascading arpeggio + counter-melody
                let step_sec = sec_per_beat * 0.25;
                let num_steps = (chord.beats * 4.0).round() as usize;
                let pattern_idx = [0usize, 2, 1, 3, 4, 3, 5, 2, 1, 4, 3, 6, 5, 3, 4, 2];
                let mut last_arp_hz = arp_hz_pool[0];
                for s in 0..num_steps {
                    let idx = pattern_idx[(s + chord_idx * 2) % pattern_idx.len()]
                        % arp_hz_pool.len().max(1);
                    let hz = arp_hz_pool[idx];
                    let accent = if s % 4 == 0 {
                        0.82
                    } else if s % 2 == 0 {
                        0.68
                    } else {
                        0.56
                    };
                    events.push(NoteEvent {
                        start_sec: cursor_sec + s as f64 * step_sec,
                        duration_sec: step_sec * 1.45,
                        start_freq_hz: last_arp_hz,
                        target_freq_hz: hz,
                        velocity: accent,
                        voice_line: 5,
                    });
                    last_arp_hz = hz;
                }
            }

            MultilinePattern::StrangerPulse => {
                // Driving 8th-note pulsing bass + sustained brassy chord swell + 16th upper synth pulse
                let eighth_sec = sec_per_beat * 0.5;
                let num_eighths = (chord.beats * 2.0).round() as usize;
                for e in 0..num_eighths {
                    let b_hz = if e % 4 == 3 { fifth_hz } else { bass_hz };
                    events.push(NoteEvent {
                        start_sec: cursor_sec + e as f64 * eighth_sec,
                        duration_sec: eighth_sec * 0.82,
                        start_freq_hz: b_hz,
                        target_freq_hz: b_hz,
                        velocity: if e % 2 == 0 { 0.88 } else { 0.72 },
                        voice_line: 0,
                    });
                }

                // Sustained warm FM pad chords
                for (vi, &hz) in inner_hz.iter().take(4).enumerate() {
                    events.push(NoteEvent {
                        start_sec: cursor_sec,
                        duration_sec: chord_dur_sec * 0.94,
                        start_freq_hz: hz,
                        target_freq_hz: hz,
                        velocity: 0.62,
                        voice_line: vi + 1,
                    });
                }

                // 16th-note retro-futuristic up-down ostinato
                let step_sec = sec_per_beat * 0.25;
                let num_steps = (chord.beats * 4.0).round() as usize;
                let ost = [0usize, 1, 2, 3, 4, 3, 2, 1];
                for s in 0..num_steps {
                    let deg = ost[s % ost.len()] % arp_hz_pool.len().max(1);
                    let hz = arp_hz_pool[deg];
                    events.push(NoteEvent {
                        start_sec: cursor_sec + s as f64 * step_sec,
                        duration_sec: step_sec * 0.95,
                        start_freq_hz: hz,
                        target_freq_hz: hz,
                        velocity: 0.72,
                        voice_line: 6,
                    });
                }
            }

            MultilinePattern::Polyrhythm3v4 => {
                // Deep pedal bass
                events.push(NoteEvent {
                    start_sec: cursor_sec,
                    duration_sec: chord_dur_sec * 0.96,
                    start_freq_hz: start_bass_hz,
                    target_freq_hz: bass_hz,
                    velocity: 0.85,
                    voice_line: 0,
                });

                // Alto Voice: 3-tuplets per 2 beats (Philip Glass rhythmic weave)
                let num_triplets = (chord.beats * 3.0).round() as usize;
                let trip_sec = chord_dur_sec / num_triplets.max(1) as f64;
                for t_i in 0..num_triplets {
                    let hz = inner_hz[t_i % inner_hz.len().max(1)];
                    events.push(NoteEvent {
                        start_sec: cursor_sec + t_i as f64 * trip_sec,
                        duration_sec: trip_sec * 1.15,
                        start_freq_hz: hz,
                        target_freq_hz: hz,
                        velocity: if t_i % 3 == 0 { 0.76 } else { 0.60 },
                        voice_line: 2,
                    });
                }

                // Soprano Voice: 4 sixteenth subdivisions against the 3-tuplets
                let num_quads = (chord.beats * 4.0).round() as usize;
                let quad_sec = chord_dur_sec / num_quads.max(1) as f64;
                for q_i in 0..num_quads {
                    let idx = (q_i * 2) % arp_hz_pool.len().max(1);
                    let hz = arp_hz_pool[idx];
                    events.push(NoteEvent {
                        start_sec: cursor_sec + q_i as f64 * quad_sec,
                        duration_sec: quad_sec * 1.20,
                        start_freq_hz: hz,
                        target_freq_hz: hz,
                        velocity: if q_i % 4 == 0 { 0.78 } else { 0.58 },
                        voice_line: 4,
                    });
                }
            }

            MultilinePattern::CounterpointFourVoice => {
                // Four independent contrapuntal voices that weave in contrary motion inside the chord
                events.push(NoteEvent {
                    start_sec: cursor_sec,
                    duration_sec: chord_dur_sec * 0.96,
                    start_freq_hz: start_bass_hz,
                    target_freq_hz: bass_hz,
                    velocity: 0.86,
                    voice_line: 0,
                });

                // Tenor moves in half-notes (ascending)
                let half_sec = chord_dur_sec * 0.5;
                let t0 = inner_hz[0];
                let t1 = inner_hz.get(1).copied().unwrap_or(t0 * 1.25);
                events.push(NoteEvent {
                    start_sec: cursor_sec,
                    duration_sec: half_sec * 0.96,
                    start_freq_hz: t0,
                    target_freq_hz: t0,
                    velocity: 0.74,
                    voice_line: 1,
                });
                events.push(NoteEvent {
                    start_sec: cursor_sec + half_sec,
                    duration_sec: half_sec * 0.96,
                    start_freq_hz: t0,
                    target_freq_hz: t1,
                    velocity: 0.76,
                    voice_line: 1,
                });

                // Alto moves in syncopated quarters (descending contrary motion)
                let q_sec = chord_dur_sec * 0.25;
                let a_notes = [
                    inner_hz.get(3).copied().unwrap_or(t1 * 1.5),
                    inner_hz.get(2).copied().unwrap_or(t1 * 1.25),
                    inner_hz.get(1).copied().unwrap_or(t1),
                    inner_hz.get(2).copied().unwrap_or(t1 * 1.25),
                ];
                let mut prev_a = a_notes[0];
                for (qi, &ahz) in a_notes.iter().enumerate() {
                    events.push(NoteEvent {
                        start_sec: cursor_sec + qi as f64 * q_sec,
                        duration_sec: q_sec * 0.98,
                        start_freq_hz: prev_a,
                        target_freq_hz: ahz,
                        velocity: 0.72,
                        voice_line: 2,
                    });
                    prev_a = ahz;
                }

                // Soprano weaves ornate 8th-note melodic ornamentation with smooth pitch glide
                let e_sec = chord_dur_sec * 0.125;
                let s_deg = [4usize, 5, 6, 5, 4, 3, 4, 5];
                let mut prev_s = arp_hz_pool[s_deg[0].min(arp_hz_pool.len() - 1)];
                for (ei, &d) in s_deg.iter().enumerate() {
                    let shz = arp_hz_pool[d.min(arp_hz_pool.len() - 1)];
                    events.push(NoteEvent {
                        start_sec: cursor_sec + ei as f64 * e_sec,
                        duration_sec: e_sec * 1.05,
                        start_freq_hz: prev_s,
                        target_freq_hz: shz,
                        velocity: if ei % 2 == 0 { 0.80 } else { 0.66 },
                        voice_line: 3,
                    });
                    prev_s = shz;
                }
            }
        }

        cursor_sec += chord_dur_sec;
    }

    (events, cursor_sec)
}
