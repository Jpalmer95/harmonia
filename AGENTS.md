# AGENTS.md — Harmonia

Hi-def 6-operator FM matrix synth, symbolic math DSP, and multiline chord workstation in **pure Rust** (stdlib only).

- Repo: `/home/jonathan/dev/harmonia`
- Binary: `cargo run --release -- <subcommand>`
- JSON is the database: compositions live as `.json` under `presets/`
- Never add audio crates unless a measured bottleneck exists
- Hermes skill: `harmonia-music` — load it before composing or rendering

```bash
cd /home/jonathan/dev/harmonia
cargo test
cargo run --release -- list
cargo run --release -- render bach_meets_vangelis --play
```
