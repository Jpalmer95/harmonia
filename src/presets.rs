//! Curated Flagship Compositions, Synth Patches, and Complete Rendering Pipeline.

use crate::effects::process_master_chain;
use crate::fm_synth::FmPatch;
use crate::math_engine::CompiledExpr;
use crate::theory::{
    generate_multiline_events, parse_progression, MultilinePattern, TuningSystem,
};
use crate::wav::StereoBuffer;

#[derive(Debug, Clone)]
pub struct CompositionSpec {
    pub id: String,
    pub title: String,
    pub subtitle: String,
    pub description: String,
    pub bpm: f64,
    pub progression: String,
    pub beats_per_chord: f64,
    pub pattern: MultilinePattern,
    pub tuning: TuningSystem,
    pub patch: FmPatch,
}

impl CompositionSpec {
    /// Render this composition into a Hi-Def `StereoBuffer` at `sample_rate` (e.g., 48000 or 96000 Hz).
    pub fn render(&self, sample_rate: u32) -> Result<StereoBuffer, String> {
        let chords = parse_progression(&self.progression, self.beats_per_chord);
        let (events, seq_dur_sec) =
            generate_multiline_events(&chords, self.bpm, self.pattern, self.tuning.clone());

        let compiled_math = if let Some(ref expr_str) = self.patch.math_expr {
            if !expr_str.trim().is_empty() && self.patch.math_mix > 0.001 {
                Some(CompiledExpr::compile(expr_str)?)
            } else {
                None
            }
        } else {
            None
        };

        // Add tail time for release and cathedral reverb bloom
        let tail_secs = 2.4;
        let total_secs = (seq_dur_sec + tail_secs).max(1.0);
        let total_frames = (total_secs * sample_rate as f64).ceil() as usize;

        let mut buf = StereoBuffer::new(sample_rate, total_frames);

        for ev in &events {
            let start_frame = (ev.start_sec * sample_rate as f64).round() as usize;
            self.patch.render_note_into(
                &mut buf.left,
                &mut buf.right,
                sample_rate,
                start_frame,
                ev.duration_sec,
                ev.start_freq_hz,
                ev.target_freq_hz,
                ev.velocity,
                ev.voice_line,
                compiled_math.as_ref(),
            );
        }

        process_master_chain(&mut buf, &self.patch);
        Ok(buf)
    }
}

pub fn builtin_patches() -> Vec<FmPatch> {
    vec![
        FmPatch::retro_classical_brass(),
        FmPatch::glass_cathedral(),
        FmPatch::cyber_baroque_harpsichord(),
        FmPatch::math_harmonic_weaver(
            "env * (0.65 * fm(f, f * 2.0, mod_idx * (0.6 + 0.4 * lfo1), t) + 0.35 * cheb(3, sin(w)))",
        ),
    ]
}

pub fn find_patch(id: &str) -> Option<FmPatch> {
    let key = id.trim().to_ascii_lowercase();
    builtin_patches()
        .into_iter()
        .find(|p| p.id.to_ascii_lowercase() == key || p.id.contains(&key))
}

pub fn builtin_compositions() -> Vec<CompositionSpec> {
    vec![
        // 1. Bach Meets Vangelis — Flagship Retro-Classical Modulated FM Multiline Progression
        CompositionSpec {
            id: "bach_meets_vangelis".to_string(),
            title: "Bach Meets Vangelis (FM Brass & Cthulhu Weave)".to_string(),
            subtitle: "6-Op FM Brass/Strings + Slow LFO Index Morph + Cthulhu Cascade".to_string(),
            description: "The quintessential retro-classical meets modern sound: a 6-operator FM brass/string hybrid where a slow quasi-periodic LFO continuously morphs the operator modulation depth across a dramatic minor-9th & Lydian-#11 multiline progression.".to_string(),
            bpm: 108.0,
            progression: "Cm9 -> Abmaj9#11 -> Fm11 -> G7b9 -> Cm9 -> Ebmaj9#11 -> Dm7b5 -> G7alt".to_string(),
            beats_per_chord: 4.0,
            pattern: MultilinePattern::CthulhuCascade,
            tuning: TuningSystem::Equal12 { a4_hz: 440.0 },
            patch: FmPatch::retro_classical_brass(),
        },
        // 2. Glass Cathedral in Pure Just Intonation
        CompositionSpec {
            id: "glass_cathedral_ji".to_string(),
            title: "Crystalline Just Intonation Cathedral".to_string(),
            subtitle: "Pure 5/7-Limit Harmonic Ratios + Bach WTC Prelude Weave + Glass FM".to_string(),
            description: "Tuned in pure Ptolemaic Just Intonation so every third, fifth, and seventh locks into beatless integer ratios while interlocking Bach Prelude 16th-note voices shimmer through an FM8-style glass electric piano & pad.".to_string(),
            bpm: 114.0,
            progression: "Fmaj9#11 -> Dm11 -> Bbmaj9 -> C13sus4 -> Am9 -> Gm11 -> Ebmaj9#11 -> C9sus4".to_string(),
            beats_per_chord: 4.0,
            pattern: MultilinePattern::BachPrelude,
            tuning: TuningSystem::JustIntonation {
                root_pc: 5, // F
                a4_hz: 440.0,
            },
            patch: FmPatch::glass_cathedral(),
        },
        // 3. Cybernetic Brandenburg 2084 (4-Voice Contrapuntal Portamento)
        CompositionSpec {
            id: "cyber_brandenburg_2084".to_string(),
            title: "Cybernetic Brandenburg 2084".to_string(),
            subtitle: "4-Voice Contrary Counterpoint + Continuous Portamento Pitch Glide".to_string(),
            description: "Four independent polyphonic FM/Chebyshev voices (Bass, Tenor, Alto, Soprano) moving at staggered rhythms with parsimonious voice-leading and continuous pitch glide.".to_string(),
            bpm: 116.0,
            progression: "Dm9 -> Gm9 -> C9 -> Fmaj9 -> Bbmaj7#11 -> Em7b5 -> A7b9 -> Dmadd9".to_string(),
            beats_per_chord: 4.0,
            pattern: MultilinePattern::CounterpointFourVoice,
            tuning: TuningSystem::JustIntonation {
                root_pc: 2, // D
                a4_hz: 440.0,
            },
            patch: FmPatch::cyber_baroque_harpsichord(),
        },
        // 4. Stranger Chronicles 1984 (Driving Synth-Classical Pulse)
        CompositionSpec {
            id: "stranger_chronicles_1984".to_string(),
            title: "Stranger Chronicles 1984".to_string(),
            subtitle: "Analog-FM Pulsing Bass + 16th Ostinato + ZDF Filter & Index Swell".to_string(),
            description: "Driving 8th-note sub-bass and 16th-note synth-classical ostinato paired with warm voice-led FM brass pads and resonant zero-delay feedback filter sweeps.".to_string(),
            bpm: 118.0,
            progression: "Am9 -> Fmaj7#11 -> Cmaj9/E -> Dm11 -> Am9 -> Fmaj9#11 -> Dm9 -> E7b9".to_string(),
            beats_per_chord: 4.0,
            pattern: MultilinePattern::StrangerPulse,
            tuning: TuningSystem::Equal12 { a4_hz: 440.0 },
            patch: {
                let mut p = FmPatch::retro_classical_brass();
                p.lfo1.rate_hz = 0.38;
                p.filter_cutoff_hz = 3800.0;
                p.filter_lfo_depth_hz = 1850.0;
                p
            },
        },
        // 5. Golden Ratio Phi & Chebyshev Mathematical Symphony
        CompositionSpec {
            id: "golden_phi_cosmos".to_string(),
            title: "Golden Ratio (Phi) & Chebyshev Equation Symphony".to_string(),
            subtitle: "Symbolic Math Expression + Chebyshev Polynomials + Polyrhythm 3v4".to_string(),
            description: "Synthesized directly from a live mathematical equation combining Golden Ratio (phi = 1.618034) sideband FM and odd-order Chebyshev polynomials in a 3-against-4 polyrhythmic weave.".to_string(),
            bpm: 102.0,
            progression: "Em9 -> Cmaj9#11 -> Am11 -> B7b9 -> Em9 -> Gmaj9#11 -> F#m7b5 -> B7alt".to_string(),
            beats_per_chord: 4.0,
            pattern: MultilinePattern::Polyrhythm3v4,
            tuning: TuningSystem::JustIntonation {
                root_pc: 4, // E
                a4_hz: 432.0,
            },
            patch: FmPatch::math_harmonic_weaver(
                "env * (0.48 * fm(f, f * phi, mod_idx * (0.5 + 0.5 * lfo1), t) + 0.32 * cheb(3, sin(w)) + 0.20 * harm(w, 6, 1.3))",
            ),
        },
        // 6. Ptolemaic Pure Ratio Harmonic Choir (Sustained Cathedral)
        CompositionSpec {
            id: "ptolemaic_pure_choir".to_string(),
            title: "Ptolemaic Pure Ratio Harmonic Cathedral".to_string(),
            subtitle: "Neo-Riemannian Chromatic Mediants in Beatless Just Intonation".to_string(),
            description: "Lush sustained cathedral chords traversing Neo-Riemannian chromatic mediants (Cmaj9 -> Abmaj9#11 -> Fmaj9 -> Dbmaj9#11) with slow FM harmonic morphing and octave shimmer reverb.".to_string(),
            bpm: 88.0,
            progression: "Cmaj9 -> Abmaj9#11 -> Fmaj9 -> Dbmaj9#11 -> Bbm11 -> Gbmaj9#11 -> Ebmaj9 -> G7sus4".to_string(),
            beats_per_chord: 4.0,
            pattern: MultilinePattern::SustainedCathedral,
            tuning: TuningSystem::JustIntonation {
                root_pc: 0,
                a4_hz: 440.0,
            },
            patch: {
                let mut p = FmPatch::glass_cathedral();
                p.operators[0].env.attack_secs = 0.28;
                p.reverb_mix = 0.48;
                p.reverb_shimmer = 0.34;
                p.glide_secs = 0.085;
                p
            },
        },
        // 7. Lute at the Edge of the Grid — Karplus + 31-EDO
        CompositionSpec {
            id: "lute_at_the_grid_edge".to_string(),
            title: "Lute at the Edge of the Grid".to_string(),
            subtitle: "Karplus–Strong lute + 31-EDO microtone cascade".to_string(),
            description: "Plucked-string physical model (ks) driving a 31-tone equal temperament weave — Renaissance lute meeting a modern grid.".to_string(),
            bpm: 96.0,
            progression: "Am9 -> Dm9 -> G7 -> Cmaj9 -> Fmaj7#11 -> Bm7b5 -> E7b9 -> Amadd9".to_string(),
            beats_per_chord: 4.0,
            pattern: MultilinePattern::BachPrelude,
            tuning: TuningSystem::from_str("edo31", 9),
            patch: FmPatch::math_harmonic_weaver(
                "env * (0.70 * ks(f, t, 0.48) + 0.30 * sin(w * 0.5))",
            ),
        },
        // 8. Neon Organum — DX7 pairs + Werckmeister
        CompositionSpec {
            id: "neon_organum".to_string(),
            title: "Neon Organum".to_string(),
            subtitle: "DX7 three-pair algorithm on Werckmeister III well temperament".to_string(),
            description: "Yamaha algorithm 5 (three modulator/carrier pairs) voiced through historical well temperament — baroque organ glow with neon chorus.".to_string(),
            bpm: 84.0,
            progression: "Dm -> Bbmaj7 -> Fmaj9 -> Csus4 -> Gm9 -> Ebmaj7 -> A7b9 -> Dm".to_string(),
            beats_per_chord: 4.0,
            pattern: MultilinePattern::SustainedCathedral,
            tuning: TuningSystem::from_str("werckmeister_iii", 2),
            patch: {
                let mut p = FmPatch::glass_cathedral();
                crate::dx7::apply_to_patch(&mut p, 5, 1.7);
                p.reverb_mix = 0.44;
                p.chorus_mix = 0.38;
                p
            },
        },
        // 9. Bohlen–Pierce Lambda Choir
        CompositionSpec {
            id: "lambda_tritave_choir".to_string(),
            title: "Lambda Tritave Choir".to_string(),
            subtitle: "Bohlen–Pierce 13-ED3 + harmonic-series pad".to_string(),
            description: "Non-octave tritave harmony (period 3/1) sung by glass FM carriers — alien but consonant if you sit with it.".to_string(),
            bpm: 72.0,
            progression: "Cmaj9 -> Gsus4 -> Fmaj7 -> Dm9 -> Bbsus2 -> Cmaj9".to_string(),
            beats_per_chord: 6.0,
            pattern: MultilinePattern::SustainedCathedral,
            tuning: TuningSystem::from_str("bp_lambda", 0),
            patch: FmPatch::glass_cathedral(),
        },
        // 10. Buchla Fold at Dusk
        CompositionSpec {
            id: "buchla_fold_dusk".to_string(),
            title: "Buchla Fold at Dusk".to_string(),
            subtitle: "West-coast wavefolder lead over stranger-pulse bass".to_string(),
            description: "Sine-folding Buchla timbre (math VM) blended with the 1984 pulse pattern — dusk-colored FM grit.".to_string(),
            bpm: 104.0,
            progression: "Em9 -> Cmaj9 -> Gmaj7 -> D9 -> Am11 -> B7alt".to_string(),
            beats_per_chord: 4.0,
            pattern: MultilinePattern::StrangerPulse,
            tuning: TuningSystem::Equal12 { a4_hz: 440.0 },
            patch: FmPatch::math_harmonic_weaver(
                "env * (0.7 * fold(sin(w) + 0.35 * sin(w * 2.002), 1.5 + mod_idx * (0.6 + 0.4 * lfo1)) + 0.3 * sin(w * 0.5))",
            ),
        },
    ]
}

pub fn find_composition(id: &str) -> Option<CompositionSpec> {
    let key = id.trim().to_ascii_lowercase();
    builtin_compositions()
        .into_iter()
        .find(|c| c.id.to_ascii_lowercase() == key || c.id.contains(&key))
}
