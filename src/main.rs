//! Harmonia CLI — Hi-Def 6-Operator FM Matrix Synthesizer, Symbolic Math DSP Engine,
//! and Multiline Chord Progression Workstation.

use harmonia::effects::process_master_chain;
use harmonia::fm_synth::FmPatch;
use harmonia::math_engine::{builtin_math_formulas, CompiledExpr, MathContext};
use harmonia::oscillator::AdsrSpec;
use harmonia::presets::{
    builtin_compositions, builtin_patches, find_composition, find_patch, CompositionSpec,
};
use harmonia::server::run_studio_server;
use harmonia::theory::{MultilinePattern, TuningSystem};
use harmonia::wav::{inspect_wav_file, BitDepth, StereoBuffer};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Instant;

fn main() {
    let args: Vec<String> = env::args().skip(1).collect();
    if args.is_empty() || args[0] == "-h" || args[0] == "--help" || args[0] == "help" {
        print_help();
        return;
    }

    let cmd = args[0].as_str();
    let rest = &args[1..];

    let res = match cmd {
        "list" | "ls" => cmd_list(),
        "render" => cmd_render(rest),
        "chord" | "chords" | "prog" => cmd_chord(rest),
        "math" | "eq" | "formula" => cmd_math(rest),
        "inspect" | "analyze" => cmd_inspect(rest),
        "play" => cmd_play(rest),
        "export-presets" | "export" => cmd_export_presets(rest),
        "serve" | "studio" => cmd_serve(rest),
        "midi" => cmd_midi(rest),
        "dx7" => cmd_dx7(rest),
        other => Err(format!(
            "Unknown subcommand '{other}'. Run `harmonia --help` for usage."
        )),
    };

    if let Err(e) = res {
        eprintln!("Error: {e}");
        std::process::exit(1);
    }
}

fn print_help() {
    println!(
        r#"╔══════════════════════════════════════════════════════════════════════════════╗
║  HARMONIA v0.1.0 — Hi-Def 6-Op FM Matrix, Math DSP & Multiline Chord Engine  ║
╚══════════════════════════════════════════════════════════════════════════════╝

USAGE:
  harmonia <SUBCOMMAND> [OPTIONS]

SUBCOMMANDS:
  list                      List built-in compositions, FM patches, math formulas,
                            multiline patterns, and microtonal tuning systems.

  render <id_or_json>       Render a built-in composition ID or custom .json spec file.
    -o, --out <path.wav>    Output WAV file path (default: renders/<id>.wav)
    --sr <hz>               Sample rate: 44100, 48000, 96000, 192000 (default: 48000)
    --bits <16|24|32f>      Bit depth: 24-bit PCM or 32-bit IEEE Float (default: 24)
    --play                  Play audio immediately after rendering

  chord "<progression>"     Synthesize a multiline frequency-modulated chord progression!
    --patch <id>            FM patch: retro_classical_brass | glass_cathedral |
                            cyber_baroque_harpsichord | math_harmonic_weaver
    --pattern <weave>       Multiline weave: cthulhu_cascade | bach_prelude |
                            sustained_cathedral | stranger_pulse | polyrhythm_3v4 |
                            counterpoint_four_voice
    --tuning <system>       Tuning: equal | just | 432 | pythagorean | harmonic | phi
    --bpm <bpm>             Tempo in BPM (default: 110)
    --beats <n>             Beats per chord (default: 4.0)
    --lfo-rate <hz>         LFO1 rate modulating FM operator index (default: patch value)
    --lfo-depth <val>       LFO1 FM index modulation depth multiplier
    --glide <secs>          Portamento pitch-glide time between voices (e.g. 0.06)
    --cutoff <hz>           ZDF State-Variable Filter cutoff in Hz
    --shimmer <0..0.6>      Cathedral octave shimmer reverb amount
    --math "<formula>"      Optional symbolic math expression layered with the FM matrix
    --sr <hz>               Sample rate (default: 48000, try 96000 for studio master)
    --bits <16|24|32f>      Bit depth (default: 24)
    -o, --out <path.wav>    Output WAV path
    --play                  Play immediately after rendering

  math "<formula>"          Synthesize sound or multiline chords from a mathematical equation!
    --freq <hz>             Fundamental frequency in Hz for single-note exploration (default: 220)
    --chords "<prog>"       Optional multiline chord progression driven by the math equation!
    --pattern <weave>       Multiline pattern when --chords is used (default: cthulhu_cascade)
    --tuning <system>       Tuning system (default: just)
    --bpm <bpm>             Tempo in BPM (default: 108)
    --duration <secs>       Duration in seconds for single-note mode (default: 6.0)
    --fm-blend <0..1>       Blend ratio of 6-Op FM warmth under the math formula (default: 0.22)
    --sr <hz>               Sample rate (default: 48000)
    --bits <16|24|32f>      Bit depth (default: 24)
    -o, --out <path.wav>    Output WAV path
    --play                  Play immediately after rendering

  inspect <path.wav>        Analyze a WAV file (peak/RMS dBFS, crest factor, ASCII envelope)
  play <path.wav|id>        Play a WAV file or render & play a composition ID via system audio
  export-presets [dir]      Export all built-in compositions as editable .json files
  serve [--port 8765]       Launch the interactive Harmonia Web Studio UI & HTTP API
  midi export <id> -o f.mid Write a Standard MIDI File of a composition's voice-led events
  midi render <file.mid>    Render an SMF through the FM engine
  dx7 <1-32>                Preview a Yamaha DX7 algorithm on the glass cathedral patch
"#
    );
}

fn cmd_list() -> Result<(), String> {
    println!("═══ 1. BUILT-IN FLAGSHIP COMPOSITIONS (`harmonia render <id>`) ═══");
    for c in builtin_compositions() {
        println!(
            "  • {:<26} │ {:<44} │ {} BPM │ {}",
            c.id,
            c.title,
            c.bpm,
            c.pattern.as_str()
        );
        println!("    Chords : {}", c.progression);
        println!("    Tuning : {} │ Patch: {}", c.tuning.label(), c.patch.name);
    }

    println!("\n═══ 2. 6-OPERATOR FM MATRIX PATCHES (`--patch <id>`) ═══");
    for p in builtin_patches() {
        println!("  • {:<26} │ {} ({})", p.id, p.name, p.algorithm_name);
    }

    println!("\n═══ 3. SYMBOLIC MATH FORMULA PRESETS (`harmonia math \"<formula>\"`) ═══");
    for m in builtin_math_formulas() {
        println!("  • {:<26} │ {}", m.id, m.name);
        println!("    Formula: {}", m.formula);
    }

    println!("\n═══ 4. MULTILINE VOICE-WEAVING PATTERNS (`--pattern <name>`) ═══");
    println!("  • cthulhu_cascade          │ Xfer Cthulhu-style multi-octave arpeggio + SATB voice-led FM pad");
    println!("  • bach_prelude             │ J.S. Bach WTC Prelude 16th-note baroque weave + pedal bass");
    println!("  • sustained_cathedral      │ Lush voice-led cathedral chords + slow upper counter-melody");
    println!("  • stranger_pulse           │ 1984 driving 8th bass + 16th synth ostinato + FM brass swell");
    println!("  • polyrhythm_3v4           │ Philip Glass / modern 3-against-4 polyrhythmic voice weave");
    println!("  • counterpoint_four_voice  │ 4 independent contrapuntal lines with portamento pitch glide");

    println!("\n═══ 5. MICROTONAL & CLASSICAL TUNING SYSTEMS (`--tuning <name>`) ═══");
    println!("  • equal       │ 12-Tone Equal Temperament (A4 = 440 Hz)");
    println!("  • just        │ Pure 5/7-Limit Ptolemaic Just Intonation (beatless pure ratios!)");
    println!("  • 432         │ 12-TET Verdi / Warm Reference (A4 = 432 Hz)");
    println!("  • pythagorean │ Pure Stacked 3:2 Fifths Tuning");
    println!("  • harmonic    │ Pure Spectral Overtone Series (Harmonics 1..32)");
    println!("  • phi         │ Golden Ratio (Phi = 1.618034) Microtonal Intervals");
    println!("  • edo19/31/53 │ Equal divisions of the octave (Scala)");
    println!("  • quarter_comma_meantone │ Historical 1/4-comma meantone");
    println!("  • werckmeister_iii / young_1799 │ Well temperaments");
    println!("  • bp_lambda   │ Bohlen–Pierce 13-ED3");

    println!("\n═══ 6. OPEN FORMATS ═══");
    println!("  • MIDI SMF    │ harmonia midi export <id>  /  harmonia midi render file.mid");
    println!("  • Scala .scl  │ --tuning edo31  or load a .scl via JSON \"tuning\": \"edo31\"");
    println!("  • DX7 algos   │ harmonia dx7 5   (Yamaha algorithms 1–32 as 6×6 matrices)");

    Ok(())
}

fn cmd_render(args: &[String]) -> Result<(), String> {
    if args.is_empty() {
        return Err("Usage: harmonia render <composition_id_or_json_path> [-o out.wav] [--sr 48000] [--bits 24]".to_string());
    }
    let target = &args[0];
    let mut out_path: Option<PathBuf> = None;
    let mut sr: u32 = 48000;
    let mut bit_depth = BitDepth::Pcm24;
    let mut do_play = false;

    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "-o" | "--out" if i + 1 < args.len() => {
                out_path = Some(PathBuf::from(&args[i + 1]));
                i += 2;
            }
            "--sr" | "--sample-rate" if i + 1 < args.len() => {
                sr = args[i + 1].parse::<u32>().unwrap_or(48000).clamp(22050, 192000);
                i += 2;
            }
            "--bits" | "--bit-depth" if i + 1 < args.len() => {
                bit_depth = BitDepth::from_str(&args[i + 1]);
                i += 2;
            }
            "--play" | "-p" => {
                do_play = true;
                i += 1;
            }
            _ => i += 1,
        }
    }

    let spec = if Path::new(target).exists() || target.ends_with(".json") {
        let raw = fs::read_to_string(target)
            .map_err(|e| format!("Failed to read JSON spec '{target}': {e}"))?;
        CompositionSpec::from_json_str(&raw)?
    } else if let Some(c) = find_composition(target) {
        c
    } else {
        return Err(format!(
            "Composition or JSON file '{target}' not found. Run `harmonia list` to see available IDs."
        ));
    };

    let final_out = out_path.unwrap_or_else(|| PathBuf::from(format!("renders/{}.wav", spec.id)));
    if let Some(parent) = final_out.parent() {
        if !parent.as_os_str().is_empty() {
            let _ = fs::create_dir_all(parent);
        }
    }

    let t0 = Instant::now();
    let stereo = spec.render(sr)?;
    let render_secs = t0.elapsed().as_secs_f64();
    stereo
        .save_wav(&final_out, bit_depth)
        .map_err(|e| format!("Failed to save WAV: {e}"))?;

    print_render_summary(&spec.title, &final_out, &stereo, bit_depth, render_secs);

    if do_play {
        play_wav_file(&final_out)?;
    }
    Ok(())
}

fn cmd_chord(args: &[String]) -> Result<(), String> {
    if args.is_empty() {
        return Err("Usage: harmonia chord \"Cm9 -> Abmaj9#11 -> Fm11 -> G7b9\" [options]".to_string());
    }
    let progression = args[0].clone();
    let mut patch_id = "retro_classical_brass".to_string();
    let mut pattern = MultilinePattern::CthulhuCascade;
    let mut tuning_str = "equal".to_string();
    let mut bpm = 110.0_f64;
    let mut beats = 4.0_f64;
    let mut lfo_rate: Option<f64> = None;
    let mut lfo_depth_mult: Option<f64> = None;
    let mut glide: Option<f64> = None;
    let mut cutoff: Option<f64> = None;
    let mut shimmer: Option<f64> = None;
    let mut math_formula: Option<String> = None;
    let mut sr = 48000u32;
    let mut bit_depth = BitDepth::Pcm24;
    let mut out_path: Option<PathBuf> = None;
    let mut do_play = false;

    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--patch" if i + 1 < args.len() => {
                patch_id = args[i + 1].clone();
                i += 2;
            }
            "--pattern" | "--arp" if i + 1 < args.len() => {
                pattern = MultilinePattern::from_str(&args[i + 1]);
                i += 2;
            }
            "--tuning" if i + 1 < args.len() => {
                tuning_str = args[i + 1].clone();
                i += 2;
            }
            "--bpm" if i + 1 < args.len() => {
                bpm = args[i + 1].parse::<f64>().unwrap_or(110.0);
                i += 2;
            }
            "--beats" if i + 1 < args.len() => {
                beats = args[i + 1].parse::<f64>().unwrap_or(4.0);
                i += 2;
            }
            "--lfo-rate" | "--mod-lfo" if i + 1 < args.len() => {
                lfo_rate = args[i + 1].parse::<f64>().ok();
                i += 2;
            }
            "--lfo-depth" | "--mod-depth" if i + 1 < args.len() => {
                lfo_depth_mult = args[i + 1].parse::<f64>().ok();
                i += 2;
            }
            "--glide" if i + 1 < args.len() => {
                glide = args[i + 1].parse::<f64>().ok();
                i += 2;
            }
            "--cutoff" if i + 1 < args.len() => {
                cutoff = args[i + 1].parse::<f64>().ok();
                i += 2;
            }
            "--shimmer" if i + 1 < args.len() => {
                shimmer = args[i + 1].parse::<f64>().ok();
                i += 2;
            }
            "--math" if i + 1 < args.len() => {
                math_formula = Some(args[i + 1].clone());
                i += 2;
            }
            "--sr" if i + 1 < args.len() => {
                sr = args[i + 1].parse::<u32>().unwrap_or(48000).clamp(22050, 192000);
                i += 2;
            }
            "--bits" if i + 1 < args.len() => {
                bit_depth = BitDepth::from_str(&args[i + 1]);
                i += 2;
            }
            "-o" | "--out" if i + 1 < args.len() => {
                out_path = Some(PathBuf::from(&args[i + 1]));
                i += 2;
            }
            "--play" | "-p" => {
                do_play = true;
                i += 1;
            }
            _ => i += 1,
        }
    }

    let mut patch = find_patch(&patch_id).unwrap_or_else(FmPatch::retro_classical_brass);
    if let Some(r) = lfo_rate {
        patch.lfo1.rate_hz = r.clamp(0.02, 25.0);
    }
    if let Some(d) = lfo_depth_mult {
        for r in 0..6 {
            for c in 0..6 {
                patch.lfo1_matrix_depth[r][c] *= d;
            }
        }
    }
    if let Some(g) = glide {
        patch.glide_secs = g.clamp(0.0, 2.0);
    }
    if let Some(fc) = cutoff {
        patch.filter_cutoff_hz = fc.clamp(100.0, 20000.0);
    }
    if let Some(sh) = shimmer {
        patch.reverb_shimmer = sh.clamp(0.0, 0.65);
    }
    if let Some(formula) = math_formula {
        patch.math_expr = Some(formula);
        if patch.math_mix < 0.1 {
            patch.math_mix = 0.55;
        }
    }

    let parsed_chords = harmonia::theory::parse_progression(&progression, beats);
    let root_pc = parsed_chords.first().map(|c| c.root_pc).unwrap_or(0);
    let tuning = TuningSystem::from_str(&tuning_str, root_pc);

    let spec = CompositionSpec {
        id: "custom_chord_progression".to_string(),
        title: format!("Custom Multiline Progression ({})", pattern.as_str()),
        subtitle: progression.clone(),
        description: "Synthesized via `harmonia chord`".to_string(),
        bpm,
        progression,
        beats_per_chord: beats,
        pattern,
        tuning,
        patch,
    };

    let final_out = out_path.unwrap_or_else(|| PathBuf::from("renders/custom_chords.wav"));
    if let Some(parent) = final_out.parent() {
        if !parent.as_os_str().is_empty() {
            let _ = fs::create_dir_all(parent);
        }
    }

    let t0 = Instant::now();
    let stereo = spec.render(sr)?;
    let render_secs = t0.elapsed().as_secs_f64();
    stereo
        .save_wav(&final_out, bit_depth)
        .map_err(|e| format!("Failed to save WAV: {e}"))?;

    print_render_summary(&spec.title, &final_out, &stereo, bit_depth, render_secs);

    if do_play {
        play_wav_file(&final_out)?;
    }
    Ok(())
}

fn cmd_math(args: &[String]) -> Result<(), String> {
    if args.is_empty() {
        return Err("Usage: harmonia math \"<formula_or_preset_id>\" [--freq 220] [--chords \"...\"] [-o out.wav]".to_string());
    }
    let raw_input = &args[0];
    // Check if user passed a built-in math formula preset ID
    let formula = builtin_math_formulas()
        .into_iter()
        .find(|p| p.id == raw_input.as_str())
        .map(|p| p.formula.to_string())
        .unwrap_or_else(|| raw_input.clone());

    let mut freq_hz = 220.0_f64;
    let mut chords_opt: Option<String> = None;
    let mut pattern = MultilinePattern::CthulhuCascade;
    let mut tuning_str = "just".to_string();
    let mut bpm = 108.0_f64;
    let mut duration_secs = 6.0_f64;
    let mut fm_blend = 0.22_f64;
    let mut sr = 48000u32;
    let mut bit_depth = BitDepth::Pcm24;
    let mut out_path: Option<PathBuf> = None;
    let mut do_play = false;

    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--freq" | "-f" if i + 1 < args.len() => {
                freq_hz = args[i + 1].parse::<f64>().unwrap_or(220.0).clamp(20.0, 12000.0);
                i += 2;
            }
            "--chords" | "--prog" if i + 1 < args.len() => {
                chords_opt = Some(args[i + 1].clone());
                i += 2;
            }
            "--pattern" | "--arp" if i + 1 < args.len() => {
                pattern = MultilinePattern::from_str(&args[i + 1]);
                i += 2;
            }
            "--tuning" if i + 1 < args.len() => {
                tuning_str = args[i + 1].clone();
                i += 2;
            }
            "--bpm" if i + 1 < args.len() => {
                bpm = args[i + 1].parse::<f64>().unwrap_or(108.0);
                i += 2;
            }
            "--duration" | "-d" if i + 1 < args.len() => {
                duration_secs = args[i + 1].parse::<f64>().unwrap_or(6.0).clamp(0.5, 120.0);
                i += 2;
            }
            "--fm-blend" if i + 1 < args.len() => {
                fm_blend = args[i + 1].parse::<f64>().unwrap_or(0.22).clamp(0.0, 1.0);
                i += 2;
            }
            "--sr" if i + 1 < args.len() => {
                sr = args[i + 1].parse::<u32>().unwrap_or(48000).clamp(22050, 192000);
                i += 2;
            }
            "--bits" if i + 1 < args.len() => {
                bit_depth = BitDepth::from_str(&args[i + 1]);
                i += 2;
            }
            "-o" | "--out" if i + 1 < args.len() => {
                out_path = Some(PathBuf::from(&args[i + 1]));
                i += 2;
            }
            "--play" | "-p" => {
                do_play = true;
                i += 1;
            }
            _ => i += 1,
        }
    }

    let final_out = out_path.unwrap_or_else(|| PathBuf::from("renders/math_synth.wav"));
    if let Some(parent) = final_out.parent() {
        if !parent.as_os_str().is_empty() {
            let _ = fs::create_dir_all(parent);
        }
    }

    let t0 = Instant::now();
    let stereo = if let Some(chords_str) = chords_opt {
        let mut patch = FmPatch::math_harmonic_weaver(&formula);
        patch.math_mix = (1.0 - fm_blend).clamp(0.0, 1.0);
        let parsed = harmonia::theory::parse_progression(&chords_str, 4.0);
        let root_pc = parsed.first().map(|c| c.root_pc).unwrap_or(0);
        let spec = CompositionSpec {
            id: "math_chord_synth".to_string(),
            title: format!("Symbolic Math Progression: {formula}"),
            subtitle: chords_str.clone(),
            description: "Synthesized via `harmonia math --chords`".to_string(),
            bpm,
            progression: chords_str,
            beats_per_chord: 4.0,
            pattern,
            tuning: TuningSystem::from_str(&tuning_str, root_pc),
            patch,
        };
        spec.render(sr)?
    } else {
        // Pure symbolic mathematical synthesis over `duration_secs` at `freq_hz`
        let compiled = CompiledExpr::compile(&formula)?;
        let num_frames = (duration_secs * sr as f64).ceil() as usize;
        let mut buf = StereoBuffer::new(sr, num_frames);
        let env_spec = AdsrSpec {
            delay_secs: 0.0,
            attack_secs: 0.08,
            hold_secs: 0.1,
            decay_secs: 1.2,
            sustain_level: 0.82,
            release_secs: 1.1,
            curve: 1.3,
        };
        let note_on_dur = (duration_secs - 1.1).max(0.2);

        for frame in 0..num_frames {
            let t = frame as f64 / sr as f64;
            let env = env_spec.eval(t, note_on_dur);
            let mod_idx = 2.2 + 1.5 * (std::f64::consts::TAU * 0.28 * t).sin();
            let ctx_l = MathContext::for_sample(t, t, freq_hz, 0.9, env, 0, mod_idx);
            let ctx_r = MathContext::for_sample(t, t + 0.004, freq_hz * 1.0018, 0.9, env, 1, mod_idx);
            buf.left[frame] = compiled.eval(&ctx_l).clamp(-2.0, 2.0) * 0.6;
            buf.right[frame] = compiled.eval(&ctx_r).clamp(-2.0, 2.0) * 0.6;
        }
        let patch = FmPatch::math_harmonic_weaver(&formula);
        process_master_chain(&mut buf, &patch);
        buf
    };

    let render_secs = t0.elapsed().as_secs_f64();
    stereo
        .save_wav(&final_out, bit_depth)
        .map_err(|e| format!("Failed to save WAV: {e}"))?;

    print_render_summary(
        &format!("Math Synth: {formula}"),
        &final_out,
        &stereo,
        bit_depth,
        render_secs,
    );

    if do_play {
        play_wav_file(&final_out)?;
    }
    Ok(())
}

fn cmd_inspect(args: &[String]) -> Result<(), String> {
    if args.is_empty() {
        return Err("Usage: harmonia inspect <path.wav>".to_string());
    }
    let path = Path::new(&args[0]);
    let (fmt_desc, rep) = inspect_wav_file(path)?;
    println!("╔══════════════════════════════════════════════════════════════════════════╗");
    println!("║  HARMONIA WAV TELEMETRY INSPECTOR                                        ║");
    println!("╠══════════════════════════════════════════════════════════════════════════╣");
    println!("  File             : {}", path.display());
    println!("  Format           : {}", fmt_desc);
    println!(
        "  Duration         : {:.2} sec ({} frames)",
        rep.duration_secs, rep.num_frames
    );
    println!(
        "  Peak Level       : {:.2} dBFS (linear {:.4})",
        rep.peak_dbfs, rep.peak_linear
    );
    println!(
        "  RMS Level        : {:.2} dBFS (linear {:.4})",
        rep.rms_dbfs, rep.rms_linear
    );
    println!("  Crest Factor     : {:.2} dB", rep.crest_factor_db);
    println!(
        "  Est. ZC Pitch    : {:.1} Hz",
        rep.zero_crossing_hz_est
    );
    println!(
        "  Stereo Width     : {:.3} L/R correlation",
        rep.stereo_width_corr
    );
    println!("  Envelope         : [{}]", rep.ascii_envelope);
    println!("╚══════════════════════════════════════════════════════════════════════════╝");
    Ok(())
}

fn cmd_play(args: &[String]) -> Result<(), String> {
    if args.is_empty() {
        return Err("Usage: harmonia play <path.wav | composition_id>".to_string());
    }
    let target = &args[0];
    let path = Path::new(target);
    if path.exists() {
        return play_wav_file(path);
    }
    // Otherwise render composition to temporary WAV and play
    let out = PathBuf::from(format!("renders/{target}.wav"));
    cmd_render(&[
        target.clone(),
        "-o".to_string(),
        out.to_string_lossy().to_string(),
        "--play".to_string(),
    ])
}

fn cmd_export_presets(args: &[String]) -> Result<(), String> {
    let dir = args
        .first()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("presets"));
    fs::create_dir_all(&dir).map_err(|e| format!("Failed to create dir: {e}"))?;

    for comp in builtin_compositions() {
        let p = dir.join(format!("{}.json", comp.id));
        fs::write(&p, comp.to_pretty_json())
            .map_err(|e| format!("Failed to write {}: {e}", p.display()))?;
        println!("Exported canonical composition spec -> {}", p.display());
    }
    Ok(())
}

fn cmd_serve(args: &[String]) -> Result<(), String> {
    let mut port = 8765u16;
    let mut i = 0;
    while i < args.len() {
        if (args[i] == "--port" || args[i] == "-p") && i + 1 < args.len() {
            port = args[i + 1].parse::<u16>().unwrap_or(8765);
            i += 2;
        } else {
            i += 1;
        }
    }
    let project_dir = env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    run_studio_server(port, project_dir)
}

fn print_render_summary(
    title: &str,
    out_path: &Path,
    stereo: &StereoBuffer,
    bit_depth: BitDepth,
    render_secs: f64,
) {
    let rep = stereo.analyze();
    let speedup = rep.duration_secs / render_secs.max(1e-6);
    println!("╔══════════════════════════════════════════════════════════════════════════╗");
    println!("║  HARMONIA HI-DEF RENDER COMPLETE                                         ║");
    println!("╠══════════════════════════════════════════════════════════════════════════╣");
    println!("  Title            : {title}");
    println!("  Output File      : {}", out_path.display());
    println!(
        "  Audio Format     : {} Hz · {} · Stereo",
        rep.sample_rate,
        bit_depth.label()
    );
    println!(
        "  Duration & Speed : {:.2}s audio rendered in {:.1}ms ({:.1}x real-time)",
        rep.duration_secs,
        render_secs * 1000.0,
        speedup
    );
    println!(
        "  Signal Dynamics  : Peak {:.2} dBFS │ RMS {:.2} dBFS │ Crest {:.1} dB",
        rep.peak_dbfs, rep.rms_dbfs, rep.crest_factor_db
    );
    println!(
        "  Stereo & Pitch   : L/R Corr {:.3} │ Est. Spectral ZC {:.1} Hz",
        rep.stereo_width_corr, rep.zero_crossing_hz_est
    );
    println!("  Amplitude Trace  : [{}]", rep.ascii_envelope);
    println!("╚══════════════════════════════════════════════════════════════════════════╝");
}

fn play_wav_file(path: &Path) -> Result<(), String> {
    let players = [
        ("pw-play", vec![path.to_string_lossy().to_string()]),
        ("paplay", vec![path.to_string_lossy().to_string()]),
        ("aplay", vec!["-q".to_string(), path.to_string_lossy().to_string()]),
        (
            "ffplay",
            vec![
                "-nodisp".to_string(),
                "-autoexit".to_string(),
                "-loglevel".to_string(),
                "quiet".to_string(),
                path.to_string_lossy().to_string(),
            ],
        ),
    ];

    for (bin, p_args) in players {
        if let Ok(mut child) = Command::new(bin).args(&p_args).spawn() {
            println!("▶ Playing {} via `{bin}`...", path.display());
            let _ = child.wait();
            return Ok(());
        }
    }

    Err("No system audio player (pw-play, paplay, aplay, ffplay) could be started".to_string())
}

fn cmd_midi(args: &[String]) -> Result<(), String> {
    if args.is_empty() {
        return Err("Usage: harmonia midi export <composition_id> -o out.mid  |  harmonia midi render <file.mid> [-o out.wav]".into());
    }
    match args[0].as_str() {
        "export" => {
            if args.len() < 2 {
                return Err("Usage: harmonia midi export <composition_id> -o out.mid".into());
            }
            let id = &args[1];
            let spec = find_composition(id).ok_or_else(|| format!("Unknown composition '{id}'"))?;
            let mut out = PathBuf::from(format!("renders/{}.mid", spec.id));
            let mut i = 2;
            while i < args.len() {
                if (args[i] == "-o" || args[i] == "--out") && i + 1 < args.len() {
                    out = PathBuf::from(&args[i + 1]);
                    i += 2;
                } else {
                    i += 1;
                }
            }
            let chords = harmonia::theory::parse_progression(&spec.progression, spec.beats_per_chord);
            let (events, _) =
                harmonia::theory::generate_multiline_events(&chords, spec.bpm, spec.pattern.clone(), spec.tuning.clone());
            let midi = harmonia::midi::MidiFile::from_note_events(&events, spec.bpm, |ev| {
                harmonia::midi::hz_to_midi(ev.target_freq_hz)
            });
            if let Some(parent) = out.parent() {
                if !parent.as_os_str().is_empty() {
                    let _ = fs::create_dir_all(parent);
                }
            }
            harmonia::midi::save_smf(&out, &midi)?;
            println!("Wrote MIDI ({} notes, {:.1} BPM) → {}", midi.notes.len(), midi.bpm(), out.display());
            Ok(())
        }
        "render" => {
            if args.len() < 2 {
                return Err("Usage: harmonia midi render <file.mid> [-o out.wav] [--play]".into());
            }
            let path = Path::new(&args[1]);
            let midi = harmonia::midi::load_smf(path)?;
            let mut out = PathBuf::from("renders/midi_render.wav");
            let mut do_play = false;
            let mut i = 2;
            while i < args.len() {
                if (args[i] == "-o" || args[i] == "--out") && i + 1 < args.len() {
                    out = PathBuf::from(&args[i + 1]);
                    i += 2;
                } else if args[i] == "--play" {
                    do_play = true;
                    i += 1;
                } else {
                    i += 1;
                }
            }
            let events = midi.to_note_events(|n| {
                440.0 * (2.0_f64).powf((n as f64 - 69.0) / 12.0)
            });
            let mut spec = find_composition("glass_cathedral_ji")
                .or_else(|| builtin_compositions().into_iter().next())
                .unwrap();
            spec.id = "midi_render".into();
            spec.title = format!("MIDI render: {}", path.display());
            let sr = 48000u32;
            let tail = 1.8;
            let last = events
                .iter()
                .map(|e| e.start_sec + e.duration_sec)
                .fold(1.0_f64, f64::max);
            let frames = ((last + tail) * sr as f64).ceil() as usize;
            let mut buf = StereoBuffer::new(sr, frames);
            for ev in &events {
                let start = (ev.start_sec * sr as f64).round() as usize;
                spec.patch.render_note_into(
                    &mut buf.left,
                    &mut buf.right,
                    sr,
                    start,
                    ev.duration_sec,
                    ev.start_freq_hz,
                    ev.target_freq_hz,
                    ev.velocity,
                    ev.voice_line,
                    None,
                );
            }
            process_master_chain(&mut buf, &spec.patch);
            if let Some(parent) = out.parent() {
                if !parent.as_os_str().is_empty() {
                    let _ = fs::create_dir_all(parent);
                }
            }
            buf.save_wav(&out, BitDepth::Pcm24)
                .map_err(|e| format!("WAV write failed: {e}"))?;
            print_render_summary(&spec.title, &out, &buf, BitDepth::Pcm24, 0.0);
            if do_play {
                play_wav_file(&out)?;
            }
            Ok(())
        }
        other => Err(format!("Unknown midi subcommand '{other}' (use export|render)")),
    }
}

fn cmd_dx7(args: &[String]) -> Result<(), String> {
    let n = args
        .first()
        .and_then(|s| s.parse::<u8>().ok())
        .unwrap_or(5)
        .clamp(1, 32);
    let algo = harmonia::dx7::algorithm(n);
    println!("DX7 Algorithm {} — {}", algo.number, algo.name);
    println!("Carriers: {:?}", algo.carriers);
    println!("Feedback operator: {}", algo.feedback_op + 1);
    print!("Matrix src↓ dst→  ");
    for c in 0..6 {
        print!("  {}", c + 1);
    }
    println!();
    for r in 0..6 {
        print!("             {:2}  ", r + 1);
        for c in 0..6 {
            print!(" {}", if algo.matrix[r][c] > 0.0 { "●" } else { "·" });
        }
        println!();
    }

    let mut spec = find_composition("glass_cathedral_ji")
        .or_else(|| builtin_compositions().into_iter().next())
        .unwrap();
    harmonia::dx7::apply_to_patch(&mut spec.patch, n, 1.9);
    spec.id = format!("dx7_alg_{n}");
    spec.title = format!("DX7 Algorithm {n} — {}", algo.name);
    spec.beats_per_chord = 2.0;
    spec.progression = "Cm9 -> Abmaj9#11 -> Fm11 -> G7b9".into();
    let out = PathBuf::from(format!("renders/dx7_alg_{n}.wav"));
    let t0 = Instant::now();
    let stereo = spec.render(48000)?;
    stereo
        .save_wav(&out, BitDepth::Pcm24)
        .map_err(|e| format!("WAV write failed: {e}"))?;
    print_render_summary(&spec.title, &out, &stereo, BitDepth::Pcm24, t0.elapsed().as_secs_f64());
    Ok(())
}
