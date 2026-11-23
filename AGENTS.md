# AGENTS.md

Instructions for AI agents working in this repository.

## Project overview

`pomowork` is a cross-platform (Windows + Linux) desktop app that combines a stopwatch and a 25-minute Pomodoro timer in a single, centered, always-on-top window. It is written in pure Rust using `eframe`/`egui` (glow/wgpu via winit), with JSON state persistence.

## Layout

```
Cargo.toml       # crate metadata and dependencies
src/main.rs      # entire application: timer logic, persistence, tests, UI
Makefile         # common developer commands (`make help`)
README.md        # user-facing build/run instructions
target/          # build output (gitignored, never edit)
```

There is deliberately no separate module structure yet. Keep new code in `src/main.rs` until it becomes genuinely unwieldy, then split into modules (e.g. `timer.rs`, `persistence.rs`, `ui.rs`).

## Commands

Always use the Makefile targets when they exist:

```bash
make help            # list all targets
make build           # debug build
make release         # optimized build
make run             # cargo run
make run-release     # build + run the release binary
make test            # unit tests
make fmt             # rustfmt
make fmt-check       # check formatting
make clippy          # clippy with -D warnings
make check           # fmt-check + clippy + test (run before finishing)
make windows         # cross-compile to x86_64-pc-windows-gnu
make reset-state     # delete saved session state for a clean start
```

Do not commit `target/`.

## Architecture and important details

- **`TimerState`** stores `running: bool`, `elapsed_secs: f64`, and `started_at: Option<u128>` (Unix milliseconds). Elapsed time is computed as `elapsed_secs + (now - started_at)`, so pausing folds the running segment into `elapsed_secs`. This design makes timers resume correctly across app restarts, even if the app was closed for a long time.
- **`AppState`** holds `stopwatch`, `pomodoro`, `pomodoro_done`, and `always_on_top`. Its `Default` implementation must keep `always_on_top = true`; do not replace it with a derived `Default` (a derived one silently sets the bool to `false`).
- **Persistence**: `dirs::data_dir()/pomowork/state.json`. Saves happen on every button press, every 2 seconds from `logic()` (crash safety), and in `on_exit()`. Treat the on-disk JSON as a stable format: when adding fields, give them sensible defaults so old state files still load (currently a parse failure falls back to `AppState::default()`).
- **eframe 0.37 API**: implement `eframe::App` with `fn logic(&mut self, ctx, frame)` for timers/repaint/persistence and `fn ui(&mut self, ui, frame)` for drawing. There is no `update()` method in this version. `CentralPanel::show` now takes `&mut Ui`.
- `logic()` calls `ctx.request_repaint_after(100ms)` while a timer runs; keep that when adding timers.
- Pomodoro completion is detected in `logic()` using wall-clock time, so a completed session is recognized immediately on relaunch.
- `always_on_top` is applied via `ViewportCommand::WindowLevel(WindowLevel::AlwaysOnTop)` once at startup and toggled by the pin button. Note that some Wayland compositors ignore this.
- The window is centered via `NativeOptions { centered: true, .. }`. Centering is not supported on Wayland by eframe.

## Conventions

- Run `make check` before declaring a task complete. Warnings are errors in this project.
- Add or update unit tests in the `tests` module at the bottom of `src/main.rs` whenever timer math, time formatting, or serialization changes.
- Keep the UI simple and vertically centered: stopwatch group on top, pomodoro group below, pin toggle last.
- Prefer small, focused changes. Avoid adding dependencies unless they remove significant complexity.
- Update `README.md` if user-facing behavior, build steps, or state location changes.
- Match the existing style: no comments for obvious code, concise comments for non-obvious invariants (e.g. wall-clock resume behavior).
