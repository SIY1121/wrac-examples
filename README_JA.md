# wrac-examples

WRAC スタックで作るオーディオプラグインの example 集です。

> English version: [README.md](README.md)

## 構成

```text
examples/
  gain-basic/   基本的な audio effect の gain plugin。
  sine-synth/   MIDI note input でサイン波を出す instrument plugin。
```

## セットアップ

```sh
git submodule update --init --recursive
```

## ビルド

```sh
# すべての example を CLAP としてビルド
cargo xtask build --all --target=clap

# ひとつの example だけをビルド
cargo xtask build --plugin=sine-synth --target=clap

# VST3 / AU / Standalone は template submodule 内の wrapper builder を使う
cargo xtask build --plugin=gain-basic --target=vst3,au,standalone
```

`--plugin` に渡す値は、`examples/` 直下の example directory 名です。
たとえば `examples/sine-synth` は `--plugin=sine-synth` で指定します。


