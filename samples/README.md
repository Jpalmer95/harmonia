# Sample renders

**[▶ Listen in the browser — jpalmer95.github.io/harmonia/samples](https://jpalmer95.github.io/harmonia/samples/)** —
waveforms, click-to-seek playback, nothing to clone or download. (GitHub serves a `.mp3` blob as a file
page, not a player, which is why the page exists.)

Two short, unedited Harmonia renders — a taste of what the engine sounds like before you build anything.
Both are straight `harmonia render` output (48 kHz / 24-bit float-free PCM, peak normalized to the
default `normalize_peak` of 0.92 ≈ −0.7 dBFS), transcoded to 192 kbps MP3 so the repo stays small.
Click a file above to play it in the GitHub blob view, or just download it.

| Sample | Length | What you are hearing | Reproduce |
|---|---|---|---|
| [`bach_meets_vangelis.mp3`](bach_meets_vangelis.mp3) | 0:24 | Retro-classical FM brass under a slowly morphing modulation index — the Chowning brass that made FM famous, opened up into a cathedral. | `harmonia render bach_meets_vangelis` |
| [`neon_organum.mp3`](neon_organum.mp3) | 0:25 | A sustained organ-like pad stack, dense harmonic series, shimmer + cathedral reverb tail. | `harmonia render neon_organum` |

No outside instruments, no samples, no DAW, no cloud synth: every sound here is the in-tree 6-operator
FM matrix plus the math DSP and the effect chain, generated from a JSON spec.

## Why MP3 and not the master

`renders/` and `library/` are git-ignored on purpose — WAVs are derived artifacts (see `AGENTS.md`).
Hi-def masters are yours to regenerate locally at any resolution:

```bash
harmonia render bach_meets_vangelis --sr 96000 --bits 32f -o master.wav
```

`presets/bach_meets_vangelis.json` is the canonical spec for that piece; `neon_organum` is a built-in
flagship (see `src/presets.rs`). Edit either, re-render, and it is a different piece.

## Publishing

`index.html` is a single self-contained page (no build step, no external requests) published by GitHub
Pages from `main` at `/`. `og.png` is the 1200×630 share card, rendered once from `og.html`; regenerate
it only when the wording changes. Adding a sample means: drop the MP3 here, add a `.card` block with a
`data-src` and an `id`, and extend the `WAVE` map in the script with a peak array (720 points, decoded
from the MP3, normalized to 1.0).

## License

These audio renders are released under the same **Apache-2.0** terms as the code.
Copyright 2026 Jonathan Korstad.
