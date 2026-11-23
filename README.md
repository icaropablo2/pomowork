# Pomowork

A small cross-platform desktop app that combines a stopwatch and a Pomodoro timer with three session modes, built in Rust with [egui/eframe](https://github.com/emilk/egui).

## Features

- **Total worked today** (stopwatch) on top, with no border, with Start/Pause and Reset (shown only once paused with time on the clock). Displays `HH:MM:SS`.
- **Timer** below it in a darker, centered square panel with session buttons at the top: **Pomodoro** (25 min), **Short Break** (5 min), and **Long Break** (15 min). The active mode is highlighted, and the Start button uses the same color. You can select any session manually at any time (this resets that session's timer).
- **Auto-advance** is on by default: a finished Pomodoro starts a Short Break, and every 4th Pomodoro starts a Long Break; a finished break starts a new Pomodoro. Untick **Auto-start next session** to select the next session without starting it — press Start when you're ready. Four circles show the Pomodoros completed in the current cycle before a long break.
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
