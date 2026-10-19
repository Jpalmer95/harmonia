//! Karplus–Strong plucked-string physical model (public-domain algorithm).
//! Used as an extra operator waveform / math helper for lute, guitar, harp transients.

use std::f64::consts::TAU;

#[derive(Debug, Clone)]
pub struct KarplusStrong {
    delay: Vec<f64>,
    pos: usize,
    damp: f64,
    stretch: f64,
    last: f64,
}

impl KarplusStrong {
    pub fn new(sample_rate: u32, freq_hz: f64, damp: f64, stretch: f64, seed: u64) -> Self {
        let len = ((sample_rate as f64 / freq_hz.max(20.0)).round() as usize).clamp(8, 4096);
        let mut delay = vec![0.0; len];
        let mut state = seed | 1;
        for s in &mut delay {
            state = state.wrapping_mul(6364136223846793005).wrapping_add(1);
            let r = ((state >> 33) as f64 / (1u64 << 31) as f64) * 2.0 - 1.0;
            *s = r;
        }
        Self {
            delay,
            pos: 0,
            damp: damp.clamp(0.0, 0.5),
            stretch: stretch.clamp(0.0, 1.0),
            last: 0.0,
        }
    }

    #[inline]
    pub fn tick(&mut self) -> f64 {
        let n = self.delay.len();
        let current = self.delay[self.pos];
        let next = self.delay[(self.pos + 1) % n];
        // Two-point lowpass + stretch (allpass-ish blend)
        let avg = 0.5 * (current + next);
        let filtered = (1.0 - self.damp) * ((1.0 - self.stretch) * avg + self.stretch * current);
        self.delay[self.pos] = filtered;
        self.pos = (self.pos + 1) % n;
        self.last = filtered;
        filtered
    }
}

/// One-shot render of a plucked note (mono) for mixing into a voice.
pub fn render_pluck(
    sample_rate: u32,
    freq_hz: f64,
    duration_secs: f64,
    damp: f64,
    velocity: f64,
) -> Vec<f64> {
    let n = ((duration_secs * sample_rate as f64).ceil() as usize).max(1);
    let mut ks = KarplusStrong::new(sample_rate, freq_hz, damp, 0.15, (freq_hz * 1000.0) as u64);
    let mut out = vec![0.0; n];
    let gain = velocity.clamp(0.05, 1.0) * 0.9;
    for i in 0..n {
        let env = (-(i as f64) / sample_rate as f64 * (0.4 + damp * 8.0)).exp();
        out[i] = ks.tick() * gain * env;
    }
    out
}

/// Cheap harmonic excitation used by the math VM `ks(f, t, damp)` helper.
#[inline]
pub fn ks_waveform(freq_hz: f64, t: f64, damp: f64) -> f64 {
    let f = freq_hz.max(20.0);
    let mut sum = 0.0;
    for k in 1..=12 {
        let kf = k as f64;
        let decay = (-t * damp.max(0.05) * (0.5 + 0.55 * kf)).exp();
        // slight inharmonic stiffness
        let fk = f * kf * (1.0 + 0.0002 * kf * kf).sqrt();
        sum += decay / kf.powf(0.9) * (TAU * fk * t).sin();
    }
    sum * 0.55
}
