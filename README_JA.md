# wrac-examples

[WRAC Plugin Template](https://github.com/novonotes/wrac-plugin-template) の補助的なサンプルプラグイン集です。

このリポジトリは `wrac-plugin-template` とあわせて参照する補助リポジトリです。
WRAC スタックによるオーディオプラグイン開発について知りたい場合、まず `wrac-plugin-template` を参照してください。
このリポジトリは、追加の実装例を確認したい場合に使います。

> English version: [README.md](README.md)

## 構成

```text
examples/
  gain-basic/   基本的な audio effect の gain plugin。
  sine-synth/   MIDI note input でサイン波を出す instrument plugin。
  analyzer-3d/  リッチな3D表現のアナライザー
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

## Standalone の起動

```sh
cargo xtask build --plugin=gain-basic --target=standalone
cargo xtask launch --plugin=gain-basic
```

## メインリポジトリ

メインプロジェクトは [wrac-plugin-template](https://github.com/novonotes/wrac-plugin-template) です。
テンプレート、セットアップガイド、ビルドワークフロー、サポート窓口はそちらにあります。

このリポジトリは、そのテンプレートから作られた補助的なサンプルだけを含みます。
質問、フィードバック、互換性レポートは [wrac-plugin-template issues](https://github.com/novonotes/wrac-plugin-template/issues) または [discussions](https://github.com/novonotes/wrac-plugin-template/discussions) を使ってください。
