//! Studio Hi-Def DSP Effects Rack in 64-bit Float Precision:
//! - Zero-Delay Feedback State-Variable Filter (ZDF SVF) with LFO cutoff modulation
//! - Quadrature Stereo Dimension Chorus
//! - Damped Stereo Ping-Pong Delay
//! - 8-Comb / 4-Allpass Algorithmic Cathedral & Shimmer Reverb
//! - Master Bus (DC Blocker + Soft-Knee Analog Saturation + Headroom Limiter)

use crate::fm_synth::FmPatch;
use crate::wav::StereoBuffer;
use std::f64::consts::{PI, TAU};

/// Apply the full patch effects chain to a rendered `StereoBuffer`.
pub fn process_master_chain(buf: &mut StereoBuffer, patch: &FmPatch) {
    let n = buf.len();
    if n == 0 {
        return;
    }

    // 1. High-pass DC Blocker (~18 Hz)
    apply_dc_blocker(buf);

    // 2. Zero-Delay Feedback State-Variable Filter (ZDF SVF) with LFO cutoff sweep
    apply_zdf_svf_filter(
        buf,
        patch.filter_cutoff_hz,
        patch.filter_resonance,
        patch.filter_lfo_depth_hz,
        patch.lfo1.rate_hz,
    );

    // 3. Quadrature Stereo Dimension Chorus
    if patch.chorus_mix > 0.01 {
        apply_stereo_chorus(buf, patch.chorus_mix);
    }

    // 4. Stereo Ping-Pong Delay
    if patch.delay_mix > 0.01 {
        apply_ping_pong_delay(
            buf,
            patch.delay_time_secs,
            patch.delay_feedback,
            patch.delay_mix,
        );
    }

    // 5. Cathedral & Shimmer Algorithmic Reverb
    if patch.reverb_mix > 0.01 {
        apply_cathedral_reverb(
            buf,
            patch.reverb_room_size,
            patch.reverb_shimmer,
            patch.reverb_mix,
        );
    }

    // 6. Master Bus Soft-Knee Saturation & True-Peak Normalization (-0.7 dBFS)
    let drive = patch.master_drive.clamp(0.5, 3.0);
    let drive_norm = 1.0 / drive.tanh();
    for i in 0..n {
        buf.left[i] = (buf.left[i] * drive).tanh() * drive_norm;
        buf.right[i] = (buf.right[i] * drive).tanh() * drive_norm;
    }

    buf.apply_fades(0.015, 0.35);
    buf.normalize(0.92);
}

pub fn apply_dc_blocker(buf: &mut StereoBuffer) {
    let sr = buf.sample_rate as f64;
    let r = 1.0 - (TAU * 18.0 / sr).clamp(0.0001, 0.05);
    let mut prev_in_l = 0.0_f64;
    let mut prev_out_l = 0.0_f64;
    let mut prev_in_r = 0.0_f64;
    let mut prev_out_r = 0.0_f64;

    for i in 0..buf.len() {
        let xl = buf.left[i];
        let yl = xl - prev_in_l + r * prev_out_l;
        prev_in_l = xl;
        prev_out_l = yl;
        buf.left[i] = yl;

        let xr = buf.right[i];
        let yr = xr - prev_in_r + r * prev_out_r;
        prev_in_r = xr;
        prev_out_r = yr;
        buf.right[i] = yr;
    }
}

/// Zero-Delay Feedback (TPT/ZDF) 2-Pole State Variable Low-Pass Filter with warm saturation.
pub fn apply_zdf_svf_filter(
    buf: &mut StereoBuffer,
    base_cutoff_hz: f64,
    resonance: f64,
    lfo_depth_hz: f64,
    lfo_rate_hz: f64,
) {
    let sr = buf.sample_rate as f64;
    let nyquist_safe = (sr * 0.44).max(1000.0);
    let k = 2.0 - 1.85 * resonance.clamp(0.0, 0.92);

    let mut ic1eq_l = 0.0_f64;
    let mut ic2eq_l = 0.0_f64;
    let mut ic1eq_r = 0.0_f64;
    let mut ic2eq_r = 0.0_f64;

    for i in 0..buf.len() {
        let t = i as f64 / sr;
        let lfo_l = (TAU * lfo_rate_hz * t).sin();
        let lfo_r = (TAU * lfo_rate_hz * t + 0.35).sin();

        let fc_l = (base_cutoff_hz + lfo_depth_hz * lfo_l).clamp(80.0, nyquist_safe);
        let fc_r = (base_cutoff_hz + lfo_depth_hz * lfo_r).clamp(80.0, nyquist_safe);

        let g_l = (PI * fc_l / sr).tan();
        let a1_l = 1.0 / (1.0 + g_l * (g_l + k));
        let a2_l = g_l * a1_l;
        let a3_l = g_l * a2_l;

        let v0_l = buf.left[i];
        let v3_l = v0_l - ic2eq_l;
        let v1_l = a1_l * ic1eq_l + a2_l * v3_l;
        let v2_l = ic2eq_l + a2_l * ic1eq_l + a3_l * v3_l;
        ic1eq_l = (2.0 * v1_l - ic1eq_l).clamp(-4.0, 4.0);
        ic2eq_l = (2.0 * v2_l - ic2eq_l).clamp(-4.0, 4.0);
        buf.left[i] = v2_l;

        let g_r = (PI * fc_r / sr).tan();
        let a1_r = 1.0 / (1.0 + g_r * (g_r + k));
        let a2_r = g_r * a1_r;
        let a3_r = g_r * a2_r;

        let v0_r = buf.right[i];
        let v3_r = v0_r - ic2eq_r;
        let v1_r = a1_r * ic1eq_r + a2_r * v3_r;
        let v2_r = ic2eq_r + a2_r * ic1eq_r + a3_r * v3_r;
        ic1eq_r = (2.0 * v1_r - ic1eq_r).clamp(-4.0, 4.0);
        ic2eq_r = (2.0 * v2_r - ic2eq_r).clamp(-4.0, 4.0);
        buf.right[i] = v2_r;
    }
}

/// Quadrature 4-tap Stereo Dimension Chorus.
pub fn apply_stereo_chorus(buf: &mut StereoBuffer, mix: f64) {
    let sr = buf.sample_rate as f64;
    let max_delay = (0.045 * sr) as usize + 8;
    let mut dl_l = vec![0.0_f64; max_delay];
    let mut dl_r = vec![0.0_f64; max_delay];
    let mut wp = 0usize;

    let wet = mix.clamp(0.0, 0.8);
    let dry = (1.0 - 0.5 * wet).sqrt();

    for i in 0..buf.len() {
        let t = i as f64 / sr;
        let in_l = buf.left[i];
        let in_r = buf.right[i];

        dl_l[wp] = in_l;
        dl_r[wp] = in_r;

        // Voice 1 & 2 delay times in samples (around 11ms and 17ms)
        let d1_l = (0.011 + 0.0028 * (TAU * 0.42 * t).sin()) * sr;
        let d2_l = (0.017 + 0.0032 * (TAU * 0.67 * t + 1.57).sin()) * sr;
        let d1_r = (0.012 + 0.0028 * (TAU * 0.42 * t + 3.14).sin()) * sr;
        let d2_r = (0.018 + 0.0032 * (TAU * 0.67 * t + 4.71).sin()) * sr;

        let c_l = 0.5 * (read_frac(&dl_l, wp, d1_l) + read_frac(&dl_r, wp, d2_l));
        let c_r = 0.5 * (read_frac(&dl_r, wp, d1_r) + read_frac(&dl_l, wp, d2_r));

        buf.left[i] = dry * in_l + wet * c_l;
        buf.right[i] = dry * in_r + wet * c_r;

        wp = (wp + 1) % max_delay;
    }
}

#[inline]
fn read_frac(ring: &[f64], wp: usize, delay_samples: f64) -> f64 {
    let len = ring.len();
    let d = delay_samples.clamp(1.0, (len - 3) as f64);
    let int_d = d.floor() as usize;
    let frac = d - int_d as f64;
    let idx0 = (wp + len - int_d) % len;
    let idx1 = (wp + len - int_d - 1) % len;
    ring[idx0] * (1.0 - frac) + ring[idx1] * frac
}

/// Damped Stereo Ping-Pong Delay.
pub fn apply_ping_pong_delay(
    buf: &mut StereoBuffer,
    delay_secs: f64,
    feedback: f64,
    mix: f64,
) {
    let sr = buf.sample_rate as f64;
    let delay_samples = ((delay_secs.clamp(0.05, 1.5) * sr) as usize).max(16);
    let mut line_l = vec![0.0_f64; delay_samples];
    let mut line_r = vec![0.0_f64; delay_samples];
    let mut pos = 0usize;
    let mut lp_l = 0.0_f64;
    let mut lp_r = 0.0_f64;

    let fb = feedback.clamp(0.0, 0.82);
    let wet = mix.clamp(0.0, 0.75);

    for i in 0..buf.len() {
        let in_l = buf.left[i];
        let in_r = buf.right[i];

        let del_l = line_l[pos];
        let del_r = line_r[pos];

        // Warm one-pole lowpass damping in feedback loop
        lp_l = 0.65 * del_l + 0.35 * lp_l;
        lp_r = 0.65 * del_r + 0.35 * lp_r;

        // Ping-pong cross-feed
        line_l[pos] = (in_l * 0.7 + lp_r * fb).clamp(-2.0, 2.0);
        line_r[pos] = (in_r * 0.7 + lp_l * fb).clamp(-2.0, 2.0);

        buf.left[i] = in_l + wet * del_l;
        buf.right[i] = in_r + wet * del_r;

        pos = (pos + 1) % delay_samples;
    }
}

struct CombFilter {
    buf: Vec<f64>,
    pos: usize,
    filter_store: f64,
    feedback: f64,
    damp1: f64,
    damp2: f64,
}

impl CombFilter {
    fn new(size: usize, feedback: f64, damp: f64) -> Self {
        Self {
            buf: vec![0.0; size.max(16)],
            pos: 0,
            filter_store: 0.0,
            feedback,
            damp1: damp,
            damp2: 1.0 - damp,
        }
    }

    #[inline]
    fn process(&mut self, input: f64) -> f64 {
        let out = self.buf[self.pos];
        self.filter_store = out * self.damp2 + self.filter_store * self.damp1;
        self.buf[self.pos] = (input + self.filter_store * self.feedback).clamp(-4.0, 4.0);
        self.pos = (self.pos + 1) % self.buf.len();
        out
    }
}

struct AllPassFilter {
    buf: Vec<f64>,
    pos: usize,
    feedback: f64,
}

impl AllPassFilter {
    fn new(size: usize, feedback: f64) -> Self {
        Self {
            buf: vec![0.0; size.max(8)],
            pos: 0,
            feedback,
        }
    }

    #[inline]
    fn process(&mut self, input: f64) -> f64 {
        let buf_out = self.buf[self.pos];
        let out = -input + buf_out;
        self.buf[self.pos] = (input + buf_out * self.feedback).clamp(-4.0, 4.0);
        self.pos = (self.pos + 1) % self.buf.len();
        out
    }
}

/// Lush 8-Comb / 4-Allpass Stereo Cathedral Reverb with Shimmer harmonic excitation.
pub fn apply_cathedral_reverb(
    buf: &mut StereoBuffer,
    room_size: f64,
    shimmer: f64,
    mix: f64,
) {
    let sr_scale = buf.sample_rate as f64 / 44100.0;
    let base_combs: [usize; 8] = [1557, 1617, 1491, 1422, 1277, 1356, 1188, 1116];
    let base_allpass: [usize; 4] = [225, 556, 441, 341];
    let stereo_spread = 29usize;

    let fb = (0.72 + 0.23 * room_size.clamp(0.1, 0.98)).min(0.955);
    let damp = 0.28;

    let mut combs_l: Vec<CombFilter> = base_combs
        .iter()
        .map(|&n| CombFilter::new(((n as f64) * sr_scale) as usize, fb, damp))
        .collect();
    let mut combs_r: Vec<CombFilter> = base_combs
        .iter()
        .map(|&n| CombFilter::new((((n + stereo_spread) as f64) * sr_scale) as usize, fb, damp))
        .collect();

    let mut ap_l: Vec<AllPassFilter> = base_allpass
        .iter()
        .map(|&n| AllPassFilter::new(((n as f64) * sr_scale) as usize, 0.5))
        .collect();
    let mut ap_r: Vec<AllPassFilter> = base_allpass
        .iter()
        .map(|&n| AllPassFilter::new((((n + stereo_spread) as f64) * sr_scale) as usize, 0.5))
        .collect();

    let wet = mix.clamp(0.0, 0.85);
    let dry = 1.0 - 0.35 * wet;
    let shim = shimmer.clamp(0.0, 0.6);

    // Dual-tap pitch-doubling shimmer buffer (octave-up grain crossfade)
    let grain_len = ((0.065 * buf.sample_rate as f64) as usize).max(256);
    let mut shim_buf = vec![0.0_f64; grain_len];
    let mut shim_wp = 0usize;
    let mut shim_rp = 0.0_f64;

    for i in 0..buf.len() {
        let in_l = buf.left[i];
        let in_r = buf.right[i];
        let mono = 0.5 * (in_l + in_r);

        // Shimmer octave excitation (full-wave harmonic + pitch-doubled grain)
        shim_buf[shim_wp] = mono;
        let idx_a = (shim_rp as usize) % grain_len;
        let idx_b = (idx_a + grain_len / 2) % grain_len;
        let phase_u = (shim_rp % (grain_len as f64)) / (grain_len as f64);
        let win_a = (PI * phase_u).sin().powi(2);
        let win_b = 1.0 - win_a;
        let oct_up = shim_buf[idx_a] * win_a + shim_buf[idx_b] * win_b;
        shim_wp = (shim_wp + 1) % grain_len;
        shim_rp = (shim_rp + 2.0) % (grain_len as f64);

        let rev_in_l = (in_l + shim * 0.45 * oct_up) * 0.035;
        let rev_in_r = (in_r + shim * 0.45 * oct_up) * 0.035;

        let mut out_l = 0.0_f64;
        let mut out_r = 0.0_f64;

        for c in &mut combs_l {
            out_l += c.process(rev_in_l);
        }
        for c in &mut combs_r {
            out_r += c.process(rev_in_r);
        }

        for a in &mut ap_l {
            out_l = a.process(out_l);
        }
        for a in &mut ap_r {
            out_r = a.process(out_r);
        }

        buf.left[i] = dry * in_l + wet * (out_l * 0.88 + out_r * 0.12);
        buf.right[i] = dry * in_r + wet * (out_r * 0.88 + out_l * 0.12);
    }
}
