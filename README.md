# Kwybars

Kwybars is a Wayland-first desktop audio visualizer for Linux.

It is inspired by terminal visualizers like `cava`, but the target UX is a real
transparent desktop overlay anchored to a screen edge.

Current status: GTK4 + layer-shell + live audio backend scaffold.

## Current Features

- GTK4 application window for visualizer rendering.
- Wayland layer-shell anchoring via `gtk4-layer-shell`.
- Edge placement from config: `bottom`, `top`, `left`, `right`.
- Live frame backend with selectable input:
  - `cava`: primary/default backend
  - `pipewire`: fallback or explicit backend
  - `dummy`: synthetic animation
  - `auto`: same priority as default (`cava -> pipewire -> dummy`)

Not implemented yet:

- Direct in-process PipeWire client (without external `pw-cat`).
- Multi-monitor control.
- User theming controls.

## Requirements (Arch Linux)

```bash
sudo pacman -S --needed rust gtk4 gtk4-layer-shell pipewire cava
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
backend = "cava" # default: cava, fallback: pipewire
bars = 48
bar_width = 6
gap = 3
framerate = 60

# PipeWire tuning (applies when backend = "pipewire" or when fallback activates)
pipewire_attack = 0.14
pipewire_decay = 0.975
pipewire_gain = 1.20
pipewire_curve = 0.95
pipewire_neighbor_mix = 0.24
```

Softer PipeWire preset:

```toml
[visualizer]
backend = "pipewire"
pipewire_attack = 0.10
pipewire_decay = 0.985
pipewire_gain = 1.05
pipewire_curve = 1.05
pipewire_neighbor_mix = 0.30
```

## Workspace Layout

- `crates/common`: shared config and frame model.
- `crates/engine`: visualizer frame pipeline and live source backends.
- `crates/overlay`: GTK overlay app (windowing + rendering).
