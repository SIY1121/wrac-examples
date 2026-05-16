# Example 追加メモ

This document is intended for maintainers.
これはメンテナー向けドキュメントです。

新しい example は、近い既存 example をコピーして作る。最小 audio effect なら
`gain-basic`、instrument / MIDI 入力なら `sine-synth` を起点にする。

変更するもの:

- `examples/my-example/src-plugin/Cargo.toml`
  - `[package].name`
  - `[package].description`
  - `[package].repository`
  - `[package.metadata.wrac]`
- `examples/my-example/src-gui/package.json`
- `examples/my-example/src-gui/package-lock.json`
- root `Cargo.toml` の `workspace.members`
- `examples/my-example/README.md`
- `examples/my-example/README_JA.md`
- root `README.md` / `README_JA.md` の example 一覧
- `.vscode/launch.json` / `.vscode/tasks.json`

