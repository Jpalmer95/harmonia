//! Core Oscillators, Multi-Stage Curved ADSR Envelopes, and LFO Modulators.

use std::f64::consts::TAU;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Waveform {
    Sine,
    FeedbackSine,
    Triangle,
    SoftSaw,
    SuperSaw,
    Pulse,
    GlassHarmonic,
    ChebyshevOdd,
    FoldedSine,
    CustomMath,
}

impl Waveform {
    pub fn from_str(s: &str) -> Self {
        match s.trim().to_ascii_lowercase().as_str() {
            "sine" | "sin" => Waveform::Sine,
            "fbsine" | "feedback" | "feedback_sine" => Waveform::FeedbackSine,
            "tri" | "triangle" => Waveform::Triangle,
            "saw" | "softsaw" | "soft_saw" => Waveform::SoftSaw,
            "supersaw" | "super_saw" => Waveform::SuperSaw,
            "pulse" | "square" | "sq" => Waveform::Pulse,
            "glass" | "glass_harmonic" | "harmonic" => Waveform::GlassHarmonic,
            "cheb" | "chebyshev" | "chebyshev_odd" => Waveform::ChebyshevOdd,
            "fold" | "folded" | "folded_sine" => Waveform::FoldedSine,
            "math" | "custom" | "custom_math" => Waveform::CustomMath,
            _ => Waveform::Sine,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Waveform::Sine => "sine",
            Waveform::FeedbackSine => "feedback_sine",
            Waveform::Triangle => "triangle",
            Waveform::SoftSaw => "soft_saw",
            Waveform::SuperSaw => "super_saw",
            Waveform::Pulse => "pulse",
            Waveform::GlassHarmonic => "glass_harmonic",
            Waveform::ChebyshevOdd => "chebyshev_odd",
            Waveform::FoldedSine => "folded_sine",
            Waveform::CustomMath => "custom_math",
        }
    }

    /// Evaluate waveform at phase `phase_rad` (in radians) with optional phase modulation `pm_rad`
    /// and previous sample feedback `prev_out` scaled by `feedback`.
    #[inline]
    pub fn sample(
        &self,
        phase_rad: f64,
        pm_rad: f64,
        prev_out: f64,
        feedback: f64,
    ) -> f64 {
        let p = phase_rad + pm_rad + feedback * prev_out;
        match self {
            Waveform::Sine | Waveform::FeedbackSine => p.sin(),
            Waveform::Triangle => {
                let u = (p / TAU).rem_euclid(1.0);
                1.0 - 4.0 * (u - 0.5).abs()
            }
            Waveform::SoftSaw => {
                // Smooth harmonic saw (rich without harsh Nyquist aliasing when used as FM operator)
                p.sin()
                    - 0.5 * (2.0 * p).sin()
                    + (1.0 / 3.0) * (3.0 * p).sin()
                    - 0.25 * (4.0 * p).sin()
                    + 0.2 * (5.0 * p).sin()
                    - (1.0 / 6.0) * (6.0 * p).sin()
            }
            Waveform::SuperSaw => {
                let s1 = p.sin() - 0.5 * (2.0 * p).sin() + 0.33 * (3.0 * p).sin();
                let s2 = (p * 1.006).sin() - 0.5 * (2.0 * p * 1.006).sin();
                let s3 = (p * 0.994).sin() - 0.5 * (2.0 * p * 0.994).sin();
                (s1 + 0.7 * s2 + 0.7 * s3) * 0.45
            }
            Waveform::Pulse => (3.5 * p.sin()).tanh(),
            Waveform::GlassHarmonic => {
                // Crystalline 1st, 2nd, 3rd, 5th, 8th Fibonacci/octaval partials
                0.52 * p.sin()
                    + 0.22 * (2.0 * p).sin()
                    + 0.14 * (3.0 * p).sin()
                    + 0.08 * (5.0 * p).sin()
                    + 0.04 * (8.0 * p).sin()
            }
            Waveform::ChebyshevOdd => {
                let x = p.sin();
                let t3 = 4.0 * x * x * x - 3.0 * x;
                let t5 = 16.0 * x.powi(5) - 20.0 * x.powi(3) + 5.0 * x;
                0.62 * x + 0.26 * t3 + 0.12 * t5
            }
            Waveform::FoldedSine => (2.2 * p.sin()).sin(),
            Waveform::CustomMath => p.sin(),
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct AdsrSpec {
    pub delay_secs: f64,
    pub attack_secs: f64,
    pub hold_secs: f64,
    pub decay_secs: f64,
    pub sustain_level: f64,
    pub release_secs: f64,
    /// Curve exponent: < 1.0 is fast/convex, 1.0 is linear, > 1.0 is exponential/concave
    pub curve: f64,
}

impl Default for AdsrSpec {
    fn default() -> Self {
        Self {
            delay_secs: 0.0,
            attack_secs: 0.04,
            hold_secs: 0.0,
            decay_secs: 0.45,
            sustain_level: 0.72,
            release_secs: 0.65,
            curve: 1.4,
        }
    }
}

impl AdsrSpec {
    pub fn pad() -> Self {
        Self {
            delay_secs: 0.0,
            attack_secs: 0.35,
            hold_secs: 0.10,
            decay_secs: 1.20,
            sustain_level: 0.80,
            release_secs: 1.40,
            curve: 1.25,
        }
    }

    pub fn brass() -> Self {
        Self {
            delay_secs: 0.0,
            attack_secs: 0.08,
            hold_secs: 0.04,
            decay_secs: 0.55,
            sustain_level: 0.76,
            release_secs: 0.50,
            curve: 1.35,
        }
    }

    pub fn pluck() -> Self {
        Self {
            delay_secs: 0.0,
            attack_secs: 0.004,
            hold_secs: 0.01,
            decay_secs: 0.65,
            sustain_level: 0.08,
            release_secs: 0.45,
            curve: 2.2,
        }
    }

    pub fn modulator_swell() -> Self {
        Self {
            delay_secs: 0.0,
            attack_secs: 0.18,
            hold_secs: 0.05,
            decay_secs: 0.95,
            sustain_level: 0.58,
            release_secs: 0.75,
            curve: 1.5,
        }
    }

    /// Evaluate the envelope at `t` seconds since note-on, where note-off occurs at `note_dur_secs`.
    #[inline]
    pub fn eval(&self, t: f64, note_dur_secs: f64) -> f64 {
        if t < self.delay_secs {
            return 0.0;
        }
        let active_t = t - self.delay_secs;
        let on_dur = (note_dur_secs - self.delay_secs).max(0.005);

        if active_t <= on_dur {
            self.eval_sustain_phase(active_t)
        } else {
            let rel_t = active_t - on_dur;
            let rel_dur = self.release_secs.max(0.002);
            if rel_t >= rel_dur {
                0.0
            } else {
                let level_at_off = self.eval_sustain_phase(on_dur);
                let u = 1.0 - (rel_t / rel_dur).clamp(0.0, 1.0);
                level_at_off * u.powf(self.curve.max(0.2))
            }
        }
    }

    #[inline]
    fn eval_sustain_phase(&self, t: f64) -> f64 {
        let atk = self.attack_secs.max(0.001);
        if t < atk {
            let u = (t / atk).clamp(0.0, 1.0);
            return u.powf(1.0 / self.curve.max(0.25));
        }
        let t_after_atk = t - atk;
        if t_after_atk < self.hold_secs {
            return 1.0;
        }
        let t_dec = t_after_atk - self.hold_secs;
        let dec = self.decay_secs.max(0.001);
        if t_dec < dec {
            let u = 1.0 - (t_dec / dec).clamp(0.0, 1.0);
            self.sustain_level + (1.0 - self.sustain_level) * u.powf(self.curve.max(0.2))
        } else {
            self.sustain_level
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LfoShape {
    Sine,
    Triangle,
    SawUp,
    SawDown,
    Square,
    SmoothDrift,
    SwellArch,
}

impl LfoShape {
    pub fn from_str(s: &str) -> Self {
        match s.trim().to_ascii_lowercase().as_str() {
            "tri" | "triangle" => LfoShape::Triangle,
            "saw" | "saw_up" => LfoShape::SawUp,
            "saw_down" | "ramp_down" => LfoShape::SawDown,
            "sq" | "square" => LfoShape::Square,
            "drift" | "smooth_drift" | "perlin" => LfoShape::SmoothDrift,
            "arch" | "swell" | "swell_arch" => LfoShape::SwellArch,
            _ => LfoShape::Sine,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            LfoShape::Sine => "sine",
            LfoShape::Triangle => "triangle",
            LfoShape::SawUp => "saw_up",
            LfoShape::SawDown => "saw_down",
            LfoShape::Square => "square",
            LfoShape::SmoothDrift => "smooth_drift",
            LfoShape::SwellArch => "swell_arch",
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct LfoSpec {
    pub shape: LfoShape,
    pub rate_hz: f64,
    pub depth: f64,
    pub phase_offset_rad: f64,
    /// Unipolar (0..depth) vs Bipolar (-depth..+depth)
    pub unipolar: bool,
}

impl Default for LfoSpec {
    fn default() -> Self {
        Self {
            shape: LfoShape::Sine,
            rate_hz: 0.28,
            depth: 1.2,
            phase_offset_rad: 0.0,
            unipolar: false,
        }
    }
}

impl LfoSpec {
    #[inline]
    pub fn eval(&self, time_secs: f64) -> f64 {
        let phase = TAU * self.rate_hz * time_secs + self.phase_offset_rad;
        let raw = match self.shape {
            LfoShape::Sine => phase.sin(),
            LfoShape::Triangle => {
                let u = (phase / TAU).rem_euclid(1.0);
                1.0 - 4.0 * (u - 0.5).abs()
            }
            LfoShape::SawUp => {
                let u = (phase / TAU).rem_euclid(1.0);
                2.0 * u - 1.0
            }
            LfoShape::SawDown => {
                let u = (phase / TAU).rem_euclid(1.0);
                1.0 - 2.0 * u
            }
            LfoShape::Square => {
                if phase.sin() >= 0.0 {
                    1.0
                } else {
                    -1.0
                }
            }
            LfoShape::SmoothDrift => {
                // Quasi-periodic irrational harmonic sum (never repeats mechanically!)
                0.52 * phase.sin()
                    + 0.31 * (phase * 1.6180339887).cos()
                    + 0.17 * (phase * 2.4142135623).sin()
            }
            LfoShape::SwellArch => {
                let u = (phase / TAU).rem_euclid(1.0);
                (std::f64::consts::PI * u).sin() * 2.0 - 1.0
            }
        };

        if self.unipolar {
            (0.5 + 0.5 * raw) * self.depth
        } else {
            raw * self.depth
        }
    }
}
