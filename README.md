# Pomowork

A small cross-platform desktop app that combines a stopwatch and a 25-minute Pomodoro timer, built in Rust with [egui/eframe](https://github.com/emilk/egui).

## Features

- **Stopwatch** on top with Start/Pause and Reset (shown only once paused with time on the clock). Displays `HH:MM:SS`.
- **Pomodoro** below it, starting at 25:00, with Start/Pause. Shows "Break time!" when finished; "Start new" begins a fresh session.
- Window is centered on screen and can be pinned always-on-top (on by default).
- Session state persists to disk, so closing and reopening the app restores both timers (including elapsed time while the app was closed).

## Build & run

```bash
cargo run --release
```

On Linux you may need system packages for X11/Wayland and graphics:

```bash
# Debian/Ubuntu
sudo apt install libxkbcommon-dev libwayland-dev libx11-dev libxcb1-dev libxrandr-dev \
                 libxi-dev libgl1-mesa-dev libudev-dev
```

Cross-compiling to Windows from Linux requires the MinGW target and linker (or use a Windows machine/CI):

```bash
rustup target add x86_64-pc-windows-gnu
cargo build --release --target x86_64-pc-windows-gnu
```

State is saved to:

- Linux: `~/.local/share/pomowork/state.json`
- Windows: `%APPDATA%\pomowork\state.json`
