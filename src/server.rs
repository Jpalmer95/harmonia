//! Built-in HTTP Studio Server (`harmonia serve --port 8765`).
//! Zero external dependencies — uses `std::net::TcpListener` to serve the interactive Web Studio
//! and REST API (`/api/presets`, `/api/render`, `/renders/<file>.wav`).

use crate::json::{escape_json, parse_json};
use crate::math_engine::builtin_math_formulas;
use crate::presets::{builtin_compositions, CompositionSpec};
use crate::wav::BitDepth;
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
    println!("║  API Endpoints   : GET  /api/presets                                 ║");
    println!("║                    POST /api/render                                  ║");
    println!("║                    GET  /renders/<name>.wav                          ║");
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
    let mut buf = vec![0u8; 65536];
    let n = stream.read(&mut buf).map_err(|e| e.to_string())?;
    if n == 0 {
        return Ok(());
    }
    let req_str = String::from_utf8_lossy(&buf[..n]);
    let first_line = req_str.lines().next().unwrap_or("");
    let mut parts = first_line.split_whitespace();
    let method = parts.next().unwrap_or("GET");
    let path = parts.next().unwrap_or("/");

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
            return send_response(stream, "200 OK", "audio/wav", &bytes);
        } else {
            return send_response(stream, "404 Not Found", "text/plain", b"WAV not found");
        }
    }

    if method == "POST" && path == "/api/render" {
        let body = req_str
            .split("\r\n\r\n")
            .nth(1)
            .or_else(|| req_str.split("\n\n").nth(1))
            .unwrap_or("{}");

        let parsed = match parse_json(body) {
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
                    "{{\n  \"ok\": true,\n  \"wav_url\": \"/renders/{}\",\n  \"file_path\": \"{}\",\n  \"render_ms\": {:.2},\n  \"sample_rate\": {},\n  \"bit_depth\": \"{}\",\n  \"duration_secs\": {:.2},\n  \"peak_dbfs\": {:.2},\n  \"rms_dbfs\": {:.2},\n  \"crest_factor_db\": {:.2},\n  \"stereo_corr\": {:.3},\n  \"ascii_envelope\": \"{}\"\n}}",
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
