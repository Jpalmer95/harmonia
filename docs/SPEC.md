# Harmonia — language & engine spec

This is the reference for humans and agents. The CLI and the studio UI are two fronts on the same types.

## 1. CompositionSpec (canonical JSON)

```json
{
  "id": "my_piece",
  "title": "My Piece",
  "bpm": 108,
  "progression": "Cm9 -> Abmaj9#11 -> Fm11 -> G7b9",
  "beats_per_chord": 4,
  "pattern": "cthulhu_cascade",
  "tuning": "just_intonation",
  "tuning_root_pc": 0,
  "tuning_a4_hz": 440,
  "dx7_algorithm": 0,
  "math_expr": "env * fm(f, f*2, mod_idx, t)",
  "math_mix": 0.0,
  "patch": { }
}
```

`base_composition` and `patch_id` overlay a builtin. Top-level keys also override patch knobs (`lfo1_rate_hz`, `reverb_shimmer`, `mod_matrix`, …). Git is the database: commit JSON, not WAV.

## 2. Chord language

- Separators: `->` `→` `|` `,` or whitespace.
- Roots: `C D E F G A B` plus `#` `b`.
- Qualities: `m` `min` `maj7` `maj9` `maj9#11` `m7b5` `dim` `aug` `sus2` `sus4` `7` `9` `11` `13` `alt` `add9` `6`.
- Slash bass: `Cmaj9/E`.
- Duration: `Cm9*8` (beats). Default `beats_per_chord`.
- Explicit Hz: `[220, 275, 330, 412.5]`.

Voice-leading is parsimonious SATB plus a bass pedal and an upper weave (pattern-dependent).

## 3. Multiline patterns

| id | Behaviour |
|---|---|
| `cthulhu_cascade` | Sustained SATB pad + cascading multi-octave arp |
| `bach_prelude` | Pedal + 16th-note interlocking figure |
| `sustained_cathedral` | Slow staggered pad + half-note countermelody |
| `stranger_pulse` | 8th bass + 16th ostinato + brass swell |
| `polyrhythm_3v4` | 3-tuplets against 16ths |
| `counterpoint_four_voice` | Bass / tenor / alto / soprano with portamento |

## 4. Tunings

| id | System |
|---|---|
| `equal` / `equal12` | 12-TET A4=440 |
| `432` | 12-TET A4=432 |
| `just` | 5/7-limit JI, root-adaptive |
| `pythagorean` | stacked 3:2 |
| `harmonic` | overtone series |
| `phi` | golden-ratio microtone |
| `edoN` | N equal divisions of 2/1 |
| `quarter_comma_meantone` | historical |
| `werckmeister_iii` `young_1799` | well temperament |
| `bp_lambda` | Bohlen–Pierce 13-ED3 |

Scala `.scl` files parse via `harmonia::scala::parse_scl`. Degree 0 = 1/1; last degree is the period.

## 5. Math VM

Infix `+ - * / % ^`, unary `-`, calls, constants `pi tau e phi`.

**Vars:** `t` note time, `gt` global time, `f` Hz, `w` = 2πft, `env` `vel` `note` `voice` `mod_idx` `lfo1` `lfo2` `lfo3` `x y z`.

**1-arg:** `sin cos tan tanh atan sinh cosh exp ln log2 sqrt cbrt abs sign floor ceil fract saw tri sq`

**2-arg:** `pow mod min max pulse fold cheb`

**3-arg:** `clamp lerp smoothstep harm(w,n,rolloff) pluck(f,t,damp) ks(f,t,damp)`

**4-arg:** `fm(fc, fm, idx, t)` `bell(f, t, idx, decay)`

**5/6:** `chord(f,t,r1,r2,r3)` `fm3(fc, r1, idx1, r2, idx2, t)`

Eval is a 64-slot stack; non-finite → 0. Sample-accurate, no heap in the inner loop.

## 6. FM matrix & DX7

`mod_matrix[src][dst]` is phase-modulation index in radians. `lfo1_matrix_depth` adds a slow morph on top of that (the “evolving FM” sound).

`dx7_algorithm` 1–32 loads Yamaha public routing tables (carriers, edges, feedback op). Algorithm 32 = all carriers (organ). 0 = leave the Harmonia patch graph.

## 7. MIDI SMF

Type 0 write / Type 0+1 read. Tempo meta, note on/off, running status. `harmonia midi export` maps voice-led events to 12-TET MIDI note numbers (JI pieces will equal-temper on the way out — expected). `midi render` plays an SMF through the glass-cathedral FM patch.

## 8. Audio I/O

Stereo, 22.05–192 kHz. Bit depths: 16 PCM, 24 PCM, 32-bit IEEE float. Master chain: DC block → ZDF SVF → chorus → ping-pong delay → cathedral+shimmer reverb → tanh drive → fades → peak −0.72 dBFS.

## 9. HTTP API (`harmonia serve`)

- `GET /` studio UI (`web/studio.html`)
- `GET /api/presets` compositions + math formulas
- `POST /api/render` JSON body = CompositionSpec overrides → `{ok, wav_url, peak_dbfs, ascii_envelope, …}`
- `GET /renders/<file>.wav`
- `GET /api/library` Harmonia Blocks index
- `POST /api/library` `{action: save|delete|mash, …}` — JSON always; `format: wav|mp3|json` attaches optional audio
- `POST /api/analyze` raw WAV bytes → pitch/chord **sketch** (not a transcription)

## 10. Harmonia Blocks & analyze bounds

Canonical store: `library/index.json`. WAV/MP3 under `library/audio/` are optional. Keep policies: `library` (until deleted), `ttl:<secs>`, `next_generation`.

**`harmonia analyze` can:** estimate a melody pitch contour (YIN), a 12-class chroma chord sketch (`maj`/`m`/`7`/`m7`), a rough BPM, and seed a Block progression.

**It cannot:** recover an FM operator graph, a math formula, DX7 algorithm, isolated stems, vocals, or a DAW-accurate score. Treat output as a starting point to tweak.

Mashup: `harmonia library mash <id_a> <id_b>` interleaves two progressions (JSON fusion, not audio stem mix).

Song-extend (Suno / MusicGen / etc.) is **out of process**. Prefer a local OSS model as BYOC sidecar later; do not call paid cloud APIs from this engine.

## 11. Extension traits

```rust
pub trait CustomSignalGenerator: Send + Sync {
    fn name(&self) -> &str;
    fn sample(&mut self, time_sec: f64, freq_hz: f64, velocity: f64) -> (f64, f64);
}
pub trait CustomAudioEffect: Send + Sync {
    fn name(&self) -> &str;
    fn process(&mut self, buffer: &mut StereoBuffer);
}
```
