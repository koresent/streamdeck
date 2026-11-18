# streamdeck-gui

Fast Wayland GUI and service for Elgato Stream Deck devices written in Rust.

## Features

- **Wayland Native**: GUI built on `eframe` (`egui`) using the `winit` Wayland backend. Hotkeys and text typing are dispatched directly via `/dev/uinput` (`evdev`).
- **Performance**: Written in Rust with minimal startup time and memory footprint (~25 MB).
- **Device Support**: Automatic detection and hotplugging via `hidapi` (Stream Deck Original, MK.2, Mini, XL, Plus, Pedal).
- **Virtual Mode**: Interactive mock deck for configuration and testing without physical hardware.
- **Config Compatibility**: Reads and writes standard `~/.streamdeck_ui.json` configuration files with JSON import/export and up to 10 pages per deck.
- **Button Actions**: Shell commands, hotkeys, text typing, page switching, and brightness controls.
- **Visuals**: PNG, JPEG, WebP, GIF, and SVG rendering with customizable labels and alignment.

## Building and Running

### Prerequisites

- Rust 1.85+
- `hidapi`

On Arch Linux:
```bash
sudo pacman -S hidapi
```

### Uinput Permissions

Ensure read/write access to `/dev/uinput` for input emulation:

```bash
echo 'KERNEL=="uinput", GROUP="input", MODE="0660", TAG+="uaccess"' | sudo tee /etc/udev/rules.d/70-uinput.rules
sudo udevadm control --reload-rules && sudo udevadm trigger
```

### Run

```bash
cargo run --release
```

## Testing

```bash
cargo test
```
