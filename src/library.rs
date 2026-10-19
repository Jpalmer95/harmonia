//! Harmonia Blocks library — JSON-canonical snippets the user can keep, mash, or expire.
//!
//! Canonical store: `library/index.json` (git-friendly). WAV/MP3 under `library/audio/`
//! are optional derived media. Unsaved studio renders live in `renders/` and may be
//! pruned on the next generation or after a TTL.

use crate::analyze::AnalyzeReport;
use crate::json::{escape_json, parse_json, JsonVal};
use crate::wav::{BitDepth, StereoBuffer};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

pub const DEFAULT_TTL_SECS: u64 = 24 * 3600;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeepPolicy {
    /// Keep until the user deletes it.
    Library,
    /// Delete after TTL (default 24h) if never promoted.
    Ttl { secs: u64 },
    /// Delete as soon as the next render starts.
    NextGeneration,
}

impl KeepPolicy {
    pub fn as_str(&self) -> String {
        match self {
            KeepPolicy::Library => "library".into(),
            KeepPolicy::Ttl { secs } => format!("ttl:{secs}"),
            KeepPolicy::NextGeneration => "next_generation".into(),
        }
    }

    pub fn from_str(s: &str) -> Self {
        let t = s.trim().to_ascii_lowercase();
        if t == "library" || t == "keep" || t == "permanent" {
            KeepPolicy::Library
        } else if t == "next" || t == "next_generation" || t == "ephemeral" {
            KeepPolicy::NextGeneration
        } else if let Some(rest) = t.strip_prefix("ttl:") {
            KeepPolicy::Ttl {
                secs: rest.parse().unwrap_or(DEFAULT_TTL_SECS),
            }
        } else {
            KeepPolicy::Ttl {
                secs: DEFAULT_TTL_SECS,
            }
        }
    }
}

#[derive(Debug, Clone)]
pub struct HarmoniaBlock {
    pub id: String,
    pub title: String,
    pub kind: String, // composition | sketch | mashup | import
    pub created_unix: u64,
    pub keep: KeepPolicy,
    pub progression: String,
    pub bpm: f64,
    pub pattern: String,
    pub tuning: String,
    pub patch_id: String,
    pub math_expr: String,
    pub notes: String,
    pub wav_rel: Option<String>,
    pub mp3_rel: Option<String>,
    pub analyze_json: Option<String>,
}

impl HarmoniaBlock {
    pub fn to_json_obj(&self) -> String {
        format!(
            "{{\n  \"id\": \"{}\",\n  \"title\": \"{}\",\n  \"kind\": \"{}\",\n  \"created_unix\": {},\n  \"keep\": \"{}\",\n  \"progression\": \"{}\",\n  \"bpm\": {:.2},\n  \"pattern\": \"{}\",\n  \"tuning\": \"{}\",\n  \"patch_id\": \"{}\",\n  \"math_expr\": \"{}\",\n  \"notes\": \"{}\",\n  \"wav\": {},\n  \"mp3\": {}\n}}",
            escape_json(&self.id),
            escape_json(&self.title),
            escape_json(&self.kind),
            self.created_unix,
            escape_json(&self.keep.as_str()),
            escape_json(&self.progression),
            self.bpm,
            escape_json(&self.pattern),
            escape_json(&self.tuning),
            escape_json(&self.patch_id),
            escape_json(&self.math_expr),
            escape_json(&self.notes),
            match &self.wav_rel {
                Some(p) => format!("\"{}\"", escape_json(p)),
                None => "null".into(),
            },
            match &self.mp3_rel {
                Some(p) => format!("\"{}\"", escape_json(p)),
                None => "null".into(),
            },
        )
    }
}

pub fn library_root(project_dir: &Path) -> PathBuf {
    project_dir.join("library")
}

pub fn now_unix() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

pub fn load_index(project_dir: &Path) -> Vec<HarmoniaBlock> {
    let path = library_root(project_dir).join("index.json");
    let Ok(raw) = fs::read_to_string(&path) else {
        return Vec::new();
    };
    parse_index(&raw)
}

fn parse_index(raw: &str) -> Vec<HarmoniaBlock> {
    let Ok(val) = parse_json(raw) else {
        return Vec::new();
    };
    let items: Vec<JsonVal> = match &val {
        JsonVal::Arr(a) => a.clone(),
        JsonVal::Obj(o) => o
            .get("blocks")
            .and_then(|v| v.as_arr())
            .map(|a| a.to_vec())
            .unwrap_or_default(),
        _ => return Vec::new(),
    };
    items.iter().filter_map(block_from_json).collect()
}

fn block_from_json(v: &JsonVal) -> Option<HarmoniaBlock> {
    Some(HarmoniaBlock {
        id: v.get("id")?.as_str()?.to_string(),
        title: v
            .get("title")
            .and_then(|x| x.as_str())
            .unwrap_or("untitled")
            .to_string(),
        kind: v
            .get("kind")
            .and_then(|x| x.as_str())
            .unwrap_or("composition")
            .to_string(),
        created_unix: v.get("created_unix").and_then(|x| x.as_f64()).unwrap_or(0.0) as u64,
        keep: KeepPolicy::from_str(v.get("keep").and_then(|x| x.as_str()).unwrap_or("library")),
        progression: v
            .get("progression")
            .and_then(|x| x.as_str())
            .unwrap_or("")
            .to_string(),
        bpm: v.get("bpm").and_then(|x| x.as_f64()).unwrap_or(110.0),
        pattern: v
            .get("pattern")
            .and_then(|x| x.as_str())
            .unwrap_or("cthulhu_cascade")
            .to_string(),
        tuning: v
            .get("tuning")
            .and_then(|x| x.as_str())
            .unwrap_or("equal")
            .to_string(),
        patch_id: v
            .get("patch_id")
            .and_then(|x| x.as_str())
            .unwrap_or("retro_classical_brass")
            .to_string(),
        math_expr: v
            .get("math_expr")
            .and_then(|x| x.as_str())
            .unwrap_or("")
            .to_string(),
        notes: v.get("notes").and_then(|x| x.as_str()).unwrap_or("").to_string(),
        wav_rel: v.get("wav").and_then(|x| x.as_str()).map(|s| s.to_string()),
        mp3_rel: v.get("mp3").and_then(|x| x.as_str()).map(|s| s.to_string()),
        analyze_json: None,
    })
}

pub fn save_index(project_dir: &Path, blocks: &[HarmoniaBlock]) -> Result<(), String> {
    let dir = library_root(project_dir);
    fs::create_dir_all(dir.join("audio")).map_err(|e| e.to_string())?;
    let mut s = String::from("{\n  \"blocks\": [\n");
    for (i, b) in blocks.iter().enumerate() {
        if i > 0 {
            s.push_str(",\n");
        }
        s.push_str(&b.to_json_obj());
    }
    s.push_str("\n  ]\n}\n");
    fs::write(dir.join("index.json"), s).map_err(|e| e.to_string())
}

pub fn prune_expired(project_dir: &Path, next_generation: bool) -> usize {
    let mut blocks = load_index(project_dir);
    let now = now_unix();
    let before = blocks.len();
    blocks.retain(|b| match b.keep {
        KeepPolicy::Library => true,
        KeepPolicy::NextGeneration => !next_generation,
        KeepPolicy::Ttl { secs } => now < b.created_unix.saturating_add(secs),
    });
    let dropped = before - blocks.len();
    if dropped > 0 {
        let _ = save_index(project_dir, &blocks);
    }
    dropped
}

pub fn add_block(project_dir: &Path, block: HarmoniaBlock) -> Result<HarmoniaBlock, String> {
    let mut blocks = load_index(project_dir);
    blocks.retain(|b| b.id != block.id);
    blocks.insert(0, block.clone());
    save_index(project_dir, &blocks)?;
    Ok(block)
}

pub fn get_block(project_dir: &Path, id: &str) -> Option<HarmoniaBlock> {
    load_index(project_dir)
        .into_iter()
        .find(|b| b.id == id)
}

pub fn delete_block(project_dir: &Path, id: &str) -> Result<bool, String> {
    let mut blocks = load_index(project_dir);
    let before = blocks.len();
    if let Some(b) = blocks.iter().find(|b| b.id == id) {
        if let Some(rel) = &b.wav_rel {
            let _ = fs::remove_file(project_dir.join(rel));
        }
        if let Some(rel) = &b.mp3_rel {
            let _ = fs::remove_file(project_dir.join(rel));
        }
    }
    blocks.retain(|b| b.id != id);
    save_index(project_dir, &blocks)?;
    Ok(blocks.len() < before)
}

/// Copy or encode audio beside a block. `format`: "wav" | "mp3" | "json".
pub fn attach_audio(
    project_dir: &Path,
    block_id: &str,
    wav: &StereoBuffer,
    format: &str,
) -> Result<(Option<String>, Option<String>), String> {
    let audio_dir = library_root(project_dir).join("audio");
    fs::create_dir_all(&audio_dir).map_err(|e| e.to_string())?;
    let mut wav_rel = None;
    let mut mp3_rel = None;
    if format == "wav" || format == "both" {
        let rel = format!("library/audio/{block_id}.wav");
        wav.save_wav(project_dir.join(&rel), BitDepth::Pcm24)
            .map_err(|e| e.to_string())?;
        wav_rel = Some(rel);
    }
    if format == "mp3" || format == "both" {
        let wav_path = project_dir.join(format!("library/audio/{block_id}.wav"));
        if !wav_path.exists() {
            wav.save_wav(&wav_path, BitDepth::Pcm16)
                .map_err(|e| e.to_string())?;
            wav_rel = Some(format!("library/audio/{block_id}.wav"));
        }
        let mp3_path = project_dir.join(format!("library/audio/{block_id}.mp3"));
        let ffmpeg = which_ffmpeg();
        let status = Command::new(&ffmpeg)
            .args([
                "-y",
                "-i",
                wav_path.to_str().unwrap_or(""),
                "-codec:a",
                "libmp3lame",
                "-q:a",
                "4",
                mp3_path.to_str().unwrap_or(""),
            ])
            .status();
        match status {
            Ok(s) if s.success() && mp3_path.exists() => {
                mp3_rel = Some(format!("library/audio/{block_id}.mp3"));
            }
            _ => {
                return Err(
                    "MP3 encode needs ffmpeg with libmp3lame. WAV was saved; install ffmpeg or save as WAV."
                        .into(),
                );
            }
        }
    }
    Ok((wav_rel, mp3_rel))
}

fn which_ffmpeg() -> String {
    let hermes = "/home/jonathan/.hermes/tools/ffmpeg-9.0.1-linux-x64/bin/ffmpeg";
    if Path::new(hermes).exists() {
        return hermes.into();
    }
    "ffmpeg".into()
}

pub fn mashup_progressions(a: &str, b: &str) -> String {
    let pa: Vec<&str> = a
        .split("->")
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .collect();
    let pb: Vec<&str> = b
        .split("->")
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .collect();
    if pa.is_empty() {
        return b.to_string();
    }
    if pb.is_empty() {
        return a.to_string();
    }
    let mut out = Vec::new();
    let n = pa.len().max(pb.len());
    for i in 0..n {
        out.push(pa[i % pa.len()]);
        out.push(pb[i % pb.len()]);
    }
    out.join(" -> ")
}

pub fn block_from_analyze(title: &str, report: &AnalyzeReport) -> HarmoniaBlock {
    HarmoniaBlock {
        id: format!("sketch_{}", now_unix()),
        title: title.to_string(),
        kind: "sketch".into(),
        created_unix: now_unix(),
        keep: KeepPolicy::Library,
        progression: report.progression.clone(),
        bpm: if report.bpm_est > 40.0 {
            report.bpm_est.round()
        } else {
            110.0
        },
        pattern: "cthulhu_cascade".into(),
        tuning: "equal".into(),
        patch_id: "retro_classical_brass".into(),
        math_expr: String::new(),
        notes: format!(
            "Imported sketch (confidence {:.0}%). {}",
            report.confidence * 100.0,
            report.caveats.first().cloned().unwrap_or_default()
        ),
        wav_rel: None,
        mp3_rel: None,
        analyze_json: Some(report.to_json()),
    }
}

pub fn index_json(project_dir: &Path) -> String {
    let blocks = load_index(project_dir);
    let mut s = String::from("{\n  \"ok\": true,\n  \"blocks\": [\n");
    for (i, b) in blocks.iter().enumerate() {
        if i > 0 {
            s.push_str(",\n");
        }
        s.push_str(&b.to_json_obj());
    }
    s.push_str("\n  ]\n}\n");
    s
}
