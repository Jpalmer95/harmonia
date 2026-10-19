# Harmonia — MASTER_PLAN

Hi-def sounds and music: 6-op FM, live math equations, multiline chord weaves.

## Done

- [x] Pure-Rust 6×6 FM matrix with LFO-driven modulation index
- [x] Symbolic math AST → stack VM (`sin`, `fm`, `cheb`, `harm`, `bell`, `pluck`, `fm3`, …)
- [x] Chord parser + parsimonious voice-leading + 6 multiline patterns
- [x] Just / Pythagorean / 432 / harmonic / phi tunings
- [x] ZDF SVF, chorus, ping-pong delay, cathedral+shimmer reverb
- [x] 16/24-bit PCM + 32-bit float WAV, no crates
- [x] CLI: `list` `render` `chord` `math` `inspect` `play` `export-presets` `serve`
- [x] Web studio + `/api/render`
- [x] Hermes skill `harmonia-music`

## Decisions

- Stdlib-only DSP (house style: prove bottleneck before deps).
- Canonical JSON specs; WAV is derived.
- BYOC: no cloud synths, no paid audio APIs.
- Independent of DJ-JK / music-catalog (contract via WAV + JSON, not a monolith).

## Next (optional)

- [ ] MIDI file import/export
- [ ] Live JACK/PipeWire streaming (would be a measured I/O bottleneck)
- [ ] Plugin crate for custom operators via `CustomSignalGenerator`
