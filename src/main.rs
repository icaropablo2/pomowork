use eframe::egui::{self, Color32, RichText, ViewportCommand, WindowLevel};
use serde::{Deserialize, Serialize};
use std::{
    fs,
    path::PathBuf,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

const POMODORO_SECONDS: f64 = 25.0 * 60.0;
const SAVE_INTERVAL_MS: u128 = 2_000;
const BUTTON_WIDTH: f32 = 108.0;

fn now_millis() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
}

fn centered_buttons<R>(
    ui: &mut egui::Ui,
    count: usize,
    add_buttons: impl FnOnce(&mut egui::Ui) -> R,
) -> R {
    let spacing = ui.spacing().item_spacing.x;
    let row_width = count as f32 * BUTTON_WIDTH + spacing * count.saturating_sub(1) as f32;
    let left_pad = ((ui.available_width() - row_width) / 2.0).max(0.0);
    ui.horizontal(|ui| {
        ui.add_space(left_pad);
        add_buttons(ui)
    })
    .inner
}

/// A timer that stores its accumulated time plus the wall-clock time of the
/// current run. This lets it resume correctly after the app is closed.
#[derive(Clone, Debug, Serialize, Deserialize)]
struct TimerState {
    running: bool,
    elapsed_secs: f64,
    started_at: Option<u128>,
}

impl Default for TimerState {
    fn default() -> Self {
        Self {
            running: false,
            elapsed_secs: 0.0,
            started_at: None,
        }
    }
}

impl TimerState {
    fn elapsed(&self, now: u128) -> f64 {
        let mut elapsed = self.elapsed_secs;
        if let Some(started_at) = self.started_at {
            elapsed += now.saturating_sub(started_at) as f64 / 1000.0;
        }
        elapsed
    }

    fn toggle(&mut self, now: u128) {
        if self.running {
            self.elapsed_secs = self.elapsed(now);
            self.started_at = None;
            self.running = false;
        } else {
            self.started_at = Some(now);
            self.running = true;
        }
    }

    fn reset(&mut self) {
        *self = Self::default();
    }

    fn is_paused(&self) -> bool {
        !self.running && self.elapsed_secs > 0.0
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct AppState {
    stopwatch: TimerState,
    pomodoro: TimerState,
    pomodoro_done: bool,
    always_on_top: bool,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            stopwatch: TimerState::default(),
            pomodoro: TimerState::default(),
            pomodoro_done: false,
            always_on_top: true,
        }
    }
}

fn state_file() -> Option<PathBuf> {
    dirs::data_dir().map(|dir| dir.join("pomowork").join("state.json"))
}

fn load_state() -> AppState {
    let Some(path) = state_file() else {
        return AppState::default();
    };

    fs::read_to_string(path)
        .ok()
        .and_then(|json| serde_json::from_str::<AppState>(&json).ok())
        .unwrap_or_default()
}

fn save_state(state: &AppState) {
    let Some(path) = state_file() else {
        return;
    };
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    if let Ok(json) = serde_json::to_string_pretty(state) {
        let _ = fs::write(path, json);
    }
}

struct PomoworkApp {
    state: AppState,
    last_saved: u128,
    applied_window_level: bool,
}

impl PomoworkApp {
    fn new(state: AppState) -> Self {
        Self {
            state,
            last_saved: 0,
            applied_window_level: false,
        }
    }

    fn save(&mut self, now: u128) {
        save_state(&self.state);
        self.last_saved = now;
    }
}

impl eframe::App for PomoworkApp {
    fn logic(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let now = now_millis();

        if !self.applied_window_level {
            let level = if self.state.always_on_top {
                WindowLevel::AlwaysOnTop
            } else {
                WindowLevel::Normal
            };
            ctx.send_viewport_cmd(ViewportCommand::WindowLevel(level));
            self.applied_window_level = true;
        }

        // Finish the pomodoro once 25 minutes have elapsed.
        if self.state.pomodoro.running && self.state.pomodoro.elapsed(now) >= POMODORO_SECONDS {
            self.state.pomodoro.elapsed_secs = POMODORO_SECONDS;
            self.state.pomodoro.started_at = None;
            self.state.pomodoro.running = false;
            self.state.pomodoro_done = true;
            self.save(now);
        }

        // Repaint while a timer is running so the digits keep ticking.
        if self.state.stopwatch.running || self.state.pomodoro.running {
            ctx.request_repaint_after(Duration::from_millis(100));
        }

        // Crash-safe persistence in addition to the save on exit.
        if now.saturating_sub(self.last_saved) >= SAVE_INTERVAL_MS {
            self.save(now);
        }
    }

    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let now = now_millis();
        let ctx = ui.ctx().clone();

        egui::CentralPanel::default().show(ui, |ui| {
            ui.vertical_centered(|ui| {
                ui.add_space(10.0);
                ui.label(RichText::new("POMOWORK").size(20.0).strong());
                ui.add_space(12.0);

                // ---- Stopwatch (top) ----
                ui.group(|ui| {
                    ui.set_min_width(330.0);
                    ui.vertical_centered(|ui| {
                        ui.label(RichText::new("STOPWATCH").size(13.0).weak());
                        ui.add_space(4.0);
                        let elapsed = self.state.stopwatch.elapsed(now);
                        ui.label(
                            RichText::new(format_stopwatch(elapsed))
                                .monospace()
                                .size(38.0)
                                .strong(),
                        );
                        ui.add_space(8.0);
                        let show_reset = self.state.stopwatch.is_paused();
                        centered_buttons(ui, if show_reset { 2 } else { 1 }, |ui| {
                            let toggle = if self.state.stopwatch.running {
                                "Pause"
                            } else {
                                "Start"
                            };
                            if ui
                                .add(
                                    egui::Button::new(toggle)
                                        .min_size(egui::vec2(BUTTON_WIDTH, 0.0)),
                                )
                                .clicked()
                            {
                                self.state.stopwatch.toggle(now);
                                self.save(now);
                            }
                            if show_reset
                                && ui
                                    .add(
                                        egui::Button::new("Reset")
                                            .min_size(egui::vec2(BUTTON_WIDTH, 0.0)),
                                    )
                                    .clicked()
                            {
                                self.state.stopwatch.reset();
                                self.save(now);
                            }
                        });
                    });
                });

                ui.add_space(18.0);

                // ---- Pomodoro (below) ----
                ui.group(|ui| {
                    ui.set_min_width(330.0);
                    ui.vertical_centered(|ui| {
                        ui.label(RichText::new("POMODORO · 25 MIN").size(13.0).weak());
                        ui.add_space(4.0);
                        let remaining =
                            (POMODORO_SECONDS - self.state.pomodoro.elapsed(now)).max(0.0);
                        let color = if self.state.pomodoro_done {
                            Color32::from_rgb(255, 120, 120)
                        } else if self.state.pomodoro.running {
                            Color32::from_rgb(120, 220, 160)
                        } else {
                            Color32::from_gray(230)
                        };
                        ui.label(
                            RichText::new(format_countdown(remaining))
                                .monospace()
                                .size(44.0)
                                .strong()
                                .color(color),
                        );
                        ui.add_space(4.0);
                        if self.state.pomodoro_done {
                            ui.label(
                                RichText::new("Break time!")
                                    .size(14.0)
                                    .color(Color32::from_rgb(255, 120, 120)),
                            );
                        }
                        ui.add_space(8.0);
                        centered_buttons(ui, 1, |ui| {
                            let toggle = if self.state.pomodoro.running {
                                "Pause"
                            } else if self.state.pomodoro_done {
                                "Start new"
                            } else {
                                "Start"
                            };
                            if ui
                                .add(
                                    egui::Button::new(toggle)
                                        .min_size(egui::vec2(BUTTON_WIDTH, 0.0)),
                                )
                                .clicked()
                            {
                                if self.state.pomodoro_done {
                                    self.state.pomodoro.reset();
                                    self.state.pomodoro_done = false;
                                }
                                self.state.pomodoro.toggle(now);
                                self.save(now);
                            }
                        });
                    });
                });

                ui.add_space(14.0);
                let pin = if self.state.always_on_top {
                    "Unpin window"
                } else {
                    "Pin on top"
                };
                if ui.button(pin).clicked() {
                    self.state.always_on_top = !self.state.always_on_top;
                    let level = if self.state.always_on_top {
                        WindowLevel::AlwaysOnTop
                    } else {
                        WindowLevel::Normal
                    };
                    ctx.send_viewport_cmd(ViewportCommand::WindowLevel(level));
                    self.save(now);
                }
            });
        });
    }

    fn on_exit(&mut self) {
        save_state(&self.state);
    }
}

fn format_stopwatch(secs: f64) -> String {
    let total_secs = secs as u64;
    let seconds = total_secs % 60;
    let minutes = (total_secs / 60) % 60;
    let hours = total_secs / 3600;
    format!("{hours:02}:{minutes:02}:{seconds:02}")
}

fn format_countdown(secs: f64) -> String {
    let total = secs.ceil() as u64;
    format!("{:02}:{:02}", total / 60, total % 60)
}

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        centered: true,
        viewport: egui::ViewportBuilder::default()
            .with_title("Pomowork")
            .with_inner_size([390.0, 430.0])
            .with_min_inner_size([340.0, 400.0]),
        ..Default::default()
    };

    eframe::run_native(
        "Pomowork",
        options,
        Box::new(move |_| Ok(Box::new(PomoworkApp::new(load_state())))),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stopwatch_pauses_and_resumes_without_losing_time() {
        let mut timer = TimerState::default();
        timer.toggle(1_000);
        assert_eq!(timer.elapsed(4_000), 3.0);

        timer.toggle(4_000);
        assert!(!timer.running);
        assert_eq!(timer.elapsed(10_000), 3.0);

        timer.toggle(10_000);
        assert_eq!(timer.elapsed(12_500), 5.5);

        timer.reset();
        assert_eq!(timer.elapsed(12_500), 0.0);
    }

    #[test]
    fn reset_is_only_available_when_paused_with_time() {
        let mut timer = TimerState::default();
        assert!(!timer.is_paused());

        timer.toggle(1_000);
        assert!(!timer.is_paused());

        timer.toggle(4_000);
        assert!(timer.is_paused());

        timer.reset();
        assert!(!timer.is_paused());
    }

    #[test]
    fn formats_time() {
        assert_eq!(format_stopwatch(0.0), "00:00:00");
        assert_eq!(format_stopwatch(3_723.456), "01:02:03");
        assert_eq!(format_countdown(1_500.0), "25:00");
        assert_eq!(format_countdown(0.4), "00:01");
        assert_eq!(format_countdown(0.0), "00:00");
    }

    #[test]
    fn state_survives_serialization() {
        let mut state = AppState::default();
        state.stopwatch.toggle(1_000);
        state.always_on_top = false;
        let json = serde_json::to_string(&state).unwrap();
        let restored: AppState = serde_json::from_str(&json).unwrap();
        assert!(restored.stopwatch.running);
        assert_eq!(restored.stopwatch.started_at, Some(1_000));
        assert!(!restored.always_on_top);
    }
}
