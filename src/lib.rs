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

pub mod effects;
pub mod fm_synth;
pub mod json;
pub mod math_engine;
pub mod oscillator;
pub mod presets;
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
}
