//! Zero-dependency JSON serializer & parser for `CompositionSpec` and `FmPatch`.
//! Keeps canonical presets human-readable, diffable in Git, and editable by humans or agents.

use crate::fm_synth::FmPatch;
use crate::oscillator::{LfoShape, Waveform};
use crate::presets::CompositionSpec;
use crate::theory::{MultilinePattern, TuningSystem};
use std::collections::BTreeMap;

#[derive(Debug, Clone)]
pub enum JsonVal {
    Null,
    Bool(bool),
    Num(f64),
    Str(String),
    Arr(Vec<JsonVal>),
    Obj(BTreeMap<String, JsonVal>),
}

impl JsonVal {
    pub fn as_f64(&self) -> Option<f64> {
        match self {
            JsonVal::Num(n) => Some(*n),
            _ => None,
        }
    }

    pub fn as_bool(&self) -> Option<bool> {
        match self {
            JsonVal::Bool(b) => Some(*b),
            _ => None,
        }
    }

    pub fn as_str(&self) -> Option<&str> {
        match self {
            JsonVal::Str(s) => Some(s.as_str()),
            _ => None,
        }
    }

    pub fn as_arr(&self) -> Option<&[JsonVal]> {
        match self {
            JsonVal::Arr(a) => Some(a.as_slice()),
            _ => None,
        }
    }

    pub fn as_obj(&self) -> Option<&BTreeMap<String, JsonVal>> {
        match self {
            JsonVal::Obj(o) => Some(o),
            _ => None,
        }
    }

    pub fn get(&self, key: &str) -> Option<&JsonVal> {
        self.as_obj()?.get(key)
    }
}

pub fn escape_json(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 8);
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c => out.push(c),
        }
    }
    out
}

impl CompositionSpec {
    pub fn to_pretty_json(&self) -> String {
        let mut s = String::with_capacity(4096);
        s.push_str("{\n");
        s.push_str(&format!("  \"id\": \"{}\",\n", escape_json(&self.id)));
        s.push_str(&format!("  \"title\": \"{}\",\n", escape_json(&self.title)));
        s.push_str(&format!(
            "  \"subtitle\": \"{}\",\n",
            escape_json(&self.subtitle)
        ));
        s.push_str(&format!(
            "  \"description\": \"{}\",\n",
            escape_json(&self.description)
        ));
        s.push_str(&format!("  \"bpm\": {:.2},\n", self.bpm));
        s.push_str(&format!(
            "  \"progression\": \"{}\",\n",
            escape_json(&self.progression)
        ));
        s.push_str(&format!("  \"beats_per_chord\": {:.2},\n", self.beats_per_chord));
        s.push_str(&format!("  \"pattern\": \"{}\",\n", self.pattern.as_str()));

        let (tuning_name, root_pc, a4_hz) = match &self.tuning {
            TuningSystem::Equal12 { a4_hz } => ("equal12".to_string(), 0, *a4_hz),
            TuningSystem::JustIntonation { root_pc, a4_hz } => {
                ("just_intonation".to_string(), *root_pc, *a4_hz)
            }
            TuningSystem::Pythagorean { root_pc, a4_hz } => {
                ("pythagorean".to_string(), *root_pc, *a4_hz)
            }
            TuningSystem::HarmonicSeries { base_hz } => {
                ("harmonic_series".to_string(), 0, *base_hz)
            }
            TuningSystem::GoldenRatio { a4_hz } => ("golden_ratio".to_string(), 0, *a4_hz),
            TuningSystem::Scala { scale, a4_hz, .. } => {
                (format!("scala:{}", scale.description), 0, *a4_hz)
            }
        };
        s.push_str(&format!("  \"tuning\": \"{tuning_name}\",\n"));
        s.push_str(&format!("  \"tuning_root_pc\": {root_pc},\n"));
        s.push_str(&format!("  \"tuning_a4_hz\": {a4_hz:.2},\n"));
        s.push_str("  \"patch\": ");
        s.push_str(&self.patch.to_pretty_json(2));
        s.push_str("\n}\n");
        s
    }

    pub fn from_json_str(raw: &str) -> Result<Self, String> {
        let val = parse_json(raw)?;
        Self::from_json_val(&val)
    }

    pub fn from_json_val(val: &JsonVal) -> Result<Self, String> {
        let mut spec = crate::presets::builtin_compositions()
            .into_iter()
            .next()
            .unwrap();

        if let Some(base_id) = val.get("base_composition").and_then(|v| v.as_str()) {
            if let Some(found) = crate::presets::find_composition(base_id) {
                spec = found;
            }
        }
        if let Some(patch_id) = val.get("patch_id").and_then(|v| v.as_str()) {
            if let Some(p) = crate::presets::find_patch(patch_id) {
                spec.patch = p;
            }
        }

        if let Some(v) = val.get("id").and_then(|v| v.as_str()) {
            spec.id = v.to_string();
        }
        if let Some(v) = val.get("title").and_then(|v| v.as_str()) {
            spec.title = v.to_string();
        }
        if let Some(v) = val.get("subtitle").and_then(|v| v.as_str()) {
            spec.subtitle = v.to_string();
        }
        if let Some(v) = val.get("description").and_then(|v| v.as_str()) {
            spec.description = v.to_string();
        }
        if let Some(v) = val.get("bpm").and_then(|v| v.as_f64()) {
            spec.bpm = v.clamp(30.0, 260.0);
        }
        if let Some(v) = val.get("progression").and_then(|v| v.as_str()) {
            spec.progression = v.to_string();
        }
        if let Some(v) = val.get("beats_per_chord").and_then(|v| v.as_f64()) {
            spec.beats_per_chord = v.clamp(0.5, 32.0);
        }
        if let Some(v) = val.get("pattern").and_then(|v| v.as_str()) {
            spec.pattern = MultilinePattern::from_str(v);
        }

        let root_pc = val
            .get("tuning_root_pc")
            .and_then(|v| v.as_f64())
            .map(|n| n as u8)
            .unwrap_or(0);
        let a4_hz = val
            .get("tuning_a4_hz")
            .and_then(|v| v.as_f64())
            .unwrap_or(440.0);
        if let Some(t_str) = val.get("tuning").and_then(|v| v.as_str()) {
            spec.tuning = match t_str.to_ascii_lowercase().as_str() {
                "just" | "ji" | "just_intonation" => {
                    TuningSystem::JustIntonation { root_pc, a4_hz }
                }
                "pyth" | "pythagorean" => TuningSystem::Pythagorean { root_pc, a4_hz },
                "harm" | "harmonic" | "harmonic_series" => TuningSystem::HarmonicSeries {
                    base_hz: a4_hz.min(110.0),
                },
                "phi" | "golden" | "golden_ratio" => TuningSystem::GoldenRatio { a4_hz },
                other => TuningSystem::from_str(other, root_pc),
            };
        }

        if let Some(algo) = val.get("dx7_algorithm").and_then(|v| v.as_f64()) {
            crate::dx7::apply_to_patch(&mut spec.patch, algo as u8, 1.8);
        }

        if let Some(patch_obj) = val.get("patch") {
            spec.patch.apply_json_overrides(patch_obj);
        }
        // Also allow top-level shorthand overrides (great for HTTP API & agent calls)
        spec.patch.apply_json_overrides(val);

        Ok(spec)
    }
}

impl FmPatch {
    pub fn to_pretty_json(&self, indent: usize) -> String {
        let pad = " ".repeat(indent);
        let pad2 = " ".repeat(indent + 2);
        let pad3 = " ".repeat(indent + 4);
        let mut s = String::with_capacity(3072);
        s.push_str("{\n");
        s.push_str(&format!("{pad2}\"id\": \"{}\",\n", escape_json(&self.id)));
        s.push_str(&format!(
            "{pad2}\"name\": \"{}\",\n",
            escape_json(&self.name)
        ));
        s.push_str(&format!(
            "{pad2}\"category\": \"{}\",\n",
            escape_json(&self.category)
        ));
        s.push_str(&format!(
            "{pad2}\"algorithm_name\": \"{}\",\n",
            escape_json(&self.algorithm_name)
        ));
        s.push_str(&format!("{pad2}\"vibrato_cents\": {:.2},\n", self.vibrato_cents));
        s.push_str(&format!("{pad2}\"glide_secs\": {:.4},\n", self.glide_secs));
        s.push_str(&format!("{pad2}\"unison_voices\": {},\n", self.unison_voices));
        s.push_str(&format!(
            "{pad2}\"unison_detune_cents\": {:.2},\n",
            self.unison_detune_cents
        ));
        match &self.math_expr {
            Some(expr) => s.push_str(&format!(
                "{pad2}\"math_expr\": \"{}\",\n",
                escape_json(expr)
            )),
            None => s.push_str(&format!("{pad2}\"math_expr\": null,\n")),
        }
        s.push_str(&format!("{pad2}\"math_mix\": {:.3},\n", self.math_mix));
        s.push_str(&format!(
            "{pad2}\"lfo1_shape\": \"{}\",\n",
            self.lfo1.shape.as_str()
        ));
        s.push_str(&format!("{pad2}\"lfo1_rate_hz\": {:.3},\n", self.lfo1.rate_hz));
        s.push_str(&format!("{pad2}\"lfo1_depth\": {:.3},\n", self.lfo1.depth));
        s.push_str(&format!(
            "{pad2}\"filter_cutoff_hz\": {:.1},\n",
            self.filter_cutoff_hz
        ));
        s.push_str(&format!(
            "{pad2}\"filter_resonance\": {:.3},\n",
            self.filter_resonance
        ));
        s.push_str(&format!(
            "{pad2}\"filter_lfo_depth_hz\": {:.1},\n",
            self.filter_lfo_depth_hz
        ));
        s.push_str(&format!("{pad2}\"chorus_mix\": {:.3},\n", self.chorus_mix));
        s.push_str(&format!("{pad2}\"delay_mix\": {:.3},\n", self.delay_mix));
        s.push_str(&format!(
            "{pad2}\"delay_time_secs\": {:.3},\n",
            self.delay_time_secs
        ));
        s.push_str(&format!(
            "{pad2}\"delay_feedback\": {:.3},\n",
            self.delay_feedback
        ));
        s.push_str(&format!("{pad2}\"reverb_mix\": {:.3},\n", self.reverb_mix));
        s.push_str(&format!(
            "{pad2}\"reverb_room_size\": {:.3},\n",
            self.reverb_room_size
        ));
        s.push_str(&format!(
            "{pad2}\"reverb_shimmer\": {:.3},\n",
            self.reverb_shimmer
        ));
        s.push_str(&format!("{pad2}\"master_drive\": {:.3},\n", self.master_drive));

        // 6x6 Mod Matrix
        s.push_str(&format!("{pad2}\"mod_matrix\": [\n"));
        for r in 0..6 {
            let row_str: Vec<String> = self.mod_matrix[r].iter().map(|v| format!("{v:.3}")).collect();
            let comma = if r < 5 { "," } else { "" };
            s.push_str(&format!("{pad3}[{}]{comma}\n", row_str.join(", ")));
        }
        s.push_str(&format!("{pad2}],\n"));

        // 6x6 LFO1 Matrix Depth
        s.push_str(&format!("{pad2}\"lfo1_matrix_depth\": [\n"));
        for r in 0..6 {
            let row_str: Vec<String> = self.lfo1_matrix_depth[r]
                .iter()
                .map(|v| format!("{v:.3}"))
                .collect();
            let comma = if r < 5 { "," } else { "" };
            s.push_str(&format!("{pad3}[{}]{comma}\n", row_str.join(", ")));
        }
        s.push_str(&format!("{pad2}],\n"));

        // 6 Operators
        s.push_str(&format!("{pad2}\"operators\": [\n"));
        for (i, op) in self.operators.iter().enumerate() {
            let comma = if i < 5 { "," } else { "" };
            s.push_str(&format!(
                "{pad3}{{\"index\": {}, \"enabled\": {}, \"ratio\": {:.4}, \"detune_cents\": {:.2}, \"waveform\": \"{}\", \"output_level\": {:.3}, \"pan\": {:.2}, \"feedback\": {:.3}, \"attack_secs\": {:.4}, \"decay_secs\": {:.4}, \"sustain_level\": {:.3}, \"release_secs\": {:.4}}}{comma}\n",
                op.index,
                op.enabled,
                op.ratio,
                op.detune_cents,
                op.waveform.as_str(),
                op.output_level,
                op.pan,
                op.feedback,
                op.env.attack_secs,
                op.env.decay_secs,
                op.env.sustain_level,
                op.env.release_secs
            ));
        }
        s.push_str(&format!("{pad2}]\n"));
        s.push_str(&format!("{pad}}}"));
        s
    }

    pub fn apply_json_overrides(&mut self, val: &JsonVal) {
        if let Some(v) = val.get("vibrato_cents").and_then(|v| v.as_f64()) {
            self.vibrato_cents = v;
        }
        if let Some(v) = val.get("glide_secs").and_then(|v| v.as_f64()) {
            self.glide_secs = v.clamp(0.0, 2.0);
        }
        if let Some(v) = val.get("unison_voices").and_then(|v| v.as_f64()) {
            self.unison_voices = (v as usize).clamp(1, 4);
        }
        if let Some(v) = val.get("unison_detune_cents").and_then(|v| v.as_f64()) {
            self.unison_detune_cents = v.clamp(0.0, 50.0);
        }
        if let Some(v) = val.get("math_expr") {
            match v {
                JsonVal::Str(s) if !s.trim().is_empty() => self.math_expr = Some(s.clone()),
                JsonVal::Null => self.math_expr = None,
                _ => {}
            }
        }
        if let Some(v) = val.get("math_mix").and_then(|v| v.as_f64()) {
            self.math_mix = v.clamp(0.0, 1.0);
        }
        if let Some(v) = val.get("lfo1_shape").and_then(|v| v.as_str()) {
            self.lfo1.shape = LfoShape::from_str(v);
        }
        if let Some(v) = val.get("lfo1_rate_hz").and_then(|v| v.as_f64()) {
            self.lfo1.rate_hz = v.clamp(0.01, 30.0);
        }
        if let Some(v) = val.get("lfo1_depth").and_then(|v| v.as_f64()) {
            self.lfo1.depth = v.clamp(0.0, 10.0);
        }
        if let Some(v) = val.get("filter_cutoff_hz").and_then(|v| v.as_f64()) {
            self.filter_cutoff_hz = v.clamp(80.0, 22000.0);
        }
        if let Some(v) = val.get("filter_resonance").and_then(|v| v.as_f64()) {
            self.filter_resonance = v.clamp(0.0, 0.92);
        }
        if let Some(v) = val.get("filter_lfo_depth_hz").and_then(|v| v.as_f64()) {
            self.filter_lfo_depth_hz = v.clamp(0.0, 12000.0);
        }
        if let Some(v) = val.get("chorus_mix").and_then(|v| v.as_f64()) {
            self.chorus_mix = v.clamp(0.0, 0.85);
        }
        if let Some(v) = val.get("delay_mix").and_then(|v| v.as_f64()) {
            self.delay_mix = v.clamp(0.0, 0.85);
        }
        if let Some(v) = val.get("delay_time_secs").and_then(|v| v.as_f64()) {
            self.delay_time_secs = v.clamp(0.05, 1.5);
        }
        if let Some(v) = val.get("delay_feedback").and_then(|v| v.as_f64()) {
            self.delay_feedback = v.clamp(0.0, 0.85);
        }
        if let Some(v) = val.get("reverb_mix").and_then(|v| v.as_f64()) {
            self.reverb_mix = v.clamp(0.0, 0.90);
        }
        if let Some(v) = val.get("reverb_room_size").and_then(|v| v.as_f64()) {
            self.reverb_room_size = v.clamp(0.1, 0.98);
        }
        if let Some(v) = val.get("reverb_shimmer").and_then(|v| v.as_f64()) {
            self.reverb_shimmer = v.clamp(0.0, 0.65);
        }
        if let Some(v) = val.get("master_drive").and_then(|v| v.as_f64()) {
            self.master_drive = v.clamp(0.5, 3.0);
        }

        if let Some(rows) = val.get("mod_matrix").and_then(|v| v.as_arr()) {
            for (r, row_val) in rows.iter().take(6).enumerate() {
                if let Some(cols) = row_val.as_arr() {
                    for (c, cell) in cols.iter().take(6).enumerate() {
                        if let Some(num) = cell.as_f64() {
                            self.mod_matrix[r][c] = num.clamp(0.0, 12.0);
                        }
                    }
                }
            }
        }

        if let Some(rows) = val.get("lfo1_matrix_depth").and_then(|v| v.as_arr()) {
            for (r, row_val) in rows.iter().take(6).enumerate() {
                if let Some(cols) = row_val.as_arr() {
                    for (c, cell) in cols.iter().take(6).enumerate() {
                        if let Some(num) = cell.as_f64() {
                            self.lfo1_matrix_depth[r][c] = num.clamp(0.0, 10.0);
                        }
                    }
                }
            }
        }

        if let Some(ops_arr) = val.get("operators").and_then(|v| v.as_arr()) {
            for (i, op_val) in ops_arr.iter().take(6).enumerate() {
                let op = &mut self.operators[i];
                if let Some(b) = op_val.get("enabled").and_then(|v| v.as_bool()) {
                    op.enabled = b;
                }
                if let Some(r) = op_val.get("ratio").and_then(|v| v.as_f64()) {
                    op.ratio = r.clamp(0.125, 32.0);
                }
                if let Some(d) = op_val.get("detune_cents").and_then(|v| v.as_f64()) {
                    op.detune_cents = d.clamp(-200.0, 200.0);
                }
                if let Some(w) = op_val.get("waveform").and_then(|v| v.as_str()) {
                    op.waveform = Waveform::from_str(w);
                }
                if let Some(lvl) = op_val.get("output_level").and_then(|v| v.as_f64()) {
                    op.output_level = lvl.clamp(0.0, 1.5);
                }
                if let Some(pan) = op_val.get("pan").and_then(|v| v.as_f64()) {
                    op.pan = pan.clamp(-1.0, 1.0);
                }
                if let Some(fb) = op_val.get("feedback").and_then(|v| v.as_f64()) {
                    op.feedback = fb.clamp(0.0, 3.0);
                }
                if let Some(a) = op_val.get("attack_secs").and_then(|v| v.as_f64()) {
                    op.env.attack_secs = a.clamp(0.001, 10.0);
                }
                if let Some(d) = op_val.get("decay_secs").and_then(|v| v.as_f64()) {
                    op.env.decay_secs = d.clamp(0.005, 15.0);
                }
                if let Some(s) = op_val.get("sustain_level").and_then(|v| v.as_f64()) {
                    op.env.sustain_level = s.clamp(0.0, 1.0);
                }
                if let Some(r) = op_val.get("release_secs").and_then(|v| v.as_f64()) {
                    op.env.release_secs = r.clamp(0.01, 15.0);
                }
            }
        }
    }
}

/// Minimal recursive-descent JSON parser.
pub fn parse_json(input: &str) -> Result<JsonVal, String> {
    let chars: Vec<char> = input.chars().collect();
    let mut pos = 0usize;
    let val = parse_value(&chars, &mut pos)?;
    skip_ws(&chars, &mut pos);
    Ok(val)
}

fn skip_ws(chars: &[char], pos: &mut usize) {
    while *pos < chars.len() && chars[*pos].is_whitespace() {
        *pos += 1;
    }
}

fn parse_value(chars: &[char], pos: &mut usize) -> Result<JsonVal, String> {
    skip_ws(chars, pos);
    if *pos >= chars.len() {
        return Err("Unexpected EOF in JSON".to_string());
    }
    match chars[*pos] {
        '{' => parse_object(chars, pos),
        '[' => parse_array(chars, pos),
        '"' => parse_string(chars, pos).map(JsonVal::Str),
        't' | 'f' => parse_bool(chars, pos),
        'n' => parse_null(chars, pos),
        c if c.is_ascii_digit() || c == '-' => parse_number(chars, pos),
        other => Err(format!("Unexpected char '{other}' at pos {pos}")),
    }
}

fn parse_object(chars: &[char], pos: &mut usize) -> Result<JsonVal, String> {
    *pos += 1; // skip '{'
    skip_ws(chars, pos);
    let mut map = BTreeMap::new();
    if *pos < chars.len() && chars[*pos] == '}' {
        *pos += 1;
        return Ok(JsonVal::Obj(map));
    }
    loop {
        skip_ws(chars, pos);
        let key = parse_string(chars, pos)?;
        skip_ws(chars, pos);
        if *pos >= chars.len() || chars[*pos] != ':' {
            return Err("Expected ':' in JSON object".to_string());
        }
        *pos += 1;
        let val = parse_value(chars, pos)?;
        map.insert(key, val);
        skip_ws(chars, pos);
        if *pos >= chars.len() {
            return Err("Unterminated JSON object".to_string());
        }
        if chars[*pos] == ',' {
            *pos += 1;
        } else if chars[*pos] == '}' {
            *pos += 1;
            break;
        } else {
            return Err(format!("Expected ',' or '}}' at pos {pos}"));
        }
    }
    Ok(JsonVal::Obj(map))
}

fn parse_array(chars: &[char], pos: &mut usize) -> Result<JsonVal, String> {
    *pos += 1; // skip '['
    skip_ws(chars, pos);
    let mut arr = Vec::new();
    if *pos < chars.len() && chars[*pos] == ']' {
        *pos += 1;
        return Ok(JsonVal::Arr(arr));
    }
    loop {
        let val = parse_value(chars, pos)?;
        arr.push(val);
        skip_ws(chars, pos);
        if *pos >= chars.len() {
            return Err("Unterminated JSON array".to_string());
        }
        if chars[*pos] == ',' {
            *pos += 1;
        } else if chars[*pos] == ']' {
            *pos += 1;
            break;
        } else {
            return Err(format!("Expected ',' or ']' at pos {pos}"));
        }
    }
    Ok(JsonVal::Arr(arr))
}

fn parse_string(chars: &[char], pos: &mut usize) -> Result<String, String> {
    skip_ws(chars, pos);
    if *pos >= chars.len() || chars[*pos] != '"' {
        return Err(format!("Expected '\"' at pos {pos}"));
    }
    *pos += 1;
    let mut out = String::new();
    while *pos < chars.len() {
        let c = chars[*pos];
        *pos += 1;
        if c == '"' {
            return Ok(out);
        }
        if c == '\\' {
            if *pos >= chars.len() {
                return Err("Unterminated escape in JSON string".to_string());
            }
            let esc = chars[*pos];
            *pos += 1;
            match esc {
                '"' => out.push('"'),
                '\\' => out.push('\\'),
                '/' => out.push('/'),
                'n' => out.push('\n'),
                'r' => out.push('\r'),
                't' => out.push('\t'),
                other => out.push(other),
            }
        } else {
            out.push(c);
        }
    }
    Err("Unterminated JSON string".to_string())
}

fn parse_bool(chars: &[char], pos: &mut usize) -> Result<JsonVal, String> {
    if chars[*pos..].starts_with(&['t', 'r', 'u', 'e']) {
        *pos += 4;
        Ok(JsonVal::Bool(true))
    } else if chars[*pos..].starts_with(&['f', 'a', 'l', 's', 'e']) {
        *pos += 5;
        Ok(JsonVal::Bool(false))
    } else {
        Err(format!("Invalid boolean at pos {pos}"))
    }
}

fn parse_null(chars: &[char], pos: &mut usize) -> Result<JsonVal, String> {
    if chars[*pos..].starts_with(&['n', 'u', 'l', 'l']) {
        *pos += 4;
        Ok(JsonVal::Null)
    } else {
        Err(format!("Invalid null at pos {pos}"))
    }
}

fn parse_number(chars: &[char], pos: &mut usize) -> Result<JsonVal, String> {
    let start = *pos;
    if chars[*pos] == '-' {
        *pos += 1;
    }
    while *pos < chars.len()
        && (chars[*pos].is_ascii_digit()
            || chars[*pos] == '.'
            || chars[*pos] == 'e'
            || chars[*pos] == 'E'
            || chars[*pos] == '+'
            || chars[*pos] == '-')
    {
        *pos += 1;
    }
    let s: String = chars[start..*pos].iter().collect();
    let num = s
        .parse::<f64>()
        .map_err(|_| format!("Invalid JSON number '{s}'"))?;
    Ok(JsonVal::Num(num))
}
