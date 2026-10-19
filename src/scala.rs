//! Scala `.scl` / `.kbm` microtonal tunings (Ana Mark / Huygens-Fokker open format).
//! Also ships well-known equal temperaments and historical scales as first-class modules.

use std::fs;
use std::path::Path;

#[derive(Debug, Clone)]
pub struct ScalaScale {
    pub description: String,
    /// Degree 0 is always 1.0 (unison). Remaining entries are frequency ratios
    /// relative to the scale tonic, including the octave/period as the last entry.
    pub ratios: Vec<f64>,
}

impl ScalaScale {
    pub fn period_ratio(&self) -> f64 {
        self.ratios.last().copied().unwrap_or(2.0).max(1.0001)
    }

    pub fn degrees(&self) -> usize {
        self.ratios.len().saturating_sub(1).max(1)
    }

    /// Map a MIDI-like integer degree (relative to tonic MIDI) onto this scale.
    pub fn degree_ratio(&self, degree: i32) -> f64 {
        let n = self.degrees() as i32;
        if n <= 0 {
            return 1.0;
        }
        let period = self.period_ratio();
        let octaves = degree.div_euclid(n);
        let step = degree.rem_euclid(n) as usize;
        period.powi(octaves) * self.ratios.get(step).copied().unwrap_or(1.0)
    }
}

pub fn parse_scl(text: &str) -> Result<ScalaScale, String> {
    let mut description = String::from("Untitled Scala scale");
    let mut expected: Option<usize> = None;
    let mut ratios = vec![1.0];
    let mut saw_desc = false;

    for raw in text.lines() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with('!') {
            continue;
        }
        if !saw_desc {
            description = line.to_string();
            saw_desc = true;
            continue;
        }
        if expected.is_none() {
            expected = Some(
                line.split_whitespace()
                    .next()
                    .unwrap_or("0")
                    .parse::<usize>()
                    .map_err(|_| format!("Invalid Scala degree count: {line}"))?,
            );
            continue;
        }
        if let Some(r) = parse_scl_pitch(line)? {
            ratios.push(r);
        }
        if let Some(n) = expected {
            if ratios.len() >= n + 1 {
                break;
            }
        }
    }

    if ratios.len() < 2 {
        return Err("Scala file must define at least one interval plus the period".into());
    }
    Ok(ScalaScale { description, ratios })
}

fn parse_scl_pitch(line: &str) -> Result<Option<f64>, String> {
    let tok = line.split_whitespace().next().unwrap_or("");
    if tok.is_empty() {
        return Ok(None);
    }
    if tok.contains('/') {
        let mut parts = tok.split('/');
        let num: f64 = parts
            .next()
            .unwrap_or("1")
            .parse()
            .map_err(|_| format!("Bad ratio numerator in '{tok}'"))?;
        let den: f64 = parts
            .next()
            .unwrap_or("1")
            .parse()
            .map_err(|_| format!("Bad ratio denominator in '{tok}'"))?;
        if den.abs() < 1e-12 {
            return Err(format!("Zero denominator in '{tok}'"));
        }
        return Ok(Some(num / den));
    }
    // Cents: 1200.0 cents = 2/1
    let cents: f64 = tok
        .parse()
        .map_err(|_| format!("Bad cents/ratio token '{tok}'"))?;
    Ok(Some(2.0_f64.powf(cents / 1200.0)))
}

pub fn load_scl<P: AsRef<Path>>(path: P) -> Result<ScalaScale, String> {
    let text = fs::read_to_string(path.as_ref()).map_err(|e| format!("Scala read failed: {e}"))?;
    parse_scl(&text)
}

pub fn equal_temperament(n: usize, period: f64) -> ScalaScale {
    let n = n.max(1);
    let mut ratios = Vec::with_capacity(n + 1);
    for i in 0..=n {
        ratios.push(period.powf(i as f64 / n as f64));
    }
    ScalaScale {
        description: format!("{n}-EDO (period {period:.4})"),
        ratios,
    }
}

pub fn builtin_scala_library() -> Vec<(&'static str, ScalaScale)> {
    vec![
        ("edo12", equal_temperament(12, 2.0)),
        ("edo19", equal_temperament(19, 2.0)),
        ("edo31", equal_temperament(31, 2.0)),
        ("edo53", equal_temperament(53, 2.0)),
        (
            "quarter_comma_meantone",
            ScalaScale {
                description: "1/4-comma meantone (12 tones)".into(),
                ratios: vec![
                    1.0,
                    1.0449,
                    1.1180,
                    1.1963,
                    1.2500,
                    1.3375,
                    1.3975,
                    1.4953,
                    1.5625,
                    1.6719,
                    1.7889,
                    1.8692,
                    2.0,
                ],
            },
        ),
        (
            "werckmeister_iii",
            ScalaScale {
                description: "Werckmeister III well temperament".into(),
                ratios: cents_scale(
                    "Werckmeister III",
                    &[
                        0.0, 90.2, 192.2, 294.1, 390.2, 498.0, 588.3, 696.1, 792.2, 888.3, 996.1,
                        1092.2, 1200.0,
                    ],
                )
                .ratios,
            },
        ),
        (
            "young_1799",
            cents_scale(
                "Thomas Young 1799/1807",
                &[
                    0.0, 94.0, 196.0, 298.0, 392.0, 502.0, 592.0, 698.0, 796.0, 894.0, 1000.0,
                    1090.0, 1200.0,
                ],
            ),
        ),
        (
            "bp_lambda",
            equal_temperament(13, 3.0), // Bohlen–Pierce 13-ED3
        ),
        (
            "harmonic_12",
            ScalaScale {
                description: "Overtone 8..16 mapped into one octave".into(),
                ratios: vec![
                    1.0,
                    9.0 / 8.0,
                    10.0 / 8.0,
                    11.0 / 8.0,
                    12.0 / 8.0,
                    13.0 / 8.0,
                    14.0 / 8.0,
                    15.0 / 8.0,
                    2.0,
                ],
            },
        ),
    ]
}

fn cents_scale(desc: &str, cents: &[f64]) -> ScalaScale {
    ScalaScale {
        description: desc.to_string(),
        ratios: cents.iter().map(|c| 2.0_f64.powf(c / 1200.0)).collect(),
    }
}

pub fn find_builtin_scala(id: &str) -> Option<ScalaScale> {
    let key = id.trim().to_ascii_lowercase();
    builtin_scala_library()
        .into_iter()
        .find(|(name, _)| *name == key)
        .map(|(_, s)| s)
}
