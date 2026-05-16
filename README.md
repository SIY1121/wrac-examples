# wrac-examples

Example audio plugins built with the WRAC stack

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
