//! Lightweight pitch / chord sketch from a WAV.
//!
//! This is **not** source-separation, not a DAW transcription, and it cannot
//! recover an FM operator graph or a math formula from a recording. It estimates
//! a melody-ish pitch contour (YIN-style) plus a 12-class chroma sketch so a
//! user or agent can seed a Harmonia Block. Treat results as a starting point.

use crate::wav::StereoBuffer;

const NOTE_NAMES: [&str; 12] = [
    "C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B",
];

#[derive(Debug, Clone)]
pub struct PitchFrame {
    pub t_sec: f64,
    pub hz: f64,
    pub midi: f64,
    pub note: String,
    pub confidence: f64,
}

#[derive(Debug, Clone)]
pub struct ChordGuess {
    pub t_sec: f64,
    pub duration_sec: f64,
    pub symbol: String,
    pub root_pc: u8,
    pub quality: &'static str,
    pub chroma: [f64; 12],
}

#[derive(Debug, Clone)]
pub struct AnalyzeReport {
    pub duration_secs: f64,
    pub sample_rate: u32,
    pub bpm_est: f64,
    pub key_guess: String,
    pub melody_notes: Vec<PitchFrame>,
    pub chords: Vec<ChordGuess>,
    pub progression: String,
    pub hz_list: Vec<f64>,
    pub confidence: f64,
    pub caveats: Vec<String>,
}

impl AnalyzeReport {
    pub fn to_json(&self) -> String {
        let mut s = String::from("{\n");
        s.push_str(&format!("  \"duration_secs\": {:.3},\n", self.duration_secs));
        s.push_str(&format!("  \"sample_rate\": {},\n", self.sample_rate));
        s.push_str(&format!("  \"bpm_est\": {:.1},\n", self.bpm_est));
        s.push_str(&format!("  \"key_guess\": \"{}\",\n", crate::json::escape_json(&self.key_guess)));
        s.push_str(&format!("  \"progression\": \"{}\",\n", crate::json::escape_json(&self.progression)));
        s.push_str(&format!("  \"confidence\": {:.3},\n", self.confidence));
        s.push_str("  \"hz_list\": [");
        for (i, h) in self.hz_list.iter().enumerate() {
            if i > 0 {
                s.push_str(", ");
            }
            s.push_str(&format!("{h:.2}"));
        }
        s.push_str("],\n  \"melody_notes\": [\n");
        for (i, n) in self.melody_notes.iter().enumerate() {
            if i > 0 {
                s.push_str(",\n");
            }
            s.push_str(&format!(
                "    {{\"t\": {:.3}, \"hz\": {:.2}, \"midi\": {:.2}, \"note\": \"{}\", \"confidence\": {:.2}}}",
                n.t_sec, n.hz, n.midi, crate::json::escape_json(&n.note), n.confidence
            ));
        }
        s.push_str("\n  ],\n  \"chords\": [\n");
        for (i, c) in self.chords.iter().enumerate() {
            if i > 0 {
                s.push_str(",\n");
            }
            s.push_str(&format!(
                "    {{\"t\": {:.3}, \"dur\": {:.3}, \"symbol\": \"{}\", \"root_pc\": {}, \"quality\": \"{}\"}}",
                c.t_sec, c.duration_sec, crate::json::escape_json(&c.symbol), c.root_pc, c.quality
            ));
        }
        s.push_str("\n  ],\n  \"caveats\": [\n");
        for (i, c) in self.caveats.iter().enumerate() {
            if i > 0 {
                s.push_str(",\n");
            }
            s.push_str(&format!("    \"{}\"", crate::json::escape_json(c)));
        }
        s.push_str("\n  ]\n}\n");
        s
    }
}

pub fn analyze_buffer(buf: &StereoBuffer) -> AnalyzeReport {
    let n = buf.len();
    let sr = buf.sample_rate.max(1);
    let duration = n as f64 / sr as f64;
    let mut caveats = vec![
        "Sketch only: not polyphonic source separation.".into(),
        "Cannot recover FM operator graphs, math formulas, or DX7 algorithms from audio.".into(),
        "Dense mixes, vocals, and drums confuse chroma; treat chords as a seed.".into(),
    ];

    if n < sr as usize / 4 {
        caveats.push("Clip shorter than 250 ms — results are unreliable.".into());
        return AnalyzeReport {
            duration_secs: duration,
            sample_rate: sr,
            bpm_est: 0.0,
            key_guess: "?".into(),
            melody_notes: vec![],
            chords: vec![],
            progression: String::new(),
            hz_list: vec![],
            confidence: 0.0,
            caveats,
        };
    }

    let hop = (sr as usize / 20).max(256); // 50 ms
    let win = hop * 4;
    let mut melody = Vec::new();
    let mut chroma_acc = [0.0_f64; 12];
    let mut onset_env = Vec::new();
    let mut prev_energy = 0.0_f64;

    let mut i = 0usize;
    while i + win < n {
        let (hz, conf) = yin_pitch(buf, i, win);
        let t = i as f64 / sr as f64;
        if hz > 40.0 && hz < 1800.0 && conf > 0.25 {
            let midi = 69.0 + 12.0 * (hz / 440.0).log2();
            let pc = midi.round().rem_euclid(12.0) as usize;
            melody.push(PitchFrame {
                t_sec: t,
                hz,
                midi,
                note: format!("{}{}", NOTE_NAMES[pc.min(11)], (midi.round() as i32 / 12) - 1),
                confidence: conf,
            });
        }
        let ch = frame_chroma(buf, i, win);
        for k in 0..12 {
            chroma_acc[k] += ch[k];
        }
        let energy: f64 = (0..win)
            .map(|k| {
                let s = 0.5 * (buf.left[i + k] + buf.right[i + k]);
                s * s
            })
            .sum();
        let flux = (energy - prev_energy).max(0.0);
        onset_env.push(flux);
        prev_energy = energy;
        i += hop;
    }

    let bpm_est = estimate_bpm(&onset_env, hop, sr);
    let key_pc = argmax12(&chroma_acc);
    let key_guess = format!("{} major/minor (chroma peak)", NOTE_NAMES[key_pc]);

    let chords = segment_chords(buf, hop, win, bpm_est);
    let progression = if chords.is_empty() {
        String::new()
    } else {
        chords
            .iter()
            .map(|c| c.symbol.as_str())
            .collect::<Vec<_>>()
            .join(" -> ")
    };

    let mut hz_list: Vec<f64> = melody.iter().map(|m| (m.hz * 2.0).round() / 2.0).collect();
    hz_list.dedup_by(|a, b| (*a - *b).abs() < 1.0);
    if hz_list.len() > 16 {
        hz_list.truncate(16);
    }

    let conf = if melody.is_empty() {
        0.15
    } else {
        let mean = melody.iter().map(|m| m.confidence).sum::<f64>() / melody.len() as f64;
        (mean * 0.7 + if chords.len() >= 2 { 0.2 } else { 0.05 }).clamp(0.1, 0.85)
    };

    AnalyzeReport {
        duration_secs: duration,
        sample_rate: sr,
        bpm_est,
        key_guess,
        melody_notes: downsample_melody(melody),
        chords,
        progression,
        hz_list,
        confidence: conf,
        caveats,
    }
}

fn downsample_melody(notes: Vec<PitchFrame>) -> Vec<PitchFrame> {
    if notes.len() <= 48 {
        return notes;
    }
    let step = notes.len() / 48;
    notes.into_iter().step_by(step.max(1)).take(48).collect()
}

/// Cumulative-mean-normalized difference (YIN, de Cheveigné & Kawahara 2002).
fn yin_pitch(buf: &StereoBuffer, start: usize, win: usize) -> (f64, f64) {
    let sr = buf.sample_rate as f64;
    let tau_min = (sr / 1800.0) as usize;
    let tau_max = ((sr / 40.0) as usize).min(win / 2);
    if tau_max <= tau_min + 4 {
        return (0.0, 0.0);
    }
    let mut d = vec![0.0_f64; tau_max + 1];
    for tau in 1..=tau_max {
        let mut sum = 0.0;
        let count = win - tau;
        for j in 0..count {
            let a = 0.5 * (buf.left[start + j] + buf.right[start + j]);
            let b = 0.5 * (buf.left[start + j + tau] + buf.right[start + j + tau]);
            let e = a - b;
            sum += e * e;
        }
        d[tau] = sum;
    }
    let mut cmnd = vec![1.0_f64; tau_max + 1];
    let mut run = 0.0;
    for tau in 1..=tau_max {
        run += d[tau];
        cmnd[tau] = d[tau] * tau as f64 / run.max(1e-12);
    }
    let thresh = 0.15;
    let mut tau_est = 0usize;
    for tau in tau_min..tau_max {
        if cmnd[tau] < thresh {
            tau_est = tau;
            while tau_est + 1 < tau_max && cmnd[tau_est + 1] < cmnd[tau_est] {
                tau_est += 1;
            }
            break;
        }
    }
    if tau_est == 0 {
        // take global min
        let mut best = tau_min;
        for tau in tau_min..tau_max {
            if cmnd[tau] < cmnd[best] {
                best = tau;
            }
        }
        tau_est = best;
    }
    let conf = (1.0 - cmnd[tau_est]).clamp(0.0, 1.0);
    if tau_est == 0 {
        return (0.0, 0.0);
    }
    (sr / tau_est as f64, conf)
}

fn frame_chroma(buf: &StereoBuffer, start: usize, win: usize) -> [f64; 12] {
    let mut acc = [0.0_f64; 12];
    let sr = buf.sample_rate as f64;
    // Goertzel-like energy at 36–84 MIDI (C2–C6)
    for midi in 36u32..=84 {
        let hz = 440.0 * (2.0_f64).powf((midi as f64 - 69.0) / 12.0);
        let w = 2.0 * std::f64::consts::PI * hz / sr;
        let coeff = 2.0 * w.cos();
        let mut s1 = 0.0;
        let mut s2 = 0.0;
        let n = win.min(buf.len() - start);
        for j in 0..n {
            let x = 0.5 * (buf.left[start + j] + buf.right[start + j]);
            let s0 = x + coeff * s1 - s2;
            s2 = s1;
            s1 = s0;
        }
        let power = s1 * s1 + s2 * s2 - coeff * s1 * s2;
        let pc = (midi as usize) % 12;
        acc[pc] += power.max(0.0);
    }
    let max = acc.iter().copied().fold(0.0_f64, f64::max).max(1e-12);
    for v in &mut acc {
        *v /= max;
    }
    acc
}

fn segment_chords(buf: &StereoBuffer, hop: usize, win: usize, bpm: f64) -> Vec<ChordGuess> {
    let sr = buf.sample_rate.max(1) as usize;
    let beat = if bpm > 40.0 && bpm < 220.0 {
        ((60.0 / bpm) * sr as f64) as usize
    } else {
        sr // 1 second
    }
    .max(hop * 4);
    let mut out: Vec<ChordGuess> = Vec::new();
    let mut pos = 0usize;
    while pos + win < buf.len() {
        let end = (pos + beat).min(buf.len());
        let ch = frame_chroma(buf, pos, (end - pos).min(win * 2).max(win));
        let (symbol, root, quality) = match_chord(&ch);
        let t = pos as f64 / sr as f64;
        let dur = (end - pos) as f64 / sr as f64;
        if let Some(last) = out.last_mut() {
            if last.symbol == symbol {
                last.duration_sec += dur;
                pos = end;
                continue;
            }
        }
        out.push(ChordGuess {
            t_sec: t,
            duration_sec: dur,
            symbol,
            root_pc: root,
            quality,
            chroma: ch,
        });
        pos = end;
        if out.len() >= 16 {
            break;
        }
    }
    out
}

fn match_chord(ch: &[f64; 12]) -> (String, u8, &'static str) {
    let templates: [(&str, [f64; 12]); 4] = [
        ("", [1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0]), // maj
        ("m", [1.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0]),
        ("7", [1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 1.0, 0.0, 0.0, 1.0, 0.0]),
        ("m7", [1.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 1.0, 0.0]),
    ];
    let mut best = f64::NEG_INFINITY;
    let mut best_root = 0u8;
    let mut best_q = "";
    for root in 0..12 {
        for (q, tmpl) in &templates {
            let mut score = 0.0;
            for i in 0..12 {
                score += ch[i] * tmpl[(i + 12 - root) % 12];
            }
            if score > best {
                best = score;
                best_root = root as u8;
                best_q = q;
            }
        }
    }
    (
        format!("{}{}", NOTE_NAMES[best_root as usize], best_q),
        best_root,
        if best_q.is_empty() { "maj" } else { best_q },
    )
}

fn argmax12(v: &[f64; 12]) -> usize {
    let mut i = 0usize;
    for k in 1..12 {
        if v[k] > v[i] {
            i = k;
        }
    }
    i
}

fn estimate_bpm(onset: &[f64], hop: usize, sr: u32) -> f64 {
    if onset.len() < 8 {
        return 0.0;
    }
    let hop_sec = hop as f64 / sr as f64;
    let min_lag = (0.27 / hop_sec).round() as usize; // ~220 bpm
    let max_lag = (1.5 / hop_sec).round() as usize; // ~40 bpm
    let max_lag = max_lag.min(onset.len() / 2).max(min_lag + 1);
    let mut best_lag = min_lag;
    let mut best = 0.0;
    for lag in min_lag..max_lag {
        let mut acc = 0.0;
        for i in 0..onset.len() - lag {
            acc += onset[i] * onset[i + lag];
        }
        if acc > best {
            best = acc;
            best_lag = lag;
        }
    }
    let bpm = 60.0 / (best_lag as f64 * hop_sec);
    if bpm < 70.0 {
        bpm * 2.0
    } else if bpm > 180.0 {
        bpm / 2.0
    } else {
        bpm
    }
}
