# wrac-examples

Supplemental example plugins for [WRAC Plugin Template](https://github.com/novonotes/wrac-plugin-template).

This repository is a companion to `wrac-plugin-template`.
If you want to learn about audio plugin development with the WRAC stack, start with `wrac-plugin-template`.
Use this repository when you want to inspect additional example implementations.

> 日本語版: [README_JA.md](README_JA.md)

## Structure

```text
examples/
  gain-basic/   Basic audio-effect gain plugin.
  sine-synth/   MIDI note input to sine-wave instrument output.
```

## Setup

```sh
git submodule update --init --recursive
```

## Build

```sh
# Build every example as CLAP.
cargo xtask build --all --target=clap

# Build one example.
cargo xtask build --plugin=sine-synth --target=clap

# VST3 / AU / Standalone use the wrapper builder from the template submodule.
cargo xtask build --plugin=gain-basic --target=vst3,au,standalone
```

The value passed to `--plugin` is the example directory name under `examples/`.
For example, `examples/sine-synth` is selected with `--plugin=sine-synth`.

## Launch Standalone

```sh
cargo xtask build --plugin=gain-basic --target=standalone
cargo xtask launch --plugin=gain-basic
```

## Install Plugin

`install` copies previously built plugin artifacts into the directories that DAWs scan.
Run `build` first; `install` does not rebuild.

```sh
# Install one example into the user-local plugin folders.
cargo xtask install --plugin=gain-basic

# Install every example.
cargo xtask install --all

# Install release artifacts.
cargo xtask install --plugin=gain-basic --release

# Install only specific formats.
cargo xtask install --plugin=gain-basic --target=clap,vst3

# Install system-wide instead of user-local.
cargo xtask install --plugin=gain-basic --scope=system
```

The default target set follows the current OS (macOS: clap, vst3, au; Windows/Linux: clap, vst3).
`standalone` is not a plugin format and cannot be installed with this command — use `launch` instead.

Installation directories per scope:

| OS      | Scope  | CLAP                                 | VST3                                 | AU                                          |
| ------- | ------ | ------------------------------------ | ------------------------------------ | ------------------------------------------- |
| macOS   | user   | `~/Library/Audio/Plug-Ins/CLAP`      | `~/Library/Audio/Plug-Ins/VST3`      | `~/Library/Audio/Plug-Ins/Components`       |
| macOS   | system | `/Library/Audio/Plug-Ins/CLAP`       | `/Library/Audio/Plug-Ins/VST3`       | `/Library/Audio/Plug-Ins/Components`        |
| Windows | user   | `%LOCALAPPDATA%\Programs\Common\CLAP` | `%LOCALAPPDATA%\Programs\Common\VST3` | —                                           |
| Windows | system | `%CommonProgramFiles%\CLAP`           | `%CommonProgramFiles%\VST3`           | —                                           |
| Linux   | user   | `~/.clap`                            | `~/.vst3`                            | —                                           |
| Linux   | system | `/usr/lib/clap`                      | `/usr/lib/vst3`                      | —                                           |

To remove installed artifacts, use `cargo xtask uninstall`. It accepts the same `--plugin` / `--all` / `--target` flags, plus `--scope=all|user|system` (defaults to `all`) and `--dry-run` to preview the paths without deleting them.

```sh
cargo xtask uninstall --plugin=gain-basic
cargo xtask uninstall --plugin=gain-basic --dry-run
```

## Main Repository

The main project is [wrac-plugin-template](https://github.com/novonotes/wrac-plugin-template). It contains the template, setup guide, build workflow, and support channels.

This repository only contains supplemental examples built from that template.
Please use the [wrac-plugin-template issues](https://github.com/novonotes/wrac-plugin-template/issues) or [discussions](https://github.com/novonotes/wrac-plugin-template/discussions) for questions, feedback, and compatibility reports.
