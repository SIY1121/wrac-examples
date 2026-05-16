# WRAC Gain Basic

> 日本語版: [README_JA.md](README_JA.md)

Minimal audio-effect example based on the WRAC plugin template.

```sh
cargo xtask build --plugin=gain-basic --target=clap
```

The plugin exposes a gain parameter, bypass parameter, state save/restore, and
the WebView GUI from the template.
