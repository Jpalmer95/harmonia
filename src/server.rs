//! Built-in HTTP Studio Server (`harmonia serve --port 8765`).
//! Zero external dependencies — uses `std::net::TcpListener` to serve the interactive Web Studio
//! and REST API (`/api/presets`, `/api/render`, `/renders/<file>.wav`).

use crate::analyze::analyze_buffer;
use crate::json::{escape_json, parse_json};
use crate::library::{
    add_block, attach_audio, delete_block, get_block, index_json, mashup_progressions, prune_expired,
    HarmoniaBlock, KeepPolicy,
};
use crate::math_engine::builtin_math_formulas;
use crate::presets::{builtin_compositions, CompositionSpec};
use crate::wav::{load_wav_bytes, load_wav_file, BitDepth};
use std::fs;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

pub fn run_studio_server(port: u16, project_dir: PathBuf) -> Result<(), String> {
    let addr = format!("0.0.0.0:{port}");
    let listener = TcpListener::bind(&addr)
        .map_err(|e| format!("Failed to bind HTTP Studio server on {addr}: {e}"))?;

    println!("╔══════════════════════════════════════════════════════════════════════╗");
    println!("║  HARMONIA HI-DEF FM & MATHEMATICAL MUSIC STUDIO SERVER               ║");
    println!("╠══════════════════════════════════════════════════════════════════════╣");
    println!("║  Local Studio UI : http://127.0.0.1:{:<5}                            ║", port);
    println!("║  API Endpoints   : GET  /api/presets  GET /api/library               ║");
    println!("║                    POST /api/render   POST /api/analyze              ║");
    println!("║                    POST /api/library  POST /api/preview              ║");
    println!("║                    GET  /renders|/library/...                        ║");
    println!("╚══════════════════════════════════════════════════════════════════════╝");

    for stream in listener.incoming() {
        if let Ok(mut s) = stream {
            let dir = project_dir.clone();
            let _ = handle_client(&mut s, &dir);
        }
    }
    Ok(())
}

fn handle_client(stream: &mut TcpStream, project_dir: &PathBuf) -> Result<(), String> {
    let mut header_buf = Vec::new();
    let mut tmp = [0u8; 4096];
    loop {
        let n = stream.read(&mut tmp).map_err(|e| e.to_string())?;
        if n == 0 {
            break;
        }
        header_buf.extend_from_slice(&tmp[..n]);
        if header_buf.windows(4).any(|w| w == b"\r\n\r\n") || header_buf.len() > 1024 * 1024 {
            break;
        }
    }
    if header_buf.is_empty() {
        return Ok(());
    }
    let header_end = header_buf
        .windows(4)
        .position(|w| w == b"\r\n\r\n")
        .map(|p| p + 4)
        .unwrap_or(header_buf.len());
    let header_str = String::from_utf8_lossy(&header_buf[..header_end.min(header_buf.len())]);
    let first_line = header_str.lines().next().unwrap_or("");
    let mut parts = first_line.split_whitespace();
    let method = parts.next().unwrap_or("GET").to_string();
    let path = parts.next().unwrap_or("/").to_string();
    let content_length: usize = header_str
        .lines()
        .find_map(|l| {
            let l = l.to_ascii_lowercase();
            l.strip_prefix("content-length:")
                .and_then(|v| v.trim().parse().ok())
        })
        .unwrap_or(0)
        .min(32 * 1024 * 1024);
    let mut body = header_buf[header_end.min(header_buf.len())..].to_vec();
    while body.len() < content_length {
        let n = stream.read(&mut tmp).map_err(|e| e.to_string())?;
        if n == 0 {
            break;
        }
        body.extend_from_slice(&tmp[..n]);
    }
    if body.len() > content_length {
        body.truncate(content_length);
    }
    dispatch(stream, project_dir, &method, &path, &body)
}

fn dispatch(
    stream: &mut TcpStream,
    project_dir: &PathBuf,
    method: &str,
    path: &str,
    body: &[u8],
) -> Result<(), String> {

    if method == "OPTIONS" {
        return send_response(stream, "200 OK", "text/plain", b"");
    }

    if method == "GET" && (path == "/" || path == "/index.html" || path == "/studio.html") {
        let html_path = project_dir.join("web/studio.html");
        let html = fs::read(&html_path).unwrap_or_else(|_| {
            b"<html><body><h1>Harmonia Studio</h1><p>web/studio.html not found</p></body></html>"
                .to_vec()
        });
        return send_response(stream, "200 OK", "text/html; charset=utf-8", &html);
    }

    if method == "GET" && path == "/api/presets" {
        let comps = builtin_compositions();
        let formulas = builtin_math_formulas();
        let mut json = String::with_capacity(16384);
        json.push_str("{\n  \"compositions\": [\n");
        for (i, c) in comps.iter().enumerate() {
            if i > 0 {
                json.push_str(",\n");
            }
            json.push_str(&c.to_pretty_json());
        }
        json.push_str("\n  ],\n  \"math_formulas\": [\n");
        for (i, f) in formulas.iter().enumerate() {
            if i > 0 {
                json.push_str(",\n");
            }
            json.push_str(&format!(
                "    {{\"id\": \"{}\", \"name\": \"{}\", \"category\": \"{}\", \"formula\": \"{}\", \"description\": \"{}\"}}",
                escape_json(f.id),
                escape_json(f.name),
                escape_json(f.category),
                escape_json(f.formula),
                escape_json(f.description)
            ));
        }
        json.push_str("\n  ],\n  \"patches\": [\n");
        let patches = crate::presets::builtin_patches();
        for (i, p) in patches.iter().enumerate() {
            if i > 0 {
                json.push_str(",\n");
            }
            json.push_str(&format!(
                "    {{\"id\": \"{}\", \"name\": \"{}\", \"algorithm\": \"{}\"}}",
                escape_json(&p.id),
                escape_json(&p.name),
                escape_json(&p.algorithm_name)
            ));
        }
        json.push_str("\n  ]\n}\n");
        return send_response(
            stream,
            "200 OK",
            "application/json; charset=utf-8",
            json.as_bytes(),
        );
    }

    if method == "GET" && path.starts_with("/renders/") {
        let filename = path.trim_start_matches("/renders/");
        if filename.contains("..") || filename.contains('/') {
            return send_response(stream, "400 Bad Request", "text/plain", b"Invalid filename");
        }
        let file_path = project_dir.join("renders").join(filename);
        if let Ok(bytes) = fs::read(&file_path) {
            let ctype = if filename.ends_with(".mid") {
                "audio/midi"
            } else {
                "audio/wav"
            };
            return send_response(stream, "200 OK", ctype, &bytes);
        } else {
            return send_response(stream, "404 Not Found", "text/plain", b"WAV not found");
        }
    }

    if method == "GET" && path.starts_with("/library/") {
        let rel = path.trim_start_matches('/');
        if rel.contains("..") {
            return send_response(stream, "400 Bad Request", "text/plain", b"Invalid path");
        }
        let file_path = project_dir.join(rel);
        if let Ok(bytes) = fs::read(&file_path) {
            let ctype = if rel.ends_with(".mp3") {
                "audio/mpeg"
            } else {
                "audio/wav"
            };
            return send_response(stream, "200 OK", ctype, &bytes);
        }
        return send_response(stream, "404 Not Found", "text/plain", b"Not found");
    }

    if method == "GET" && path == "/api/library" {
        prune_expired(project_dir, false);
        let json = index_json(project_dir);
        return send_response(stream, "200 OK", "application/json; charset=utf-8", json.as_bytes());
    }

    if method == "POST" && path == "/api/analyze" {
        let wav = match load_wav_bytes(body) {
            Ok(w) => w,
            Err(e) => {
                let err = format!("{{\"ok\":false,\"error\":\"{}\"}}", escape_json(&e));
                return send_response(stream, "400 Bad Request", "application/json", err.as_bytes());
            }
        };
        let report = analyze_buffer(&wav);
        let inner = report.to_json();
        let inner = inner.trim();
        let inner = inner.strip_prefix('{').unwrap_or(inner);
        let inner = inner.strip_suffix('}').unwrap_or(inner).trim();
        let resp = format!("{{\n  \"ok\": true,\n  {inner}\n}}");
        return send_response(stream, "200 OK", "application/json", resp.as_bytes());
    }

    if method == "POST" && path == "/api/library" {
        let text = String::from_utf8_lossy(body);
        let parsed = match parse_json(&text) {
            Ok(v) => v,
            Err(e) => {
                let err = format!("{{\"ok\":false,\"error\":\"{}\"}}", escape_json(&e));
                return send_response(stream, "400 Bad Request", "application/json", err.as_bytes());
            }
        };
        let action = parsed.get("action").and_then(|v| v.as_str()).unwrap_or("save");
        match action {
            "delete" => {
                let id = parsed.get("id").and_then(|v| v.as_str()).unwrap_or("");
                let _ = delete_block(project_dir, id);
                let json = index_json(project_dir);
                return send_response(stream, "200 OK", "application/json", json.as_bytes());
            }
            "mash" => {
                let a = parsed.get("a").and_then(|v| v.as_str()).unwrap_or("");
                let b = parsed.get("b").and_then(|v| v.as_str()).unwrap_or("");
                let ba = get_block(project_dir, a);
                let bb = get_block(project_dir, b);
                if let (Some(ba), Some(bb)) = (ba, bb) {
                    let prog = mashup_progressions(&ba.progression, &bb.progression);
                    let mut block = ba.clone();
                    block.id = format!("mash_{a}_{b}");
                    block.title = format!("{} × {}", ba.title, bb.title);
                    block.kind = "mashup".into();
                    block.progression = prog.clone();
                    let _ = add_block(project_dir, block);
                    let resp = format!(
                        "{{\"ok\":true,\"progression\":\"{}\",\"id\":\"mash_{}_{}\"}}",
                        escape_json(&prog),
                        escape_json(a),
                        escape_json(b)
                    );
                    return send_response(stream, "200 OK", "application/json", resp.as_bytes());
                }
                return send_response(
                    stream,
                    "404 Not Found",
                    "application/json",
                    b"{\"ok\":false,\"error\":\"missing block\"}",
                );
            }
            _ => {
                prune_expired(project_dir, parsed.get("next_generation").and_then(|v| v.as_bool()).unwrap_or(false));
                let id = parsed
                    .get("id")
                    .and_then(|v| v.as_str())
                    .unwrap_or("block")
                    .to_string();
                let format = parsed.get("format").and_then(|v| v.as_str()).unwrap_or("json");
                let keep = KeepPolicy::from_str(
                    parsed.get("keep").and_then(|v| v.as_str()).unwrap_or("library"),
                );
                let mut block = HarmoniaBlock {
                    id: id.clone(),
                    title: parsed
                        .get("title")
                        .and_then(|v| v.as_str())
                        .unwrap_or("Saved")
                        .to_string(),
                    kind: parsed
                        .get("kind")
                        .and_then(|v| v.as_str())
                        .unwrap_or("composition")
                        .to_string(),
                    created_unix: SystemTime::now()
                        .duration_since(UNIX_EPOCH)
                        .map(|d| d.as_secs())
                        .unwrap_or(0),
                    keep,
                    progression: parsed
                        .get("progression")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string(),
                    bpm: parsed.get("bpm").and_then(|v| v.as_f64()).unwrap_or(110.0),
                    pattern: parsed
                        .get("pattern")
                        .and_then(|v| v.as_str())
                        .unwrap_or("cthulhu_cascade")
                        .to_string(),
                    tuning: parsed
                        .get("tuning")
                        .and_then(|v| v.as_str())
                        .unwrap_or("equal")
                        .to_string(),
                    patch_id: parsed
                        .get("patch_id")
                        .and_then(|v| v.as_str())
                        .unwrap_or("retro_classical_brass")
                        .to_string(),
                    math_expr: parsed
                        .get("math_expr")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string(),
                    notes: String::new(),
                    wav_rel: None,
                    mp3_rel: None,
                    analyze_json: None,
                };
                if format == "wav" || format == "mp3" || format == "both" {
                    if let Some(wav_name) = parsed.get("wav_file").and_then(|v| v.as_str()) {
                        let p = project_dir.join("renders").join(wav_name);
                        match load_wav_file(&p) {
                            Ok(buf) => match attach_audio(project_dir, &id, &buf, format) {
                                Ok((w, m)) => {
                                    block.wav_rel = w;
                                    block.mp3_rel = m;
                                }
                                Err(e) => {
                                    let err = format!(
                                        "{{\"ok\":false,\"error\":\"{}\"}}",
                                        escape_json(&e)
                                    );
                                    return send_response(
                                        stream,
                                        "400 Bad Request",
                                        "application/json",
                                        err.as_bytes(),
                                    );
                                }
                            },
                            Err(e) => {
                                let err = format!(
                                    "{{\"ok\":false,\"error\":\"could not load render WAV: {}\"}}",
                                    escape_json(&e)
                                );
                                return send_response(
                                    stream,
                                    "400 Bad Request",
                                    "application/json",
                                    err.as_bytes(),
                                );
                            }
                        }
                    } else {
                        return send_response(
                            stream,
                            "400 Bad Request",
                            "application/json",
                            b"{\"ok\":false,\"error\":\"Render a WAV first, then save audio.\"}",
                        );
                    }
                }
                let saved_id = block.id.clone();
                let wav_rel = block.wav_rel.clone();
                let _ = add_block(project_dir, block);
                let mut json = index_json(project_dir);
                // index_json already has "ok": true — also surface the saved id for the UI.
                if json.starts_with('{') {
                    json = format!(
                        "{{\n  \"saved_id\": \"{}\",\n  \"saved_wav\": {},{}",
                        escape_json(&saved_id),
                        match &wav_rel {
                            Some(p) => format!("\"{}\"", escape_json(p)),
                            None => "null".into(),
                        },
                        json.trim_start_matches('{')
                    );
                }
                return send_response(stream, "200 OK", "application/json", json.as_bytes());
            }
        }
    }

    if method == "POST" && path == "/api/lint" {
        let text = String::from_utf8_lossy(body);
        let parsed = parse_json(&text).ok();
        let formula = parsed
            .as_ref()
            .and_then(|v| v.get("math_expr").and_then(|x| x.as_str()))
            .unwrap_or(text.trim());
        if formula.is_empty() {
            return send_response(
                stream,
                "200 OK",
                "application/json",
                b"{\"ok\":true,\"empty\":true}",
            );
        }
        match crate::math_engine::CompiledExpr::compile(formula) {
            Ok(_) => {
                let resp = format!(
                    "{{\"ok\":true,\"empty\":false,\"ops\":0,\"message\":\"formula compiles\"}}"
                );
                return send_response(stream, "200 OK", "application/json", resp.as_bytes());
            }
            Err(e) => {
                let resp = format!("{{\"ok\":false,\"error\":\"{}\"}}", escape_json(&e));
                return send_response(stream, "200 OK", "application/json", resp.as_bytes());
            }
        }
    }

    if method == "POST" && path == "/api/preview" {
        let text = String::from_utf8_lossy(body);
        let parsed = match parse_json(&text) {
            Ok(v) => v,
            Err(e) => {
                let err = format!("{{\"ok\":false,\"error\":\"{}\"}}", escape_json(&e));
                return send_response(stream, "400 Bad Request", "application/json", err.as_bytes());
            }
        };
        let token = parsed
            .get("token")
            .and_then(|v| v.as_str())
            .or_else(|| parsed.get("chord").and_then(|v| v.as_str()))
            .unwrap_or("")
            .trim()
            .to_string();
        if token.is_empty() {
            return send_response(
                stream,
                "400 Bad Request",
                "application/json",
                b"{\"ok\":false,\"error\":\"token is required (e.g. Cm9 or C)\"}",
            );
        }
        let seconds = parsed
            .get("seconds")
            .and_then(|v| v.as_f64())
            .unwrap_or(1.15)
            .clamp(0.35, 2.4);
        let spec = match CompositionSpec::from_json_val(&parsed) {
            Ok(mut s) => {
                s.progression = token.clone();
                s.beats_per_chord = (seconds * s.bpm / 60.0).max(0.5);
                s
            }
            Err(e) => {
                let err = format!("{{\"ok\":false,\"error\":\"{}\"}}", escape_json(&e));
                return send_response(stream, "400 Bad Request", "application/json", err.as_bytes());
            }
        };
        match spec.preview_token(&token, seconds) {
            Ok(buf) => {
                let bytes = buf.to_wav_bytes(BitDepth::Pcm16);
                return send_response(stream, "200 OK", "audio/wav", &bytes);
            }
            Err(e) => {
                let err = format!("{{\"ok\":false,\"error\":\"{}\"}}", escape_json(&e));
                return send_response(stream, "400 Bad Request", "application/json", err.as_bytes());
            }
        }
    }

    if method == "POST" && path == "/api/midi" {
        let text = String::from_utf8_lossy(body);
        let parsed = match parse_json(&text) {
            Ok(v) => v,
            Err(e) => {
                let err = format!("{{\"ok\":false,\"error\":\"{}\"}}", escape_json(&e));
                return send_response(stream, "400 Bad Request", "application/json", err.as_bytes());
            }
        };
        let spec = match CompositionSpec::from_json_val(&parsed) {
            Ok(s) => s,
            Err(e) => {
                let err = format!("{{\"ok\":false,\"error\":\"{}\"}}", escape_json(&e));
                return send_response(stream, "400 Bad Request", "application/json", err.as_bytes());
            }
        };
        let chords = crate::theory::parse_progression(&spec.progression, spec.beats_per_chord);
        let (events, _) = crate::theory::generate_multiline_events(
            &chords,
            spec.bpm,
            spec.pattern.clone(),
            spec.tuning.clone(),
        );
        let midi = crate::midi::MidiFile::from_note_events(&events, spec.bpm, |ev| {
            crate::midi::hz_to_midi(ev.target_freq_hz)
        });
        let ts = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        let safe_id: String = spec
            .id
            .chars()
            .filter(|c| c.is_ascii_alphanumeric() || *c == '_' || *c == '-')
            .collect();
        let filename = format!("{}_{}.mid", safe_id, ts);
        let renders_dir = project_dir.join("renders");
        let _ = fs::create_dir_all(&renders_dir);
        let out_path = renders_dir.join(&filename);
        if let Err(e) = crate::midi::save_smf(&out_path, &midi) {
            let err = format!("{{\"ok\":false,\"error\":\"{}\"}}", escape_json(&e));
            return send_response(stream, "500 Internal Server Error", "application/json", err.as_bytes());
        }
        let resp = format!(
            "{{\"ok\":true,\"midi_url\":\"/renders/{}\",\"notes\":{},\"bpm\":{:.1}}}",
            escape_json(&filename),
            midi.notes.len(),
            midi.bpm()
        );
        return send_response(stream, "200 OK", "application/json", resp.as_bytes());
    }

    if method == "POST" && path == "/api/render" {
        let body_str = String::from_utf8_lossy(body);
        let parsed = match parse_json(&body_str) {
            Ok(v) => v,
            Err(e) => {
                let err_json = format!("{{\"ok\": false, \"error\": \"{}\"}}", escape_json(&e));
                return send_response(
                    stream,
                    "400 Bad Request",
                    "application/json",
                    err_json.as_bytes(),
                );
            }
        };

        let sr = parsed
            .get("sample_rate")
            .and_then(|v| v.as_f64())
            .map(|n| n as u32)
            .unwrap_or(48000)
            .clamp(22050, 192000);
        let bits_str = parsed
            .get("bit_depth")
            .and_then(|v| v.as_str())
            .unwrap_or("24");
        let bit_depth = BitDepth::from_str(bits_str);

        let spec = match CompositionSpec::from_json_val(&parsed) {
            Ok(s) => s,
            Err(e) => {
                let err_json = format!("{{\"ok\": false, \"error\": \"{}\"}}", escape_json(&e));
                return send_response(
                    stream,
                    "400 Bad Request",
                    "application/json",
                    err_json.as_bytes(),
                );
            }
        };

        let t0 = std::time::Instant::now();
        match spec.render(sr) {
            Ok(stereo) => {
                let elapsed_ms = t0.elapsed().as_secs_f64() * 1000.0;
                let report = stereo.analyze();
                let ts = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .map(|d| d.as_secs())
                    .unwrap_or(0);
                let safe_id: String = spec
                    .id
                    .chars()
                    .filter(|c| c.is_ascii_alphanumeric() || *c == '_' || *c == '-')
                    .collect();
                let filename = format!("{}_{}.wav", safe_id, ts);
                let renders_dir = project_dir.join("renders");
                let _ = fs::create_dir_all(&renders_dir);
                let out_path = renders_dir.join(&filename);
                if let Err(e) = stereo.save_wav(&out_path, bit_depth) {
                    let err_json =
                        format!("{{\"ok\": false, \"error\": \"{}\"}}", escape_json(&e.to_string()));
                    return send_response(
                        stream,
                        "500 Internal Server Error",
                        "application/json",
                        err_json.as_bytes(),
                    );
                }

                let resp = format!(
                    "{{\n  \"ok\": true,\n  \"wav_url\": \"/renders/{}\",\n  \"file_path\": \"{}\",\n  \"render_ms\": {:.2},\n  \"sample_rate\": {},\n  \"bit_depth\": \"{}\",\n  \"duration_secs\": {:.2},\n  \"peak_dbfs\": {:.2},\n  \"rms_dbfs\": {:.2},\n  \"crest_factor_db\": {:.2},\n  \"stereo_corr\": {:.3},\n  \"mono_compatible\": {},\n  \"stereo_width_label\": \"{}\",\n  \"ascii_envelope\": \"{}\"\n}}",
                    escape_json(&filename),
                    escape_json(&out_path.to_string_lossy()),
                    elapsed_ms,
                    sr,
                    bit_depth.label(),
                    report.duration_secs,
                    report.peak_dbfs,
                    report.rms_dbfs,
                    report.crest_factor_db,
                    report.stereo_width_corr,
                    if report.mono_compatible { "true" } else { "false" },
                    escape_json(&report.stereo_width_label),
                    escape_json(&report.ascii_envelope)
                );
                return send_response(stream, "200 OK", "application/json", resp.as_bytes());
            }
            Err(e) => {
                let err_json = format!("{{\"ok\": false, \"error\": \"{}\"}}", escape_json(&e));
                return send_response(
                    stream,
                    "400 Bad Request",
                    "application/json",
                    err_json.as_bytes(),
                );
            }
        }
    }

    send_response(stream, "404 Not Found", "text/plain", b"Not Found")
}

fn send_response(
    stream: &mut TcpStream,
    status: &str,
    content_type: &str,
    body: &[u8],
) -> Result<(), String> {
    let header = format!(
        "HTTP/1.1 {status}\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\nAccess-Control-Allow-Origin: *\r\nAccess-Control-Allow-Methods: GET, POST, OPTIONS\r\nAccess-Control-Allow-Headers: Content-Type\r\nConnection: close\r\n\r\n",
        body.len()
    );
    stream
        .write_all(header.as_bytes())
        .map_err(|e| e.to_string())?;
    stream.write_all(body).map_err(|e| e.to_string())?;
    stream.flush().map_err(|e| e.to_string())
}
