# WRAC Sine Synth

> English version: [README.md](README.md)

最小構成の instrument example です。note input port を 1 つ、stereo audio
output port を 1 つ宣言します。入力された note-on/note-off event で簡易
polyphonic sine-wave oscillator を鳴らし、template GUI の gain control を出力
level として再利用しています。

```sh
cargo xtask build --plugin=sine-synth --target=clap
```
