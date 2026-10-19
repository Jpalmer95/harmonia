# AGENTS.md — Harmonia

Hi-def 6-operator FM matrix synth, symbolic math DSP, and multiline chord workstation in **pure Rust** (stdlib only). This file is the contract for any coding agent (Hermes, Claude, Codex, Aider, …).

## Identity

- Repo: this directory. Binary: `cargo run --release -- <subcommand>` or `./target/release/harmonia`.
- JSON under `presets/` is the database. WAV under `renders/` is **derived** — never commit large WAVs.
- Never add audio crates unless a measured bottleneck exists (house style: stdlib-only DSP).
- Hermes skill: `harmonia-music` — load it before composing or rendering.

## Do / don't

| Do | Don't |
|---|---|
| `harmonia render\|chord\|math\|midi\|dx7\|analyze\|library` | Invent PCM / WAV bytes |
| Edit a JSON spec in `presets/` | Fork the engine for a one-off song |
| `--release` for anything > 1s of audio | Run long renders in debug |
| `harmonia inspect <wav>` after a render | Claim a render succeeded without the file |
| 24-bit / 48 kHz default; `--sr 96000 --bits 32f` for masters | Call cloud synth APIs |

## First commands

```bash
cargo test
cargo build --release
./target/release/harmonia list
./target/release/harmonia render bach_meets_vangelis --play
./target/release/harmonia serve --port 8765
```

## How to make music (agent workflow)

1. Pick a flagship ID **or** write a progression string.
2. Render:
   ```bash
   ./target/release/harmonia chord "Cm9 -> Abmaj9#11 -> Fm11 -> G7b9" \
     --patch retro_classical_brass --pattern cthulhu_cascade --tuning just \
     -o renders/take.wav
   ```
3. Verify: `./target/release/harmonia inspect renders/take.wav` — expect peak ≈ −0.7 dBFS, duration > 0.
4. If the user wants a new reusable piece: `harmonia export-presets presets/` then edit the JSON (id, progression, pattern, patch knobs). Do not copy `src/presets.rs` for a one-off.

## Vocabulary

- **Patches:** `retro_classical_brass`, `glass_cathedral`, `cyber_baroque_harpsichord`, `math_harmonic_weaver`
- **Patterns:** `cthulhu_cascade`, `bach_prelude`, `sustained_cathedral`, `stranger_pulse`, `polyrhythm_3v4`, `counterpoint_four_voice`
- **Tunings:** `equal`, `just`, `432`, `pythagorean`, `harmonic`, `phi`, `edo19`, `edo31`, `edo53`, `quarter_comma_meantone`, `werckmeister_iii`, `bp_lambda`
- **Math vars:** `t gt f w env vel mod_idx lfo1 lfo2 lfo3 x y z phi pi tau note voice`
- **Math fns:** `sin cos tanh saw tri sq harm(w,n,rolloff) fm(fc,fm,idx,t) fm3(...) cheb(n,x) fold(x,g) bell(f,t,idx,decay) pluck(f,t,damp) ks(f,t,damp) chord(f,t,r1,r2,r3)`
- **DX7:** `harmonia dx7 5` or JSON `"dx7_algorithm": 5` (Yamaha algos 1–32)
- **MIDI:** `harmonia midi export <id> -o out.mid` / `harmonia midi render file.mid`
- **Analyze:** `harmonia analyze clip.wav --save` — YIN + chroma **sketch**. Never claim it recovered an FM equation.
- **Library:** `harmonia library list|save|mash|delete` — JSON is the block; WAV/MP3 optional.

Chord syntax: `Cm9 -> Abmaj9#11`, slash bass `Cmaj9/E`, duration `Cm9*8`, Hz list `[220, 275, 330]`.

Agent bounds for “extract this recording”:
1. Run `analyze`. Report confidence + caveats verbatim.
2. Seed a Block from `progression` / `hz_list`.
3. Let the human (or a follow-up prompt) pick patch, pattern, math, DX7.
4. Do **not** invent operator matrices or formulas from the WAV.

Full spec: [docs/SPEC.md](docs/SPEC.md). Human quickstart: [README.md](README.md).

## Extending the engine

- New oscillator / physical model → `CustomSignalGenerator` or a `math_engine` `FuncId`.
- New FX → `CustomAudioEffect` or `src/effects.rs`.
- New tuning → `src/scala.rs` builtin library, or parse a `.scl`.
- Keep DSP stdlib-only. MIDI/Scala/DX7 are in-tree modules, not crates.

## Tests

```bash
cargo test
```

Must stay green. Adds: math VM, chord parser, Just ratios, JSON roundtrip, MIDI SMF, Scala `.scl`, DX7 algos, Karplus.
