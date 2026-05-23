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
  echo-lama/    Monophonic vowel-morphing synth with delay and a custom GUI.
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

## Main Repository

The main project is [wrac-plugin-template](https://github.com/novonotes/wrac-plugin-template). It contains the template, setup guide, build workflow, and support channels.

This repository only contains supplemental examples built from that template.
Please use the [wrac-plugin-template issues](https://github.com/novonotes/wrac-plugin-template/issues) or [discussions](https://github.com/novonotes/wrac-plugin-template/discussions) for questions, feedback, and compatibility reports.
