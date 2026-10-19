//! High-Definition WAV encoder, reader, and signal analyzer in pure Rust.
//! Supports 16-bit PCM, 24-bit Studio PCM, and 32-bit IEEE Float formats at any sample rate (44.1kHz..192kHz).

use std::fs::{self, File};
use std::io::{BufWriter, Read, Write};
use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BitDepth {
    Pcm16,
    Pcm24,
    Float32,
}

impl BitDepth {
    pub fn from_str(s: &str) -> Self {
        match s.trim().to_ascii_lowercase().as_str() {
            "16" | "pcm16" | "16bit" => BitDepth::Pcm16,
            "32" | "32f" | "float" | "float32" | "f32" => BitDepth::Float32,
            _ => BitDepth::Pcm24,
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            BitDepth::Pcm16 => "16-bit PCM",
            BitDepth::Pcm24 => "24-bit Hi-Def PCM",
            BitDepth::Float32 => "32-bit IEEE Float",
        }
    }

    pub fn bytes_per_sample(&self) -> u16 {
        match self {
            BitDepth::Pcm16 => 2,
            BitDepth::Pcm24 => 3,
            BitDepth::Float32 => 4,
        }
    }

    pub fn format_tag(&self) -> u16 {
        match self {
            BitDepth::Pcm16 | BitDepth::Pcm24 => 1, // WAVE_FORMAT_PCM
            BitDepth::Float32 => 3,                 // WAVE_FORMAT_IEEE_FLOAT
        }
    }
}

#[derive(Debug, Clone)]
pub struct StereoBuffer {
    pub sample_rate: u32,
    pub left: Vec<f64>,
    pub right: Vec<f64>,
    /// TPDF dither scale for 16-bit encode (0 = off). 1.0 = ±1 LSB.
    pub dither_amp: f64,
}

#[derive(Debug, Clone)]
pub struct AudioReport {
    pub sample_rate: u32,
    pub num_frames: usize,
    pub duration_secs: f64,
    pub peak_linear: f64,
    pub peak_dbfs: f64,
    pub rms_linear: f64,
    pub rms_dbfs: f64,
    pub crest_factor_db: f64,
    pub zero_crossing_hz_est: f64,
    pub stereo_width_corr: f64,
    pub dc_offset: f64,
    pub ascii_envelope: String,
    pub mono_compatible: bool,
    pub stereo_width_label: String,
}

impl StereoBuffer {
    pub fn new(sample_rate: u32, num_frames: usize) -> Self {
        Self {
            sample_rate,
            left: vec![0.0; num_frames],
            right: vec![0.0; num_frames],
            dither_amp: 0.0,
        }
    }

    pub fn len(&self) -> usize {
        self.left.len().min(self.right.len())
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn duration_secs(&self) -> f64 {
        self.len() as f64 / self.sample_rate.max(1) as f64
    }

    /// Normalize peak to target linear amplitude (e.g., 0.92 for -0.7 dBFS headroom).
    pub fn normalize(&mut self, target_peak: f64) {
        let mut max_abs = 0.0_f64;
        for (&l, &r) in self.left.iter().zip(self.right.iter()) {
            let m = l.abs().max(r.abs());
            if m.is_finite() && m > max_abs {
                max_abs = m;
            }
        }
        if max_abs > 1e-9 {
            let gain = target_peak / max_abs;
            for l in &mut self.left {
                *l = (*l * gain).clamp(-1.0, 1.0);
            }
            for r in &mut self.right {
                *r = (*r * gain).clamp(-1.0, 1.0);
            }
        }
    }

    /// Apply smooth fade-in and fade-out to prevent boundary clicks.
    pub fn apply_fades(&mut self, fade_in_secs: f64, fade_out_secs: f64) {
        let n = self.len();
        if n == 0 {
            return;
        }
        let fade_in_samples = ((fade_in_secs * self.sample_rate as f64) as usize).min(n / 2);
        let fade_out_samples = ((fade_out_secs * self.sample_rate as f64) as usize).min(n / 2);

        for i in 0..fade_in_samples {
            let u = i as f64 / fade_in_samples.max(1) as f64;
            let g = 0.5 * (1.0 - (std::f64::consts::PI * u).cos());
            self.left[i] *= g;
            self.right[i] *= g;
        }
        for i in 0..fade_out_samples {
            let u = i as f64 / fade_out_samples.max(1) as f64;
            let g = 0.5 * (1.0 - (std::f64::consts::PI * u).cos());
            let idx = n - 1 - i;
            self.left[idx] *= g;
            self.right[idx] *= g;
        }
    }

    /// Encode to WAV bytes in memory.
    pub fn to_wav_bytes(&self, bit_depth: BitDepth) -> Vec<u8> {
        let num_frames = self.len();
        let num_channels: u16 = 2;
        let bytes_per_sample = bit_depth.bytes_per_sample();
        let block_align: u16 = num_channels * bytes_per_sample;
        let byte_rate: u32 = self.sample_rate * block_align as u32;
        let data_size: u32 = (num_frames as u32).saturating_mul(block_align as u32);
        let riff_size: u32 = 36 + data_size;

        let mut buf = Vec::with_capacity((riff_size + 8) as usize);
        buf.extend_from_slice(b"RIFF");
        buf.extend_from_slice(&riff_size.to_le_bytes());
        buf.extend_from_slice(b"WAVE");

        // fmt chunk
        buf.extend_from_slice(b"fmt ");
        buf.extend_from_slice(&16u32.to_le_bytes());
        buf.extend_from_slice(&bit_depth.format_tag().to_le_bytes());
        buf.extend_from_slice(&num_channels.to_le_bytes());
        buf.extend_from_slice(&self.sample_rate.to_le_bytes());
        buf.extend_from_slice(&byte_rate.to_le_bytes());
        buf.extend_from_slice(&block_align.to_le_bytes());
        buf.extend_from_slice(&(bytes_per_sample * 8).to_le_bytes());

        // data chunk
        buf.extend_from_slice(b"data");
        buf.extend_from_slice(&data_size.to_le_bytes());

        let dither = if bit_depth == BitDepth::Pcm16 {
            self.dither_amp
        } else {
            0.0
        };
        let mut tpdf = 0xA5A5_u32.wrapping_mul(self.sample_rate | 1);
        for i in 0..num_frames {
            let mut l = if self.left[i].is_finite() {
                self.left[i].clamp(-1.0, 1.0)
            } else {
                0.0
            };
            let mut r = if self.right[i].is_finite() {
                self.right[i].clamp(-1.0, 1.0)
            } else {
                0.0
            };
            if dither > 0.0 {
                l = (l + tpdf_noise(&mut tpdf) * dither / 32768.0).clamp(-1.0, 1.0);
                r = (r + tpdf_noise(&mut tpdf) * dither / 32768.0).clamp(-1.0, 1.0);
            }
            write_sample(&mut buf, l, bit_depth);
            write_sample(&mut buf, r, bit_depth);
        }

        buf
    }

    /// Write WAV file to disk.
    pub fn save_wav<P: AsRef<Path>>(&self, path: P, bit_depth: BitDepth) -> std::io::Result<()> {
        let bytes = self.to_wav_bytes(bit_depth);
        let file = File::create(path)?;
        let mut writer = BufWriter::new(file);
        writer.write_all(&bytes)?;
        writer.flush()
    }

    /// Compute acoustic & signal quality telemetry report.
    pub fn analyze(&self) -> AudioReport {
        let n = self.len();
        if n == 0 {
            return AudioReport {
                sample_rate: self.sample_rate,
                num_frames: 0,
                duration_secs: 0.0,
                peak_linear: 0.0,
                peak_dbfs: -120.0,
                rms_linear: 0.0,
                rms_dbfs: -120.0,
                crest_factor_db: 0.0,
                zero_crossing_hz_est: 0.0,
                stereo_width_corr: 1.0,
                dc_offset: 0.0,
                ascii_envelope: String::new(),
                mono_compatible: true,
                stereo_width_label: "empty".into(),
            };
        }

        let mut peak = 0.0_f64;
        let mut sum_sq = 0.0_f64;
        let mut sum_dc = 0.0_f64;
        let mut zero_crossings = 0usize;
        let mut prev_mono = 0.0_f64;
        let mut sum_lr = 0.0_f64;
        let mut sum_l2 = 0.0_f64;
        let mut sum_r2 = 0.0_f64;

        for i in 0..n {
            let l = self.left[i];
            let r = self.right[i];
            let m = 0.5 * (l + r);
            let abs_max = l.abs().max(r.abs());
            if abs_max > peak {
                peak = abs_max;
            }
            sum_sq += 0.5 * (l * l + r * r);
            sum_dc += m;
            if i > 0 && ((m >= 0.0 && prev_mono < 0.0) || (m < 0.0 && prev_mono >= 0.0)) {
                zero_crossings += 1;
            }
            prev_mono = m;
            sum_lr += l * r;
            sum_l2 += l * l;
            sum_r2 += r * r;
        }

        let rms = (sum_sq / n as f64).sqrt();
        let peak_dbfs = if peak > 1e-12 {
            20.0 * peak.log10()
        } else {
            -120.0
        };
        let rms_dbfs = if rms > 1e-12 {
            20.0 * rms.log10()
        } else {
            -120.0
        };
        let duration = n as f64 / self.sample_rate.max(1) as f64;
        let zc_hz = (zero_crossings as f64 / 2.0) / duration.max(1e-6);
        let denom = (sum_l2 * sum_r2).sqrt();
        let corr = if denom > 1e-12 { sum_lr / denom } else { 1.0 };

        // Build 48-column ASCII amplitude bar
        let cols = 48usize;
        let chunk = (n / cols).max(1);
        let bars = [' ', '▁', '▂', '▃', '▄', '▅', '▆', '▇', '█'];
        let mut ascii = String::with_capacity(cols * 4);
        for c in 0..cols {
            let start = c * chunk;
            let end = ((c + 1) * chunk).min(n);
            if start >= end {
                ascii.push(' ');
                continue;
            }
            let mut c_peak = 0.0_f64;
            for i in start..end {
                let a = self.left[i].abs().max(self.right[i].abs());
                if a > c_peak {
                    c_peak = a;
                }
            }
            let norm = if peak > 1e-9 {
                (c_peak / peak).clamp(0.0, 1.0)
            } else {
                0.0
            };
            let idx = (norm * (bars.len() - 1) as f64).round() as usize;
            ascii.push(bars[idx.min(bars.len() - 1)]);
        }

        AudioReport {
            sample_rate: self.sample_rate,
            num_frames: n,
            duration_secs: duration,
            peak_linear: peak,
            peak_dbfs,
            rms_linear: rms,
            rms_dbfs,
            crest_factor_db: peak_dbfs - rms_dbfs,
            zero_crossing_hz_est: zc_hz,
            stereo_width_corr: corr,
            dc_offset: sum_dc / n as f64,
            ascii_envelope: ascii,
            mono_compatible: corr > 0.35,
            stereo_width_label: if corr > 0.92 {
                "almost mono".into()
            } else if corr > 0.55 {
                "compatible".into()
            } else if corr > 0.2 {
                "wide".into()
            } else {
                "out of phase / check mono".into()
            },
        }
    }
}

fn tpdf_noise(state: &mut u32) -> f64 {
    *state = state.wrapping_mul(1664525).wrapping_add(1013904223);
    let a = (*state >> 16) as f64 / 65536.0;
    *state = state.wrapping_mul(1664525).wrapping_add(1013904223);
    let b = (*state >> 16) as f64 / 65536.0;
    a + b - 1.0
}

fn write_sample(buf: &mut Vec<u8>, sample: f64, bit_depth: BitDepth) {
    match bit_depth {
        BitDepth::Pcm16 => {
            let v = (sample * 32767.0).round().clamp(-32768.0, 32767.0) as i16;
            buf.extend_from_slice(&v.to_le_bytes());
        }
        BitDepth::Pcm24 => {
            let v = (sample * 8_388_607.0)
                .round()
                .clamp(-8_388_608.0, 8_388_607.0) as i32;
            let b = v.to_le_bytes();
            buf.push(b[0]);
            buf.push(b[1]);
            buf.push(b[2]);
        }
        BitDepth::Float32 => {
            let v = sample as f32;
            buf.extend_from_slice(&v.to_le_bytes());
        }
    }
}

/// Inspect an existing WAV file from disk and return its AudioReport + metadata string.
pub fn inspect_wav_file<P: AsRef<Path>>(path: P) -> Result<(String, AudioReport), String> {
    let mut f = File::open(path.as_ref()).map_err(|e| format!("Failed to open WAV: {e}"))?;
    let mut bytes = Vec::new();
    f.read_to_end(&mut bytes)
        .map_err(|e| format!("Failed to read WAV: {e}"))?;

    if bytes.len() < 44 || &bytes[0..4] != b"RIFF" || &bytes[8..12] != b"WAVE" {
        return Err("Not a valid RIFF/WAVE file".to_string());
    }

    let mut pos = 12usize;
    let mut format_tag = 1u16;
    let mut channels = 2u16;
    let mut sample_rate = 48000u32;
    let mut bits_per_sample = 16u16;
    let mut data_slice: &[u8] = &[];

    while pos + 8 <= bytes.len() {
        let chunk_id = &bytes[pos..pos + 4];
        let chunk_size = u32::from_le_bytes([
            bytes[pos + 4],
            bytes[pos + 5],
            bytes[pos + 6],
            bytes[pos + 7],
        ]) as usize;
        pos += 8;
        let end = (pos + chunk_size).min(bytes.len());
        if chunk_id == b"fmt " && chunk_size >= 16 {
            format_tag = u16::from_le_bytes([bytes[pos], bytes[pos + 1]]);
            channels = u16::from_le_bytes([bytes[pos + 2], bytes[pos + 3]]);
            sample_rate = u32::from_le_bytes([
                bytes[pos + 4],
                bytes[pos + 5],
                bytes[pos + 6],
                bytes[pos + 7],
            ]);
            bits_per_sample = u16::from_le_bytes([bytes[pos + 14], bytes[pos + 15]]);
        } else if chunk_id == b"data" {
            data_slice = &bytes[pos..end];
            break;
        }
        pos = end + (chunk_size & 1);
    }

    let bytes_per_sample = (bits_per_sample / 8).max(1) as usize;
    let frame_bytes = bytes_per_sample * (channels.max(1) as usize);
    let num_frames = data_slice.len() / frame_bytes;
    let mut stereo = StereoBuffer::new(sample_rate, num_frames);

    for i in 0..num_frames {
        let base = i * frame_bytes;
        let l = decode_sample(
            &data_slice[base..base + bytes_per_sample],
            format_tag,
            bits_per_sample,
        );
        let r = if channels >= 2 {
            decode_sample(
                &data_slice[base + bytes_per_sample..base + 2 * bytes_per_sample],
                format_tag,
                bits_per_sample,
            )
        } else {
            l
        };
        stereo.left[i] = l;
        stereo.right[i] = r;
    }

    let fmt_desc = format!(
        "{} Hz, {}-bit ({}), {} ch",
        sample_rate,
        bits_per_sample,
        if format_tag == 3 { "IEEE Float" } else { "PCM" },
        channels
    );
    Ok((fmt_desc, stereo.analyze()))
}

/// Decode a WAV file into a stereo buffer (mono is duplicated).
pub fn load_wav_file<P: AsRef<Path>>(path: P) -> Result<StereoBuffer, String> {
    load_wav_bytes(&fs::read(path.as_ref()).map_err(|e| format!("Failed to read WAV: {e}"))?)
}

/// Decode WAV bytes into a stereo buffer.
pub fn load_wav_bytes(bytes: &[u8]) -> Result<StereoBuffer, String> {
    if bytes.len() < 44 || &bytes[0..4] != b"RIFF" || &bytes[8..12] != b"WAVE" {
        return Err("Not a valid RIFF/WAVE file".to_string());
    }

    let mut pos = 12usize;
    let mut format_tag = 1u16;
    let mut channels = 2u16;
    let mut sample_rate = 48000u32;
    let mut bits_per_sample = 16u16;
    let mut data_slice: &[u8] = &[];

    while pos + 8 <= bytes.len() {
        let chunk_id = &bytes[pos..pos + 4];
        let chunk_size = u32::from_le_bytes([
            bytes[pos + 4],
            bytes[pos + 5],
            bytes[pos + 6],
            bytes[pos + 7],
        ]) as usize;
        pos += 8;
        let end = (pos + chunk_size).min(bytes.len());
        if chunk_id == b"fmt " && chunk_size >= 16 {
            format_tag = u16::from_le_bytes([bytes[pos], bytes[pos + 1]]);
            channels = u16::from_le_bytes([bytes[pos + 2], bytes[pos + 3]]);
            sample_rate = u32::from_le_bytes([
                bytes[pos + 4],
                bytes[pos + 5],
                bytes[pos + 6],
                bytes[pos + 7],
            ]);
            bits_per_sample = u16::from_le_bytes([bytes[pos + 14], bytes[pos + 15]]);
        } else if chunk_id == b"data" {
            data_slice = &bytes[pos..end];
            break;
        }
        pos = end + (chunk_size & 1);
    }

    let bytes_per_sample = (bits_per_sample / 8).max(1) as usize;
    let frame_bytes = bytes_per_sample * (channels.max(1) as usize);
    if frame_bytes == 0 {
        return Err("Invalid WAV frame size".into());
    }
    let num_frames = data_slice.len() / frame_bytes;
    let mut stereo = StereoBuffer::new(sample_rate, num_frames);

    for i in 0..num_frames {
        let base = i * frame_bytes;
        let l = decode_sample(
            &data_slice[base..base + bytes_per_sample],
            format_tag,
            bits_per_sample,
        );
        let r = if channels >= 2 {
            decode_sample(
                &data_slice[base + bytes_per_sample..base + 2 * bytes_per_sample],
                format_tag,
                bits_per_sample,
            )
        } else {
            l
        };
        stereo.left[i] = l;
        stereo.right[i] = r;
    }
    Ok(stereo)
}

fn decode_sample(raw: &[u8], format_tag: u16, bits: u16) -> f64 {
    match (format_tag, bits) {
        (3, 32) if raw.len() >= 4 => {
            f32::from_le_bytes([raw[0], raw[1], raw[2], raw[3]]) as f64
        }
        (_, 24) if raw.len() >= 3 => {
            let sign = if raw[2] & 0x80 != 0 { 0xFF } else { 0x00 };
            let v = i32::from_le_bytes([raw[0], raw[1], raw[2], sign]);
            v as f64 / 8_388_608.0
        }
        (_, 16) if raw.len() >= 2 => {
            let v = i16::from_le_bytes([raw[0], raw[1]]);
            v as f64 / 32768.0
        }
        _ => 0.0,
    }
}
