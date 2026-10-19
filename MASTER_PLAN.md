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
- [x] CLI: `list` `render` `chord` `math` `inspect` `play` `export-presets` `serve` `midi` `dx7`
- [x] Web studio playground (learn / play / FM matrix tabs)
- [x] Hermes skill `harmonia-music` + public AGENTS.md
- [x] docs/SPEC.md

## Decisions

- Stdlib-only DSP (house style: prove bottleneck before deps). MIDI/Scala/DX7/KS are in-tree modules, not crates.
- Canonical JSON specs; WAV is derived; do not commit large renders.
- BYOC: no cloud synths, no paid audio APIs.
- Independent of DJ-JK / music-catalog (contract via WAV + JSON, not a monolith).
- DX7 tables are public routing graphs, not ROM samples.

## Next (optional)

- [ ] JACK/PipeWire live streaming (measured I/O bottleneck only)
- [ ] `.scl` file path on the CLI (`--scl path.scl`)
- [ ] Plugin crate for out-of-tree `CustomSignalGenerator`s
