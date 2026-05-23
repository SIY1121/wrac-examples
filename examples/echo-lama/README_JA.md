# Echo Lama

> English version: [README.md](README.md)

Monophonic な vowel-morphing synth の example です。note input port を 1 つと
stereo audio output port を 1 つ宣言し、built-in delay と専用 WebView GUI を
備えています。

```sh
cargo xtask build --plugin=echo-lama --target=clap
```
