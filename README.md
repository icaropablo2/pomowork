# Pomowork

A small cross-platform desktop app that combines a stopwatch, a Pomodoro timer with three session modes, and a to-do list, built in Rust with [egui/eframe](https://github.com/emilk/egui).

## Features

- **Total worked today** (stopwatch) on top, with no border, with Start/Pause and Reset (shown only once paused with time on the clock). Displays `HH:MM:SS`. Starting any timer session (Pomodoro, Short Break, or Long Break) also starts or resumes the stopwatch if it isn't already running.
- **Timer** below it in a darker, centered square panel with session buttons at the top: **Pomodoro** (25 min), **Short Break** (5 min), and **Long Break** (15 min). The active mode is highlighted, and the Start button uses the same color. You can select any session manually at any time (this resets that session's timer).
- **Auto-advance** selects the next session without starting it: a finished Pomodoro switches to a Short Break, and every 4th Pomodoro switches to a Long Break; a finished break switches back to a new Pomodoro. Press Start when you're ready to begin. Four circles show the Pomodoros completed in the current cycle before a long break.
- **To-do list** below the timer: type into the grey input field pinned to the bottom of the window (80 px above the lower edge, matching the Pomodoro panel's width, with a drop shadow) and press Enter to append an item. Tick an item's checkbox to mark it done; completed items are shown with a strikethrough and can be unticked again. Remove an item inline with its delete button. The list grows to show every item that fits in the available space and only scrolls when the items no longer fit. The rest of the app scrolls if the window gets too small.
- **Focus** is optional: click the ◉/◎ button on a to-do to make it your focus (click it again, or pick another item, to change it). Every completed Pomodoro work session adds a small filled circle to the focused item, and the circles keep accumulating with each session. Breaks don't add circles, and you can run the timer with no item focused. Removing the focused item clears the focus; removing an item above it keeps the focus on the same task.
- **Sounds** are on by default and can be turned off with the **Sounds** checkbox in the ☰ menu: brown noise loops while a Pomodoro (work) session runs, and a kitchen alarm plays once whenever any session finishes. Unticking the box mutes both immediately and stops the noise.
- Window is centered on screen and can be pinned always-on-top (on by default); use **Unpin window** / **Pin on top** in the ☰ menu to change it.
- The **☰ menu** at the top right holds all configuration: **Sounds**, **Unpin window** / **Pin on top**, and **Delete all to-dos**.
- Session state persists to disk, so closing and reopening the app restores both timers (including elapsed time while the app was closed), the to-do list, the focused item, and each item's session circles.

## Build & run

```bash
cargo run --release
```

On Linux you may need system packages for X11/Wayland and graphics:

```bash
# Debian/Ubuntu
sudo apt install libxkbcommon-dev libwayland-dev libx11-dev libxcb1-dev libxrandr-dev \
                 libxi-dev libgl1-mesa-dev libudev-dev libasound2-dev
```

Cross-compiling to Windows from Linux requires the MinGW target and linker (or use a Windows machine/CI):

```bash
rustup target add x86_64-pc-windows-gnu
cargo build --release --target x86_64-pc-windows-gnu
```

State is saved to:

- Linux: `~/.local/share/pomowork/state.json`
- Windows: `%APPDATA%\pomowork\state.json`

## Audio assets

- `ticking-brown.mp3` — `/audios/ticking/ticking-brown.mp3`
- `alarm-kitchen.mp3` — `/audios/alarms/alarm-kitchen.mp3`