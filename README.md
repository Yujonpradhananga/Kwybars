# Kwybars

Kwybars is a Wayland-first desktop audio visualizer for Linux.

It is inspired by terminal visualizers like `cava`, but the target UX is a real
transparent desktop overlay anchored to a screen edge.

Current status: GTK4 + layer-shell + live audio backend scaffold.

## Current Features

- GTK4 application window for visualizer rendering.
- Wayland layer-shell anchoring via `gtk4-layer-shell`.
- Edge placement from config: `bottom`, `top`, `left`, `right`.
- Live frame backend with `auto` source selection:
  - tries `cava` raw output first
  - falls back to dummy animation when unavailable

Not implemented yet:

- Native PipeWire capture path (without `cava`).
- Multi-monitor control.
- User theming controls.

## Requirements (Arch Linux)

```bash
sudo pacman -S --needed rust gtk4 gtk4-layer-shell cava
```

## Build

```bash
cd /home/ns/Projects/Kwybars/Kwybars
cargo build --workspace
```

## Run

```bash
cd /home/ns/Projects/Kwybars/Kwybars
cargo run -p kwybars-overlay
```

Run this in a graphical Wayland session. Without a display server, GTK exits
with `Failed to open display`.

## Configuration

Config path resolution order:

1. `KWYBARS_CONFIG`
2. `$XDG_CONFIG_HOME/kwybars/config.toml`
3. `~/.config/kwybars/config.toml`
4. `./kwybars.toml`

Example:

```toml
[overlay]
position = "bottom"
anchor_margin = 12

[visualizer]
backend = "auto" # auto | cava | dummy
bars = 48
bar_width = 6
gap = 3
framerate = 60
```

## Workspace Layout

- `crates/common`: shared config and frame model.
- `crates/engine`: visualizer frame pipeline and live source backends.
- `crates/overlay`: GTK overlay app (windowing + rendering).
