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

## プラグインのインストール

`install` は、ビルド済みのプラグインの artifact を DAW がスキャンするディレクトリにコピーします。
事前に `build` を実行しておく必要があります（`install` はビルドを行いません）。

```sh
# ひとつの example を user-local のプラグインフォルダにインストール
cargo xtask install --plugin=gain-basic

# すべての example をインストール
cargo xtask install --all

# release の artifact をインストール
cargo xtask install --plugin=gain-basic --release

# 特定のフォーマットだけをインストール
cargo xtask install --plugin=gain-basic --target=clap,vst3

# user-local ではなく system-wide にインストール
cargo xtask install --plugin=gain-basic --scope=system
```

デフォルトの target は OS ごとに決まります（macOS: clap, vst3, au / Windows・Linux: clap, vst3）。
`standalone` はプラグインフォーマットではないので、このコマンドではインストールできません。代わりに `launch` を使ってください。

scope ごとのインストール先:

| OS      | Scope  | CLAP                                 | VST3                                 | AU                                          |
| ------- | ------ | ------------------------------------ | ------------------------------------ | ------------------------------------------- |
| macOS   | user   | `~/Library/Audio/Plug-Ins/CLAP`      | `~/Library/Audio/Plug-Ins/VST3`      | `~/Library/Audio/Plug-Ins/Components`       |
| macOS   | system | `/Library/Audio/Plug-Ins/CLAP`       | `/Library/Audio/Plug-Ins/VST3`       | `/Library/Audio/Plug-Ins/Components`        |
| Windows | user   | `%LOCALAPPDATA%\Programs\Common\CLAP` | `%LOCALAPPDATA%\Programs\Common\VST3` | —                                           |
| Windows | system | `%CommonProgramFiles%\CLAP`           | `%CommonProgramFiles%\VST3`           | —                                           |
| Linux   | user   | `~/.clap`                            | `~/.vst3`                            | —                                           |
| Linux   | system | `/usr/lib/clap`                      | `/usr/lib/vst3`                      | —                                           |

インストール済みの artifact を削除するには `cargo xtask uninstall` を使います。`--plugin` / `--all` / `--target` は install と同じく指定でき、加えて `--scope=all|user|system`（デフォルトは `all`）と、削除せずに対象パスだけを表示する `--dry-run` があります。

```sh
cargo xtask uninstall --plugin=gain-basic
cargo xtask uninstall --plugin=gain-basic --dry-run
```

## メインリポジトリ

メインプロジェクトは [wrac-plugin-template](https://github.com/novonotes/wrac-plugin-template) です。
テンプレート、セットアップガイド、ビルドワークフロー、サポート窓口はそちらにあります。

このリポジトリは、そのテンプレートから作られた補助的なサンプルだけを含みます。
質問、フィードバック、互換性レポートは [wrac-plugin-template issues](https://github.com/novonotes/wrac-plugin-template/issues) または [discussions](https://github.com/novonotes/wrac-plugin-template/discussions) を使ってください。
