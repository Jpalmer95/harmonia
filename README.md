# Harmonia

**Hi-def 6-operator FM matrix synthesizer, symbolic mathematical DSP, and multiline chord workstation — pure Rust, zero audio crates.**

Out of the box you get the retro-classical-meets-modern sound: frequency-modulated multiline chords whose operator *index* slowly morphs under an LFO, glass cathedral FM, Cthulhu-style weaves, Just Intonation, and a live math playground (`fm()`, `cheb()`, `harm()`, `bell()`, `pluck()`, `fm3()`).

```bash
cd /home/jonathan/dev/harmonia
cargo run --release -- list
cargo run --release -- render bach_meets_vangelis --play
cargo run --release -- chord "Cm9 -> Abmaj9#11 -> Fm11 -> G7b9" --pattern cthulhu_cascade --tuning just --play
cargo run --release -- math "env * fm(f, f*phi, mod_idx*(0.5+0.5*lfo1), t)" --chords "Em9 -> Cmaj9#11" --play
cargo run --release -- serve --port 8765   # http://127.0.0.1:8765
```

## Why this exists

Commercial FM (DX7 / FM8 / Serum / Phase Plant) plus Scaler/Cthulhu/Ableton is the usual stack. Harmonia is the **self-hosted, git-diffable, agent-native** version of that stack: JSON specs in the repo, 24-bit / 96 kHz WAV out, Hermes skill in, no DAW required.

## Architecture

| Module | Role |
|---|---|
| `fm_synth` | 6 operators, 6×6 mod matrix, LFO-driven index, unison, glide |
| `math_engine` | Tokenize → AST bytecode VM, sample-accurate |
| `theory` | Chord parser, voice-leading, 6 multiline patterns, tunings |
| `effects` | ZDF SVF, chorus, ping-pong delay, cathedral + shimmer reverb |
| `wav` | 16 / 24 PCM + 32-bit float, telemetry |
| `json` | Canonical composition specs (git is the database) |
| `server` | Studio UI + `POST /api/render` |

Extension points: `CustomSignalGenerator`, `CustomAudioEffect`.

## Hermes

Load skill **`harmonia-music`**. Agents should call `harmonia chord` / `harmonia math` / `harmonia render`, never invent audio bytes.

## License

MIT.
