# Harmonia

**Hi-def 6-operator FM matrix synthesizer, symbolic mathematical DSP, and multiline chord workstation — pure Rust, zero audio crates.**

Out of the box: retro-classical FM brass whose *modulation index* slowly morphs, glass cathedral pads, Cthulhu-style weaves, Just Intonation, Yamaha DX7 algorithms 1–32, Scala microtuning, Standard MIDI Files, and a live math playground (`fm()`, `cheb()`, `ks()`, `bell()`, `fm3()`).

```bash
git clone https://github.com/Jpalmer95/harmonia.git
cd harmonia
cargo run --release -- list
cargo run --release -- render bach_meets_vangelis --play
cargo run --release -- serve --port 8765   # http://127.0.0.1:8765  ← the tinkering UI
```

## Quickstart (5 minutes)

1. **Install Rust** ([rustup](https://rustup.rs)) — `rustc 1.70+`.
2. **Build** `cargo build --release` (always `--release`; debug is too slow for 48 kHz polyphony).
3. **Hear something** `./target/release/harmonia render bach_meets_vangelis --play`
4. **Open the studio** `./target/release/harmonia serve --port 8765` then visit http://127.0.0.1:8765
5. **Write a progression** `./target/release/harmonia chord "Cm9 -> Abmaj9#11 -> Fm11 -> G7b9" --tuning just --play`

Need a DAW? You don't. Harmonia *is* the sound engine, the sequencer, and the mastering chain.

## Is there a UI?

**Yes.** `harmonia serve` is the playground: flagship compositions, a chord keyboard, FM 6×6 matrix, DX7 algorithm picker, math formulas, Scala tunings, live 24-bit render + HTML5 audio. Tabs:

| Tab | For |
|---|---|
| Playground | Chords, pattern, tuning, math mix, DX7 algo, render |
| How it works | In-app tutorial for tinkering |
| FM matrix / DX7 | Edit the 6×6 graph; apply Yamaha algorithms 1–32 |

The CLI is the same engine (`harmonia chord`, `harmonia math`, `harmonia midi`). Agents use the CLI; humans can use either.

## Why this exists

Commercial FM (DX7 / FM8 / Serum / Phase Plant) plus Scaler/Cthulhu/Ableton is the usual stack. Harmonia is the **self-hosted, git-diffable, agent-native** version: JSON specs in the repo, 24-bit / 96 kHz WAV out, no DAW, no cloud synths (BYOC).

## Architecture

| Module | Role |
|---|---|
| `fm_synth` | 6 operators, 6×6 mod matrix, LFO-driven index, unison, glide |
| `dx7` | Yamaha DX7 algorithms 1–32 as routing tables |
| `math_engine` | Tokenize → stack VM, sample-accurate |
| `theory` | Chord parser, voice-leading, 6 multiline patterns, tunings |
| `scala` | `.scl` parser + 19/31/53-EDO, meantone, Werckmeister, Bohlen–Pierce |
| `midi` | Standard MIDI File Type 0 import/export |
| `karplus` | Karplus–Strong plucked string (`ks(f,t,damp)`) |
| `effects` | ZDF SVF, chorus, ping-pong delay, cathedral + shimmer reverb |
| `wav` | 16 / 24 PCM + 32-bit float, telemetry |
| `json` | Canonical composition specs (git is the database) |
| `server` | Studio UI + `POST /api/render` |

Extension points: `CustomSignalGenerator`, `CustomAudioEffect` in `src/lib.rs`.

## Manual use

See **[docs/SPEC.md](docs/SPEC.md)** for the full language (chords, math VM, JSON schema, MIDI, Scala, DX7).

```bash
harmonia list
harmonia render glass_cathedral_ji --sr 96000 --bits 32f -o master.wav
harmonia chord "Am9 -> Fmaj7#11 -> Cmaj9/E -> E7b9" --pattern stranger_pulse --lfo-rate 0.38 --play
harmonia math "env * ks(f, t, 0.45)" --freq 220 --play
harmonia math chowning_fm_brass --chords "Em9 -> Cmaj9#11" --tuning edo31 --play
harmonia midi export bach_meets_vangelis -o renders/bach.mid
harmonia midi render song.mid --play
harmonia dx7 5                  # preview Yamaha algorithm 5
harmonia inspect renders/bach_meets_vangelis.wav
harmonia export-presets presets/
```

Progression syntax: `Cm9 -> Abmaj9#11`, slash bass `Cmaj9/E`, duration `Cm9*8`, raw Hz `[220, 275, 330, 412.5]`.

Tunings: `equal`, `just`, `432`, `pythagorean`, `harmonic`, `phi`, `edo19`, `edo31`, `edo53`, `quarter_comma_meantone`, `werckmeister_iii`, `young_1799`, `bp_lambda`.

Patterns: `cthulhu_cascade`, `bach_prelude`, `sustained_cathedral`, `stranger_pulse`, `polyrhythm_3v4`, `counterpoint_four_voice`.

## Agent use

Read **[AGENTS.md](AGENTS.md)**. Short version: never invent WAV bytes; call `harmonia render|chord|math`; JSON under `presets/` is canonical.

Hermes skill: `harmonia-music`.

## License

MIT. DX7 algorithm graphs are public routing tables (not ROM samples). Scala format is the Huygens-Fokker open spec. Karplus–Strong is a public-domain algorithm.
