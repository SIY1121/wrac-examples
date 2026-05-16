# WRAC Sine Synth

> 日本語版: [README_JA.md](README_JA.md)

Minimal instrument example. It declares one note input port and one stereo audio
output port. Incoming note-on/note-off events drive a small polyphonic sine-wave
oscillator, and the template GUI's gain control is reused as output level.

```sh
cargo xtask build --plugin=sine-synth --target=clap
```
