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

## Test-driven development (TDD)

TDD is mandatory for all code changes. Follow the red-green-refactor cycle:

1. **Red:** Before writing or modifying any implementation, add or update a test in the `tests` module at the bottom of `src/main.rs` that covers the new or changed behavior. Run `make test` and confirm the new test fails for the expected reason.
2. **Green:** Write the smallest implementation that makes the test pass. Run `make test` again and confirm all tests pass.
3. **Refactor:** Clean up the code while keeping all tests green.

Rules:

- Every implementation must have tests. No new function, branch, behavior, or bug fix is complete without a corresponding test that covers it.
- Every time we create something new or modify existing code, tests covering that change must be written **first**, before the implementation.
- If a change is genuinely untestable in the `tests` module (for example, pure UI wiring), note that explicitly and keep the untested surface as small as possible; all logic around it still needs tests.
- Never skip, weaken, or delete tests just to make a change pass. Fix the implementation instead.
- Run `make test` after every red and green step, and `make check` before declaring a task complete.

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
- Add or update unit tests in the `tests` module at the bottom of `src/main.rs` first (before implementation) for every new or modified behavior, especially timer math, time formatting, and serialization. See the TDD section above.
- Keep the UI simple and vertically centered: stopwatch group on top, pomodoro group below, pin toggle last.
- Prefer small, focused changes. Avoid adding dependencies unless they remove significant complexity.
- Update `README.md` if user-facing behavior, build steps, or state location changes.
- Match the existing style: no comments for obvious code, concise comments for non-obvious invariants (e.g. wall-clock resume behavior).
