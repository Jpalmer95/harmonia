//! # Harmonia
//!
//! High-Definition 6-Operator FM Matrix Synthesizer, Symbolic Mathematical DSP Engine,
//! and Multiline Chord Progression Workstation in Pure Rust.
//!
//! ## Modular Extension Traits
//! Developers can extend Harmonia by implementing:
//! - [`CustomSignalGenerator`] for new oscillators or physical models
//! - [`CustomAudioEffect`] for custom DSP post-processors
//! - [`math_engine::CompiledExpr`] for arbitrary mathematical equations at sample rate

pub mod dx7;
pub mod effects;
pub mod fm_synth;
pub mod json;
pub mod karplus;
pub mod math_engine;
pub mod midi;
pub mod oscillator;
pub mod presets;
pub mod scala;
pub mod server;
pub mod theory;
pub mod wav;

use crate::wav::StereoBuffer;

/// Trait for plugging custom audio generators / voices into Harmonia.
pub trait CustomSignalGenerator: Send + Sync {
    fn name(&self) -> &str;
    fn sample(&mut self, time_sec: f64, freq_hz: f64, velocity: f64) -> (f64, f64);
}

/// Trait for plugging custom stereo DSP effects into Harmonia's mastering chain.
pub trait CustomAudioEffect: Send + Sync {
    fn name(&self) -> &str;
    fn process(&mut self, buffer: &mut StereoBuffer);
}

#[cfg(test)]
mod tests {
    use crate::math_engine::{CompiledExpr, MathContext};
    use crate::presets::{builtin_compositions, CompositionSpec};
    use crate::theory::{
        generate_multiline_events, parse_progression, MultilinePattern, TuningSystem,
    };
    use crate::wav::BitDepth;

    #[test]
    fn test_math_engine_compilation_and_eval() {
        let expr = CompiledExpr::compile(
            "env * (0.6 * fm(f, f * 2.0, mod_idx * (0.5 + 0.5 * lfo1), t) + 0.4 * cheb(3, sin(w)))",
        )
        .expect("Math formula should compile cleanly");

        let ctx = MathContext::for_sample(0.05, 0.05, 220.0, 0.9, 0.85, 0, 2.2);
        let val = expr.eval(&ctx);
        assert!(val.is_finite(), "Math expression output must be finite");
        assert!(
            val.abs() <= 2.0,
            "Math expression output should be within expected range, got {val}"
        );
    }

    #[test]
    fn test_chord_parser_and_voice_leading() {
        let chords = parse_progression("Cm9 -> Abmaj9#11 -> Fm11 -> G7b9", 4.0);
        assert_eq!(chords.len(), 4);
        assert_eq!(chords[0].root_pc, 0); // C
        assert_eq!(chords[1].root_pc, 8); // Ab
        assert_eq!(chords[2].root_pc, 5); // F
        assert_eq!(chords[3].root_pc, 7); // G

        let (events, total_sec) = generate_multiline_events(
            &chords,
            120.0,
            MultilinePattern::CthulhuCascade,
            TuningSystem::JustIntonation {
                root_pc: 0,
                a4_hz: 440.0,
            },
        );
        assert!(events.len() > 40, "Should generate rich multiline note events");
        assert!((total_sec - 8.0).abs() < 0.01);
    }

    #[test]
    fn test_just_intonation_pure_ratios() {
        let ji = TuningSystem::JustIntonation {
            root_pc: 9, // A
            a4_hz: 440.0,
        };
        let root_a4 = ji.midi_to_hz(69.0, Some(9));
        let maj_third_c_sharp5 = ji.midi_to_hz(73.0, Some(9));
        let perf_fifth_e5 = ji.midi_to_hz(76.0, Some(9));

        assert!((root_a4 - 440.0).abs() < 1e-6);
        // Pure 5:4 major third = 550.0 Hz (instead of 12-TET 554.365 Hz!)
        assert!((maj_third_c_sharp5 - 550.0).abs() < 1e-6);
        // Pure 3:2 perfect fifth = 660.0 Hz
        assert!((perf_fifth_e5 - 660.0).abs() < 1e-6);
    }

    #[test]
    fn test_full_composition_render_and_json_roundtrip() {
        let mut comp = builtin_compositions().into_iter().next().unwrap();
        comp.beats_per_chord = 1.0; // Short render for fast unit test
        comp.progression = "Cm9 -> Abmaj9#11".to_string();

        let json_str = comp.to_pretty_json();
        let restored = CompositionSpec::from_json_str(&json_str)
            .expect("JSON roundtrip of CompositionSpec should succeed");
        assert_eq!(restored.id, comp.id);
        assert_eq!(restored.progression, comp.progression);

        let stereo = restored.render(24000).expect("Render should succeed");
        let report = stereo.analyze();
        assert!(report.peak_linear > 0.5 && report.peak_linear <= 1.0);
        assert!(report.rms_linear > 0.02);

        let wav_bytes = stereo.to_wav_bytes(BitDepth::Pcm24);
        assert_eq!(&wav_bytes[0..4], b"RIFF");
        assert_eq!(&wav_bytes[8..12], b"WAVE");
    }

    #[test]
    fn test_midi_roundtrip_and_scala_edo31() {
        use crate::midi::{encode_smf, hz_to_midi, parse_smf, MidiFile};
        use crate::scala::{parse_scl, equal_temperament};

        let scl = parse_scl("! test\n31 edo\n31\n38.70968\n77.41935\n116.12903\n154.83871\n193.54839\n232.25806\n270.96774\n309.67742\n348.38710\n387.09677\n425.80645\n464.51613\n503.22581\n541.93548\n580.64516\n619.35484\n658.06452\n696.77419\n735.48387\n774.19355\n812.90323\n851.61290\n890.32258\n929.03226\n967.74194\n1006.45161\n1045.16129\n1083.87097\n1122.58065\n1161.29032\n1200.0\n")
            .expect("scl parse");
        assert_eq!(scl.degrees(), 31);
        let edo = equal_temperament(31, 2.0);
        assert!((edo.period_ratio() - 2.0).abs() < 1e-9);

        let chords = parse_progression("Am9 -> Fmaj7", 2.0);
        let (events, _) = generate_multiline_events(
            &chords,
            100.0,
            MultilinePattern::SustainedCathedral,
            TuningSystem::Equal12 { a4_hz: 440.0 },
        );
        let midi = MidiFile::from_note_events(&events, 100.0, |ev| hz_to_midi(ev.target_freq_hz));
        let bytes = encode_smf(&midi);
        let back = parse_smf(&bytes).expect("midi parse");
        assert!(!back.notes.is_empty());
        assert!((back.bpm() - 100.0).abs() < 1.0);
    }

    #[test]
    fn test_dx7_algorithms_and_karplus() {
        let a5 = crate::dx7::algorithm(5);
        assert_eq!(a5.number, 5);
        assert!(a5.matrix[1][0] > 0.0);
        let a32 = crate::dx7::algorithm(32);
        assert_eq!(a32.carriers.len(), 6);
        let y = crate::karplus::ks_waveform(220.0, 0.02, 0.4);
        assert!(y.is_finite());
    }
}
