//! 6-Operator FM & Hybrid Phase Modulation Matrix Synthesizer.
//! Inspired by Yamaha DX7, Native Instruments FM8, Xfer Serum, and Kilohearts Phase Plant.
//! Supports arbitrary 6x6 operator modulation matrices, self-feedback, per-operator ADSR envelopes,
//! LFO-driven harmonic index morphing, continuous portamento pitch glide, and hybrid math expressions.

use crate::math_engine::{CompiledExpr, MathContext};
use crate::oscillator::{AdsrSpec, LfoShape, LfoSpec, Waveform};
use std::f64::consts::TAU;

#[derive(Debug, Clone)]
pub struct OperatorSpec {
    pub index: usize,
    pub enabled: bool,
    pub ratio: f64,
    pub fixed_hz: Option<f64>,
    pub detune_cents: f64,
    pub waveform: Waveform,
    /// Carrier level sent to main stereo bus (0.0 = pure modulator, >0.0 = audible carrier)
    pub output_level: f64,
    /// Stereo pan (-1.0 left .. 0.0 center .. +1.0 right)
    pub pan: f64,
    /// Velocity sensitivity (0.0..=1.0)
    pub vel_sens: f64,
    /// High-note brightness damping (0.0 = flat, >0.0 = softer modulation on high notes)
    pub key_scale: f64,
    /// Self-modulation feedback in radians
    pub feedback: f64,
    pub env: AdsrSpec,
}

impl Default for OperatorSpec {
    fn default() -> Self {
        Self {
            index: 0,
            enabled: true,
            ratio: 1.0,
            fixed_hz: None,
            detune_cents: 0.0,
            waveform: Waveform::Sine,
            output_level: 0.0,
            pan: 0.0,
            vel_sens: 0.65,
            key_scale: 0.15,
            feedback: 0.0,
            env: AdsrSpec::default(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct FmPatch {
    pub id: String,
    pub name: String,
    pub category: String,
    pub description: String,
    pub algorithm_name: String,
    pub operators: [OperatorSpec; 6],
    /// `mod_matrix[src][dst]` = modulation index in radians from Operator `src` into Operator `dst`
    pub mod_matrix: [[f64; 6]; 6],
    /// How strongly `lfo1` scales each cell `mod_matrix[src][dst]` (enables evolving harmonic morphing!)
    pub lfo1_matrix_depth: [[f64; 6]; 6],
    pub lfo1: LfoSpec,
    pub lfo2: LfoSpec,
    /// Subtle pitch vibrato in cents from LFO2
    pub vibrato_cents: f64,
    /// Continuous portamento / pitch-glide time in seconds between voice notes
    pub glide_secs: f64,
    /// Unison stereo voices (1..=4) and unison detune spread in cents
    pub unison_voices: usize,
    pub unison_detune_cents: f64,
    /// Optional hybrid mathematical expression blended with or replacing the FM matrix
    pub math_expr: Option<String>,
    pub math_mix: f64,
    /// Filter & Effects send parameters attached to patch
    pub filter_cutoff_hz: f64,
    pub filter_resonance: f64,
    pub filter_env_depth_hz: f64,
    pub filter_lfo_depth_hz: f64,
    pub chorus_mix: f64,
    pub delay_mix: f64,
    pub delay_time_secs: f64,
    pub delay_feedback: f64,
    pub reverb_mix: f64,
    pub reverb_room_size: f64,
    pub reverb_shimmer: f64,
    pub master_drive: f64,
    /// Peak linear target after the master chain (0.92 ≈ −0.72 dBFS).
    pub normalize_peak: f64,
    /// TPDF dither amplitude when encoding 16-bit PCM (0 disables).
    pub dither_amp: f64,
    /// Soft-knee limiter ceiling in linear amplitude (1.0 = 0 dBFS).
    pub limiter_ceiling: f64,
}

impl Default for FmPatch {
    fn default() -> Self {
        Self::retro_classical_brass()
    }
}

impl FmPatch {
    pub fn empty_ops() -> [OperatorSpec; 6] {
        std::array::from_fn(|i| OperatorSpec {
            index: i,
            enabled: false,
            ..OperatorSpec::default()
        })
    }

    /// 1. Retro-Classical FM Brass & Strings (Vangelis CS-80 meets DX7 / FM8)
    pub fn retro_classical_brass() -> Self {
        let mut ops = Self::empty_ops();

        // Op 1: Warm fundamental carrier (left-biased stereo)
        ops[0] = OperatorSpec {
            index: 0,
            enabled: true,
            ratio: 1.0,
            detune_cents: -4.5,
            waveform: Waveform::Sine,
            output_level: 0.68,
            pan: -0.35,
            vel_sens: 0.55,
            key_scale: 0.1,
            feedback: 0.18,
            env: AdsrSpec {
                delay_secs: 0.0,
                attack_secs: 0.09,
                hold_secs: 0.05,
                decay_secs: 0.85,
                sustain_level: 0.78,
                release_secs: 0.85,
                curve: 1.3,
            },
            ..Default::default()
        };

        // Op 2: Brass harmonic modulator -> Op 1 (1:1 ratio for rich saw/brass spectrum)
        ops[1] = OperatorSpec {
            index: 1,
            enabled: true,
            ratio: 1.0,
            detune_cents: 2.0,
            waveform: Waveform::Sine,
            output_level: 0.0,
            pan: 0.0,
            vel_sens: 0.85,
            key_scale: 0.30,
            feedback: 0.42,
            env: AdsrSpec {
                delay_secs: 0.0,
                attack_secs: 0.16,
                hold_secs: 0.04,
                decay_secs: 0.90,
                sustain_level: 0.58,
                release_secs: 0.75,
                curve: 1.45,
            },
            ..Default::default()
        };

        // Op 3: Upper harmonic modulator -> Op 2 (2:1 octave brightness)
        ops[2] = OperatorSpec {
            index: 2,
            enabled: true,
            ratio: 2.0,
            detune_cents: -1.5,
            waveform: Waveform::Sine,
            output_level: 0.0,
            pan: 0.0,
            vel_sens: 0.90,
            key_scale: 0.40,
            feedback: 0.10,
            env: AdsrSpec {
                delay_secs: 0.0,
                attack_secs: 0.22,
                hold_secs: 0.0,
                decay_secs: 0.70,
                sustain_level: 0.38,
                release_secs: 0.60,
                curve: 1.6,
            },
            ..Default::default()
        };

        // Op 4: Lush stereo twin carrier (right-biased stereo, octave/unison warmth)
        ops[3] = OperatorSpec {
            index: 3,
            enabled: true,
            ratio: 1.0,
            detune_cents: 5.2,
            waveform: Waveform::GlassHarmonic,
            output_level: 0.58,
            pan: 0.38,
            vel_sens: 0.60,
            key_scale: 0.12,
            feedback: 0.12,
            env: AdsrSpec::pad(),
            ..Default::default()
        };

        // Op 5: Fifth/Octave shimmer modulator -> Op 4
        ops[4] = OperatorSpec {
            index: 4,
            enabled: true,
            ratio: 3.0,
            detune_cents: 3.0,
            waveform: Waveform::Sine,
            output_level: 0.0,
            pan: 0.0,
            vel_sens: 0.75,
            key_scale: 0.35,
            feedback: 0.0,
            env: AdsrSpec::modulator_swell(),
            ..Default::default()
        };

        // Op 6: Sub-octave foundation carrier
        ops[5] = OperatorSpec {
            index: 5,
            enabled: true,
            ratio: 0.5,
            detune_cents: 0.0,
            waveform: Waveform::Triangle,
            output_level: 0.32,
            pan: 0.0,
            vel_sens: 0.40,
            key_scale: 0.05,
            feedback: 0.25,
            env: AdsrSpec::pad(),
            ..Default::default()
        };

        let mut mod_matrix = [[0.0; 6]; 6];
        mod_matrix[1][0] = 2.15; // Op2 -> Op1 main brass warmth
        mod_matrix[2][1] = 1.05; // Op3 -> Op2 cascaded harmonic bite
        mod_matrix[4][3] = 1.35; // Op5 -> Op4 glassy 3rd harmonic evolution

        let mut lfo1_matrix_depth = [[0.0; 6]; 6];
        lfo1_matrix_depth[1][0] = 0.85; // Slow LFO morphs Op2->Op1 index!
        lfo1_matrix_depth[4][3] = 0.75; // Slow LFO morphs Op5->Op4 shimmer!

        Self {
            id: "retro_classical_brass".to_string(),
            name: "Retro-Classical FM Brass & Strings".to_string(),
            category: "Classical-Modern FM".to_string(),
            description: "6-Operator dual-branch FM brass/string hybrid where a slow LFO continuously morphs the operator modulation index over chords.".to_string(),
            algorithm_name: "Alg 2: Dual Cascaded Stereo Stack (Op3->Op2->Op1 + Op5->Op4 + Op6 Sub)".to_string(),
            operators: ops,
            mod_matrix,
            lfo1_matrix_depth,
            lfo1: LfoSpec {
                shape: LfoShape::SmoothDrift,
                rate_hz: 0.24,
                depth: 1.0,
                phase_offset_rad: 0.0,
                unipolar: false,
            },
            lfo2: LfoSpec {
                shape: LfoShape::Sine,
                rate_hz: 4.8,
                depth: 1.0,
                phase_offset_rad: 0.0,
                unipolar: false,
            },
            vibrato_cents: 3.2,
            glide_secs: 0.045,
            unison_voices: 2,
            unison_detune_cents: 6.5,
            math_expr: None,
            math_mix: 0.0,
            filter_cutoff_hz: 4800.0,
            filter_resonance: 0.28,
            filter_env_depth_hz: 2600.0,
            filter_lfo_depth_hz: 1100.0,
            chorus_mix: 0.32,
            delay_mix: 0.18,
            delay_time_secs: 0.375,
            delay_feedback: 0.36,
            reverb_mix: 0.34,
            reverb_room_size: 0.84,
            reverb_shimmer: 0.18,
            master_drive: 1.18,
            normalize_peak: 0.92,
            dither_amp: 1.0,
            limiter_ceiling: 0.98,
        }
    }

    /// 2. Glass Cathedral Crystalline FM (NI FM8 / Philip Glass / Cthulhu Arpeggio style)
    pub fn glass_cathedral() -> Self {
        let mut ops = Self::empty_ops();

        ops[0] = OperatorSpec {
            index: 0,
            enabled: true,
            ratio: 1.0,
            detune_cents: -3.0,
            waveform: Waveform::Sine,
            output_level: 0.72,
            pan: -0.25,
            vel_sens: 0.75,
            key_scale: 0.20,
            feedback: 0.08,
            env: AdsrSpec {
                delay_secs: 0.0,
                attack_secs: 0.008,
                hold_secs: 0.02,
                decay_secs: 1.15,
                sustain_level: 0.32,
                release_secs: 1.10,
                curve: 1.8,
            },
            ..Default::default()
        };

        ops[1] = OperatorSpec {
            index: 1,
            enabled: true,
            ratio: 2.0,
            detune_cents: 1.5,
            waveform: Waveform::Sine,
            output_level: 0.0,
            pan: 0.0,
            vel_sens: 0.85,
            key_scale: 0.35,
            feedback: 0.15,
            env: AdsrSpec {
                delay_secs: 0.0,
                attack_secs: 0.004,
                hold_secs: 0.01,
                decay_secs: 0.65,
                sustain_level: 0.22,
                release_secs: 0.70,
                curve: 2.0,
            },
            ..Default::default()
        };

        ops[2] = OperatorSpec {
            index: 2,
            enabled: true,
            ratio: 4.0,
            detune_cents: 4.0,
            waveform: Waveform::Sine,
            output_level: 0.0,
            pan: 0.0,
            vel_sens: 0.90,
            key_scale: 0.45,
            feedback: 0.0,
            env: AdsrSpec::pluck(),
            ..Default::default()
        };

        ops[3] = OperatorSpec {
            index: 3,
            enabled: true,
            ratio: 1.0,
            detune_cents: 3.5,
            waveform: Waveform::GlassHarmonic,
            output_level: 0.50,
            pan: 0.30,
            vel_sens: 0.60,
            key_scale: 0.15,
            feedback: 0.0,
            env: AdsrSpec::pad(),
            ..Default::default()
        };

        ops[4] = OperatorSpec {
            index: 4,
            enabled: true,
            ratio: 3.0,
            detune_cents: -2.0,
            waveform: Waveform::Sine,
            output_level: 0.0,
            pan: 0.0,
            vel_sens: 0.80,
            key_scale: 0.35,
            feedback: 0.0,
            env: AdsrSpec::modulator_swell(),
            ..Default::default()
        };

        ops[5] = OperatorSpec {
            index: 5,
            enabled: true,
            ratio: 0.5,
            detune_cents: 0.0,
            waveform: Waveform::Sine,
            output_level: 0.30,
            pan: 0.0,
            vel_sens: 0.35,
            key_scale: 0.0,
            feedback: 0.10,
            env: AdsrSpec::pad(),
            ..Default::default()
        };

        let mut mod_matrix = [[0.0; 6]; 6];
        mod_matrix[1][0] = 1.75;
        mod_matrix[2][0] = 0.85;
        mod_matrix[4][3] = 1.40;

        let mut lfo1_matrix_depth = [[0.0; 6]; 6];
        lfo1_matrix_depth[1][0] = 0.65;
        lfo1_matrix_depth[4][3] = 0.95;

        Self {
            id: "glass_cathedral".to_string(),
            name: "Glass Cathedral Hybrid Epiano & Pad".to_string(),
            category: "Classical-Modern FM".to_string(),
            description: "Crystalline FM8/DX7 glassy electric piano & bell attack layered over an evolving 3:1 harmonic pad.".to_string(),
            algorithm_name: "Alg 3: Glass Tree ((Op2+Op3)->Op1 + Op5->Op4 + Op6 Sub)".to_string(),
            operators: ops,
            mod_matrix,
            lfo1_matrix_depth,
            lfo1: LfoSpec {
                shape: LfoShape::Sine,
                rate_hz: 0.31,
                depth: 1.0,
                phase_offset_rad: 0.5,
                unipolar: false,
            },
            lfo2: LfoSpec {
                shape: LfoShape::Sine,
                rate_hz: 5.4,
                depth: 1.0,
                phase_offset_rad: 0.0,
                unipolar: false,
            },
            vibrato_cents: 2.0,
            glide_secs: 0.02,
            unison_voices: 2,
            unison_detune_cents: 5.0,
            math_expr: None,
            math_mix: 0.0,
            filter_cutoff_hz: 6800.0,
            filter_resonance: 0.22,
            filter_env_depth_hz: 3200.0,
            filter_lfo_depth_hz: 900.0,
            chorus_mix: 0.36,
            delay_mix: 0.26,
            delay_time_secs: 0.333,
            delay_feedback: 0.42,
            reverb_mix: 0.42,
            reverb_room_size: 0.90,
            reverb_shimmer: 0.28,
            master_drive: 1.12,
            normalize_peak: 0.92,
            dither_amp: 1.0,
            limiter_ceiling: 0.98,
        }
    }

    /// 3. Cybernetic Baroque Counterpoint (Clean polyphonic FM harpsichord/clav + warm cello matrix)
    pub fn cyber_baroque_harpsichord() -> Self {
        let mut ops = Self::empty_ops();

        ops[0] = OperatorSpec {
            index: 0,
            enabled: true,
            ratio: 1.0,
            detune_cents: -2.5,
            waveform: Waveform::ChebyshevOdd,
            output_level: 0.65,
            pan: -0.2,
            vel_sens: 0.70,
            key_scale: 0.20,
            feedback: 0.25,
            env: AdsrSpec {
                delay_secs: 0.0,
                attack_secs: 0.012,
                hold_secs: 0.03,
                decay_secs: 0.75,
                sustain_level: 0.48,
                release_secs: 0.55,
                curve: 1.6,
            },
            ..Default::default()
        };

        ops[1] = OperatorSpec {
            index: 1,
            enabled: true,
            ratio: 2.0,
            detune_cents: 2.5,
            waveform: Waveform::Sine,
            output_level: 0.0,
            pan: 0.0,
            vel_sens: 0.85,
            key_scale: 0.30,
            feedback: 0.30,
            env: AdsrSpec {
                delay_secs: 0.0,
                attack_secs: 0.005,
                hold_secs: 0.01,
                decay_secs: 0.45,
                sustain_level: 0.30,
                release_secs: 0.40,
                curve: 1.9,
            },
            ..Default::default()
        };

        ops[2] = OperatorSpec {
            index: 2,
            enabled: true,
            ratio: 3.0,
            detune_cents: 0.0,
            waveform: Waveform::Sine,
            output_level: 0.0,
            pan: 0.0,
            vel_sens: 0.80,
            key_scale: 0.40,
            feedback: 0.0,
            env: AdsrSpec::pluck(),
            ..Default::default()
        };

        ops[3] = OperatorSpec {
            index: 3,
            enabled: true,
            ratio: 1.0,
            detune_cents: 3.5,
            waveform: Waveform::SoftSaw,
            output_level: 0.52,
            pan: 0.25,
            vel_sens: 0.60,
            key_scale: 0.15,
            feedback: 0.15,
            env: AdsrSpec::brass(),
            ..Default::default()
        };

        ops[4] = OperatorSpec {
            index: 4,
            enabled: true,
            ratio: 1.5,
            detune_cents: 0.0,
            waveform: Waveform::Sine,
            output_level: 0.0,
            pan: 0.0,
            vel_sens: 0.75,
            key_scale: 0.35,
            feedback: 0.0,
            env: AdsrSpec::modulator_swell(),
            ..Default::default()
        };

        let mut mod_matrix = [[0.0; 6]; 6];
        mod_matrix[1][0] = 1.90;
        mod_matrix[2][1] = 1.15;
        mod_matrix[4][3] = 1.10;

        let mut lfo1_matrix_depth = [[0.0; 6]; 6];
        lfo1_matrix_depth[1][0] = 0.70;
        lfo1_matrix_depth[4][3] = 0.80;

        Self {
            id: "cyber_baroque_harpsichord".to_string(),
            name: "Cybernetic Baroque Lute & Bowed FM".to_string(),
            category: "Classical-Modern FM".to_string(),
            description: "Articulate Chebyshev & FM plucked transient morphing into a sustained bowed harmonic counterpoint voice.".to_string(),
            algorithm_name: "Alg 1: Baroque Hybrid Stack".to_string(),
            operators: ops,
            mod_matrix,
            lfo1_matrix_depth,
            lfo1: LfoSpec {
                shape: LfoShape::Triangle,
                rate_hz: 0.35,
                depth: 1.0,
                phase_offset_rad: 0.0,
                unipolar: false,
            },
            lfo2: LfoSpec::default(),
            vibrato_cents: 2.5,
            glide_secs: 0.035,
            unison_voices: 2,
            unison_detune_cents: 4.5,
            math_expr: None,
            math_mix: 0.0,
            filter_cutoff_hz: 5600.0,
            filter_resonance: 0.25,
            filter_env_depth_hz: 2400.0,
            filter_lfo_depth_hz: 950.0,
            chorus_mix: 0.25,
            delay_mix: 0.22,
            delay_time_secs: 0.25,
            delay_feedback: 0.34,
            reverb_mix: 0.32,
            reverb_room_size: 0.82,
            reverb_shimmer: 0.14,
            master_drive: 1.15,
            normalize_peak: 0.92,
            dither_amp: 1.0,
            limiter_ceiling: 0.98,
        }
    }

    /// 4. Mathematical Equation Hybrid Patch (Symbolic DSP + 6-Op FM Warmth)
    pub fn math_harmonic_weaver(formula: &str) -> Self {
        let mut base = Self::glass_cathedral();
        base.id = "math_harmonic_weaver".to_string();
        base.name = "Symbolic Math & FM Harmonic Weaver".to_string();
        base.category = "Mathematical DSP".to_string();
        base.description = format!("Hybrid symbolic equation (`{formula}`) blended with 6-operator FM warmth.");
        base.math_expr = Some(formula.to_string());
        base.math_mix = 0.78;
        base
    }

    /// Render a single polyphonic voice event (with optional pitch glide from `start_freq_hz` to `target_freq_hz`)
    /// into a stereo slice starting at `start_frame`.
    pub fn render_note_into(
        &self,
        left_bus: &mut [f64],
        right_bus: &mut [f64],
        sample_rate: u32,
        start_frame: usize,
        note_dur_secs: f64,
        start_freq_hz: f64,
        target_freq_hz: f64,
        velocity: f64,
        voice_idx: usize,
        compiled_math: Option<&CompiledExpr>,
    ) {
        let sr = sample_rate as f64;
        let max_rel = self
            .operators
            .iter()
            .filter(|o| o.enabled && o.output_level > 0.0)
            .map(|o| o.env.release_secs)
            .fold(0.8_f64, f64::max);

        let total_note_secs = note_dur_secs + max_rel;
        let total_frames = (total_note_secs * sr).ceil() as usize;
        let end_frame = (start_frame + total_frames).min(left_bus.len()).min(right_bus.len());
        if start_frame >= end_frame {
            return;
        }

        let unison = self.unison_voices.clamp(1, 4);
        let unison_norm = 1.0 / (unison as f64).sqrt();

        // Key-scale factor relative to A3 (220 Hz)
        let octaves_above_a3 = (target_freq_hz.max(20.0) / 220.0).log2().max(0.0);

        // State per unison sub-voice: 6 operator phases and 6 previous outputs
        let mut op_phases = [[0.0_f64; 6]; 4];
        let mut op_prev = [[0.0_f64; 6]; 4];

        // Slight stereo phase decorrelation per unison voice
        for u in 0..unison {
            for op_i in 0..6 {
                op_phases[u][op_i] = (u as f64 * 0.71 + voice_idx as f64 * 0.37) * 0.4;
            }
        }

        let glide_tau = self.glide_secs.max(0.0005);

        for frame in start_frame..end_frame {
            let t = (frame - start_frame) as f64 / sr;
            let gt = frame as f64 / sr;

            // Smooth exponential portamento glide from start_freq_hz to target_freq_hz
            let base_freq = if (start_freq_hz - target_freq_hz).abs() > 0.01 && self.glide_secs > 0.001 {
                let alpha = 1.0 - (-t / glide_tau).exp();
                start_freq_hz + (target_freq_hz - start_freq_hz) * alpha
            } else {
                target_freq_hz
            };

            let lfo1_val = self.lfo1.eval(gt);
            let lfo2_val = self.lfo2.eval(gt);

            // Pitch vibrato multiplier
            let vib_cents = self.vibrato_cents * lfo2_val;
            let vib_mult = (2.0_f64).powf(vib_cents / 1200.0);
            let inst_freq = base_freq * vib_mult;

            // Evaluate operator envelopes once per sample
            let mut op_env = [0.0_f64; 6];
            for i in 0..6 {
                let op = &self.operators[i];
                if op.enabled {
                    let vel_scale = (1.0 - op.vel_sens) + op.vel_sens * velocity;
                    let ks_atten = (-op.key_scale * octaves_above_a3 * 0.45).exp();
                    op_env[i] = op.env.eval(t, note_dur_secs) * vel_scale * ks_atten;
                }
            }

            let mut sample_l = 0.0_f64;
            let mut sample_r = 0.0_f64;

            if self.math_mix < 0.999 {
                let fm_weight = 1.0 - self.math_mix.clamp(0.0, 1.0);
                for u in 0..unison {
                    let u_pos = if unison == 1 {
                        0.0
                    } else {
                        (u as f64 / (unison - 1) as f64) * 2.0 - 1.0
                    };
                    let u_detune = u_pos * self.unison_detune_cents;

                    let mut curr_out = [0.0_f64; 6];

                    // Evaluate operators in reverse order (5 down to 0) so higher-numbered modulators
                    // feed lower-numbered carriers on the same sample, while reverse/feedback paths use `op_prev`.
                    for dst in (0..6).rev() {
                        let op = &self.operators[dst];
                        if !op.enabled {
                            continue;
                        }

                        // Sum phase modulation from all 6 operators into `dst`
                        let mut pm_rad = 0.0_f64;
                        for src in 0..6 {
                            let base_idx = self.mod_matrix[src][dst];
                            let lfo_idx = self.lfo1_matrix_depth[src][dst] * lfo1_val;
                            let eff_idx = (base_idx + lfo_idx).max(0.0);
                            if eff_idx > 1e-6 {
                                let src_sig = if src > dst {
                                    curr_out[src]
                                } else {
                                    op_prev[u][src]
                                };
                                pm_rad += eff_idx * src_sig;
                            }
                        }

                        let op_freq = match op.fixed_hz {
                            Some(hz) => hz,
                            None => inst_freq * op.ratio,
                        } * (2.0_f64).powf((op.detune_cents + u_detune) / 1200.0);

                        let phase = op_phases[u][dst];
                        let raw = op.waveform.sample(phase, pm_rad, op_prev[u][dst], op.feedback);
                        let sig = raw * op_env[dst];
                        curr_out[dst] = sig;

                        // Advance phase
                        op_phases[u][dst] = (phase + TAU * op_freq / sr).rem_euclid(TAU * 1024.0);

                        if op.output_level > 0.0 {
                            let pan = (op.pan + 0.22 * u_pos).clamp(-1.0, 1.0);
                            let angle = (pan + 1.0) * std::f64::consts::FRAC_PI_4;
                            let gain = op.output_level * unison_norm * fm_weight;
                            sample_l += sig * gain * angle.cos();
                            sample_r += sig * gain * angle.sin();
                        }
                    }

                    op_prev[u] = curr_out;
                }
            }

            // Optional Symbolic Mathematical Expression Layer
            if let Some(expr) = compiled_math {
                if self.math_mix > 0.001 {
                    let main_env = op_env[0].max(AdsrSpec::pad().eval(t, note_dur_secs));
                    let dyn_mod_idx = (2.0 + 1.4 * lfo1_val).max(0.1);
                    let ctx = MathContext::for_sample(
                        t,
                        gt,
                        inst_freq,
                        velocity,
                        main_env,
                        voice_idx,
                        dyn_mod_idx,
                    );
                    let math_val = expr.eval(&ctx).clamp(-2.0, 2.0) * 0.45 * self.math_mix;
                    // Slight stereo spread by voice index
                    let v_pan = ((voice_idx as f64 % 4.0) - 1.5) * 0.22;
                    let angle = (v_pan + 1.0) * std::f64::consts::FRAC_PI_4;
                    sample_l += math_val * angle.cos();
                    sample_r += math_val * angle.sin();
                }
            }

            left_bus[frame] += sample_l;
            right_bus[frame] += sample_r;
        }
    }
}
