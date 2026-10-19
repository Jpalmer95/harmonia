# Harmonia — MASTER_PLAN

Hi-def sounds and music: 6-op FM, live math equations, multiline chord weaves.

## Done

- [x] Pure-Rust 6×6 FM matrix with LFO-driven modulation index
- [x] Symbolic math AST → stack VM (`sin`, `fm`, `cheb`, `harm`, `bell`, `pluck`, `fm3`, `ks`, …)
- [x] Chord parser + parsimonious voice-leading + 6 multiline patterns
- [x] Just / Pythagorean / 432 / harmonic / phi tunings
- [x] Scala `.scl` + 19/31/53-EDO, meantone, Werckmeister, Bohlen–Pierce
- [x] Yamaha DX7 algorithms 1–32 as in-tree routing tables
- [x] Standard MIDI File import/export
- [x] Karplus–Strong physical model
- [x] ZDF SVF, chorus, ping-pong delay, cathedral+shimmer reverb
- [x] 16/24-bit PCM + 32-bit float WAV, no crates
- [x] CLI: `list` `render` `chord` `math` `inspect` `play` `export-presets` `serve` `midi` `dx7` `analyze` `library`
- [x] Web studio playground (learn / play / library / FM matrix tabs)
- [x] Hermes skill `harmonia-music` + public AGENTS.md
- [x] docs/SPEC.md
- [x] Harmonia Blocks library (`library/index.json`) with optional WAV/MP3
- [x] WAV pitch/chord *sketch* (`analyze`) — not FM reverse-engineering
- [x] Four more flagships (lute/31-EDO, neon organum, lambda choir, Buchla dusk)
- [x] Studio chord builder: quality chips, long-press/right-click palette, progression chips, slash bass, length
- [x] Visible library-save feedback (toast + tab switch); formula lint; patch picker; MIDI export; spec undo; limiter/normalize/dither; analyze chip confidence

## Decisions

- Stdlib-only DSP (house style: prove bottleneck before deps). MIDI/Scala/DX7/KS are in-tree modules, not crates.
- Canonical JSON specs; WAV is derived; do not commit large renders.
- BYOC: no cloud synths, no paid audio APIs.
- Independent of DJ-JK / music-catalog (contract via WAV + JSON, not a monolith).
- DX7 tables are public routing graphs, not ROM samples.
- **Analyze is a sketch.** YIN melody + 12-class chroma cannot recover operator graphs, math formulas, or isolated stems. Seed a Block, then edit.
- Library: JSON always; WAV/MP3 opt-in. Default keep = explicit save. Unsaved renders may TTL or drop on next generate.

## Next (optional)

- [ ] JACK/PipeWire live streaming (measured I/O bottleneck only)
- [ ] `.scl` file path on the CLI (`--scl path.scl`)
- [ ] Plugin crate for out-of-tree `CustomSignalGenerator`s
- [ ] Local OSS song-extend (Stable Audio Open / MusicGen / ACE-Step) as a *separate* BYOC sidecar — never in-process, never Suno-by-default
