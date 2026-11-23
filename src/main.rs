use eframe::egui::{self, Color32, RichText, ViewportCommand, WindowLevel};
use serde::{Deserialize, Serialize};
use std::{
    fs,
    path::{Path, PathBuf},
    time::{Duration, SystemTime, UNIX_EPOCH},
};

const POMODORO_SECONDS: f64 = 25.0 * 60.0;
const SHORT_BREAK_SECONDS: f64 = 5.0 * 60.0;
const LONG_BREAK_SECONDS: f64 = 15.0 * 60.0;
const POMODOROS_PER_LONG_BREAK: u32 = 4;
const SAVE_INTERVAL_MS: u128 = 2_000;
const BUTTON_WIDTH: f32 = 108.0;
const MODE_BUTTON_WIDTH: f32 = 96.0;
const POMODORO_PANEL_SIZE: f32 = 320.0;
const POMODORO_PANEL_MARGIN: f32 = 14.0;
const POMODORO_TOP_PADDING: f32 = 8.0;
const PROGRESS_CIRCLE_SIZE: f32 = 8.0;
const BUTTON_PADDING: f32 = 8.0;
const BUTTON_BASE_HEIGHT: f32 = 26.0;
const TIMER_GROUP_PADDING: i8 = 32;

fn now_millis() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
}

fn centered_buttons<R>(
    ui: &mut egui::Ui,
    count: usize,
    button_width: f32,
    add_buttons: impl FnOnce(&mut egui::Ui) -> R,
) -> R {
    let spacing = ui.spacing().item_spacing.x;
    let row_width = count as f32 * button_width + spacing * count.saturating_sub(1) as f32;
    let left_pad = ((ui.available_width() - row_width) / 2.0).max(0.0);
    ui.horizontal(|ui| {
        ui.add_space(left_pad);
        add_buttons(ui)
    })
    .inner
}

fn pomodoro_panel_side(available_width: f32) -> f32 {
    (available_width - 2.0 * POMODORO_PANEL_MARGIN).clamp(0.0, POMODORO_PANEL_SIZE)
}

fn pomodoro_top_space(side: f32) -> f32 {
    POMODORO_TOP_PADDING.min(side)
}

fn start_button_size() -> egui::Vec2 {
    egui::vec2(
        BUTTON_WIDTH + 2.0 * BUTTON_PADDING,
        BUTTON_BASE_HEIGHT + 2.0 * BUTTON_PADDING,
    )
}

fn timer_group_margin() -> egui::Margin {
    egui::Margin::symmetric(0, TIMER_GROUP_PADDING)
}

fn progress_row_width(count: usize, circle_size: f32, spacing: f32) -> f32 {
    count as f32 * circle_size + spacing * count.saturating_sub(1) as f32
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum CircleState {
    Filled,
    Empty,
}

fn progress_circles(completed: u32, total: u32) -> Vec<CircleState> {
    if total == 0 {
        return Vec::new();
    }
    let filled = completed % total;
    (0..total)
        .map(|index| {
            if index < filled {
                CircleState::Filled
            } else {
                CircleState::Empty
            }
        })
        .collect()
}

fn progress_indicator(ui: &mut egui::Ui, completed: u32, total: u32) {
    ui.horizontal(|ui| {
        let states = progress_circles(completed, total);
        let spacing = ui.spacing().item_spacing.x;
        let row_width = progress_row_width(states.len(), PROGRESS_CIRCLE_SIZE, spacing);
        ui.add_space(((ui.available_width() - row_width) / 2.0).max(0.0));
        let active_fill = ui.visuals().selection.bg_fill;
        for state in states {
            let (rect, _) = ui.allocate_exact_size(
                egui::vec2(PROGRESS_CIRCLE_SIZE, PROGRESS_CIRCLE_SIZE),
                egui::Sense::hover(),
            );
            let center = rect.center();
            let radius = PROGRESS_CIRCLE_SIZE / 2.0;
            match state {
                CircleState::Filled => {
                    ui.painter().circle_filled(center, radius, active_fill);
                }
                CircleState::Empty => {
                    ui.painter().circle_stroke(
                        center,
                        radius - 1.0,
                        egui::Stroke::new(1.5, Color32::from_gray(100)),
                    );
                }
            }
        }
    });
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

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
enum PomodoroMode {
    #[default]
    Pomodoro,
    ShortBreak,
    LongBreak,
}

impl PomodoroMode {
    const ALL: [Self; 3] = [Self::Pomodoro, Self::ShortBreak, Self::LongBreak];

    fn label(self) -> &'static str {
        match self {
            Self::Pomodoro => "Pomodoro",
            Self::ShortBreak => "Short Break",
            Self::LongBreak => "Long Break",
        }
    }

    fn seconds(self) -> f64 {
        match self {
            Self::Pomodoro => POMODORO_SECONDS,
            Self::ShortBreak => SHORT_BREAK_SECONDS,
            Self::LongBreak => LONG_BREAK_SECONDS,
        }
    }
}

fn default_auto_start_next() -> bool {
    true
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct AppState {
    stopwatch: TimerState,
    pomodoro: TimerState,
    #[serde(default)]
    pomodoro_mode: PomodoroMode,
    #[serde(default)]
    completed_pomodoros: u32,
    #[serde(default = "default_auto_start_next")]
    auto_start_next: bool,
    pomodoro_done: bool,
    always_on_top: bool,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            stopwatch: TimerState::default(),
            pomodoro: TimerState::default(),
            pomodoro_mode: PomodoroMode::default(),
            completed_pomodoros: 0,
            auto_start_next: default_auto_start_next(),
            pomodoro_done: false,
            always_on_top: true,
        }
    }
}

impl AppState {
    fn select_pomodoro_mode(&mut self, mode: PomodoroMode) {
        if self.pomodoro_mode == mode {
            return;
        }
        self.pomodoro_mode = mode;
        self.pomodoro.reset();
        self.pomodoro_done = false;
    }

    fn next_mode(&self) -> PomodoroMode {
        match self.pomodoro_mode {
            PomodoroMode::Pomodoro => {
                if (self.completed_pomodoros + 1).is_multiple_of(POMODOROS_PER_LONG_BREAK) {
                    PomodoroMode::LongBreak
                } else {
                    PomodoroMode::ShortBreak
                }
            }
            PomodoroMode::ShortBreak | PomodoroMode::LongBreak => PomodoroMode::Pomodoro,
        }
    }

    fn complete_session(&mut self, now: u128) {
        let next = self.next_mode();
        if self.pomodoro_mode == PomodoroMode::Pomodoro {
            self.completed_pomodoros += 1;
        }
        self.pomodoro_mode = next;
        self.pomodoro.reset();
        self.pomodoro_done = !self.auto_start_next;
        if self.auto_start_next {
            self.pomodoro.toggle(now);
        }
    }
}

fn state_file() -> Option<PathBuf> {
    dirs::data_dir().map(|dir| dir.join("pomowork").join("state.json"))
}

fn load_state_from(path: &Path) -> AppState {
    fs::read_to_string(path)
        .ok()
        .and_then(|json| serde_json::from_str::<AppState>(&json).ok())
        .unwrap_or_default()
}

fn load_state() -> AppState {
    state_file().map_or_else(AppState::default, |path| load_state_from(&path))
}

fn save_state_to(path: &Path, state: &AppState) {
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    if let Ok(json) = serde_json::to_string_pretty(state) {
        let _ = fs::write(path, json);
    }
}

fn save_state(state: &AppState) {
    if let Some(path) = state_file() {
        save_state_to(&path, state);
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

    fn maybe_complete_session(&mut self, now: u128) -> bool {
        let mode_seconds = self.state.pomodoro_mode.seconds();
        if self.state.pomodoro.running && self.state.pomodoro.elapsed(now) >= mode_seconds {
            self.state.complete_session(now);
            return true;
        }
        false
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

        // Finish the session once the selected mode's duration has elapsed.
        if self.maybe_complete_session(now) {
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
                ui.vertical_centered(|ui| {
                    ui.set_min_width(330.0);
                    ui.label(RichText::new("TOTAL WORKED TODAY").size(13.0).weak());
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
                    centered_buttons(ui, if show_reset { 2 } else { 1 }, BUTTON_WIDTH, |ui| {
                        let toggle = if self.state.stopwatch.running {
                            "Pause"
                        } else {
                            "Start"
                        };
                        if ui
                            .add(egui::Button::new(toggle).min_size(egui::vec2(BUTTON_WIDTH, 0.0)))
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

                ui.add_space(18.0);

                // ---- Pomodoro (below) ----
                ui.vertical_centered(|ui| {
                    let side = pomodoro_panel_side(ui.available_width());
                    let active_fill = ui.visuals().selection.bg_fill;
                    egui::Frame::new()
                        .fill(Color32::from_rgb(18, 18, 22))
                        .inner_margin(POMODORO_PANEL_MARGIN)
                        .corner_radius(12.0)
                        .show(ui, |ui| {
                            ui.allocate_ui_with_layout(
                                egui::vec2(side, side),
                                egui::Layout::top_down(egui::Align::Center),
                                |ui| {
                                    ui.add_space(pomodoro_top_space(side));
                                    centered_buttons(
                                        ui,
                                        PomodoroMode::ALL.len(),
                                        MODE_BUTTON_WIDTH,
                                        |ui| {
                                            for mode in PomodoroMode::ALL {
                                                let selected = self.state.pomodoro_mode == mode;
                                                if ui
                                                    .add(
                                                        egui::Button::new(mode.label())
                                                            .selected(selected)
                                                            .min_size(egui::vec2(
                                                                MODE_BUTTON_WIDTH,
                                                                0.0,
                                                            )),
                                                    )
                                                    .clicked()
                                                {
                                                    self.state.select_pomodoro_mode(mode);
                                                    self.save(now);
                                                }
                                            }
                                        },
                                    );
                                    egui::Frame::new().inner_margin(timer_group_margin()).show(
                                        ui,
                                        |ui| {
                                            ui.vertical_centered(|ui| {
                                                let mode = self.state.pomodoro_mode;
                                                let remaining = (mode.seconds()
                                                    - self.state.pomodoro.elapsed(now))
                                                .max(0.0);
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
                                                        RichText::new(format!(
                                                            "{} ready · press Start",
                                                            mode.label()
                                                        ))
                                                        .size(14.0)
                                                        .color(Color32::from_rgb(255, 120, 120)),
                                                    );
                                                } else {
                                                    progress_indicator(
                                                        ui,
                                                        self.state.completed_pomodoros,
                                                        POMODOROS_PER_LONG_BREAK,
                                                    );
                                                }
                                            });
                                        },
                                    );
                                    let button_size = start_button_size();
                                    centered_buttons(ui, 1, button_size.x, |ui| {
                                        let toggle = if self.state.pomodoro.running {
                                            "Pause"
                                        } else {
                                            "Start"
                                        };
                                        if ui
                                            .add(
                                                egui::Button::new(toggle)
                                                    .fill(active_fill)
                                                    .min_size(button_size),
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
                                },
                            );
                        });
                });

                ui.add_space(10.0);
                if ui
                    .checkbox(&mut self.state.auto_start_next, "Auto-start next session")
                    .changed()
                {
                    self.save(now);
                }

                ui.add_space(10.0);
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
            .with_inner_size([390.0, 500.0])
            .with_min_inner_size([340.0, 460.0]),
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
    fn progress_circles_track_the_current_pomodoro_cycle() {
        assert_eq!(
            progress_circles(0, POMODOROS_PER_LONG_BREAK),
            vec![CircleState::Empty; POMODOROS_PER_LONG_BREAK as usize]
        );
        assert_eq!(
            progress_circles(2, POMODOROS_PER_LONG_BREAK),
            vec![
                CircleState::Filled,
                CircleState::Filled,
                CircleState::Empty,
                CircleState::Empty,
            ]
        );
        assert_eq!(
            progress_circles(POMODOROS_PER_LONG_BREAK, POMODOROS_PER_LONG_BREAK),
            vec![CircleState::Empty; POMODOROS_PER_LONG_BREAK as usize]
        );
        assert_eq!(
            progress_circles(POMODOROS_PER_LONG_BREAK + 1, POMODOROS_PER_LONG_BREAK),
            vec![
                CircleState::Filled,
                CircleState::Empty,
                CircleState::Empty,
                CircleState::Empty,
            ]
        );
    }

    #[test]
    fn progress_circles_handle_an_empty_cycle() {
        assert!(progress_circles(3, 0).is_empty());
    }

    #[test]
    fn pomodoro_panel_side_is_a_square_that_fits_the_available_width() {
        assert_eq!(pomodoro_panel_side(500.0), POMODORO_PANEL_SIZE);
        assert_eq!(
            pomodoro_panel_side(300.0),
            300.0 - 2.0 * POMODORO_PANEL_MARGIN
        );
        assert_eq!(pomodoro_panel_side(10.0), 0.0);
    }

    #[test]
    fn pomodoro_content_is_top_aligned_with_a_small_padding() {
        assert_eq!(pomodoro_top_space(320.0), POMODORO_TOP_PADDING);
        assert_eq!(pomodoro_top_space(2.0), 2.0);
    }

    #[test]
    fn progress_circles_are_small_and_fit_in_the_panel() {
        let row = progress_row_width(POMODOROS_PER_LONG_BREAK as usize, PROGRESS_CIRCLE_SIZE, 8.0);
        let previous_row = progress_row_width(POMODOROS_PER_LONG_BREAK as usize, 12.0, 8.0);
        assert!(row > 0.0 && row < previous_row);
        assert!(row < POMODORO_PANEL_SIZE / 2.0);
    }

    #[test]
    fn start_button_has_padding_around_its_label() {
        assert_eq!(start_button_size().x, BUTTON_WIDTH + 2.0 * BUTTON_PADDING);
        assert_eq!(
            start_button_size().y,
            BUTTON_BASE_HEIGHT + 2.0 * BUTTON_PADDING
        );
        assert!(start_button_size().y > BUTTON_BASE_HEIGHT);
    }

    #[test]
    fn timer_and_circles_have_vertical_breathing_room() {
        let margin = timer_group_margin();
        assert_eq!(margin.top, 32);
        assert_eq!(margin.bottom, 32);
        assert_eq!(margin.left, 0);
        assert_eq!(margin.right, 0);
    }

    #[test]
    fn state_survives_serialization() {
        let mut state = AppState::default();
        state.stopwatch.toggle(1_000);
        state.select_pomodoro_mode(PomodoroMode::LongBreak);
        state.completed_pomodoros = 4;
        state.auto_start_next = false;
        state.always_on_top = false;
        let json = serde_json::to_string(&state).unwrap();
        let restored: AppState = serde_json::from_str(&json).unwrap();
        assert!(restored.stopwatch.running);
        assert_eq!(restored.stopwatch.started_at, Some(1_000));
        assert_eq!(restored.pomodoro_mode, PomodoroMode::LongBreak);
        assert_eq!(restored.completed_pomodoros, 4);
        assert!(!restored.auto_start_next);
        assert!(!restored.always_on_top);
    }

    #[test]
    fn old_state_uses_new_field_defaults() {
        let json = serde_json::to_string(&AppState::default()).unwrap();
        let mut value: serde_json::Value = serde_json::from_str(&json).unwrap();
        let object = value.as_object_mut().unwrap();
        object.remove("pomodoro_mode");
        object.remove("completed_pomodoros");
        object.remove("auto_start_next");
        let restored: AppState = serde_json::from_value(value).unwrap();
        assert_eq!(restored.pomodoro_mode, PomodoroMode::Pomodoro);
        assert_eq!(restored.completed_pomodoros, 0);
        assert!(restored.auto_start_next);
    }

    #[test]
    fn session_modes_have_expected_durations() {
        assert_eq!(PomodoroMode::Pomodoro.seconds(), 25.0 * 60.0);
        assert_eq!(PomodoroMode::ShortBreak.seconds(), 5.0 * 60.0);
        assert_eq!(PomodoroMode::LongBreak.seconds(), 15.0 * 60.0);
        assert_eq!(PomodoroMode::default(), PomodoroMode::Pomodoro);
    }

    #[test]
    fn switching_mode_resets_the_timer() {
        let mut state = AppState::default();
        state.pomodoro.toggle(1_000);
        state.pomodoro_done = true;

        state.select_pomodoro_mode(PomodoroMode::ShortBreak);
        assert_eq!(state.pomodoro_mode, PomodoroMode::ShortBreak);
        assert_eq!(state.pomodoro.elapsed(4_000), 0.0);
        assert!(!state.pomodoro.running);
        assert!(!state.pomodoro_done);

        state.pomodoro.toggle(4_000);
        state.select_pomodoro_mode(PomodoroMode::ShortBreak);
        assert!(state.pomodoro.running);
    }

    #[test]
    fn long_break_follows_every_fourth_pomodoro() {
        let mut state = AppState::default();
        let mut next_modes = Vec::new();
        for completed in 0..8 {
            state.completed_pomodoros = completed;
            next_modes.push(state.next_mode());
        }
        assert_eq!(
            next_modes,
            vec![
                PomodoroMode::ShortBreak,
                PomodoroMode::ShortBreak,
                PomodoroMode::ShortBreak,
                PomodoroMode::LongBreak,
                PomodoroMode::ShortBreak,
                PomodoroMode::ShortBreak,
                PomodoroMode::ShortBreak,
                PomodoroMode::LongBreak,
            ]
        );

        state.pomodoro_mode = PomodoroMode::LongBreak;
        assert_eq!(state.next_mode(), PomodoroMode::Pomodoro);
    }

    #[test]
    fn completing_a_session_advances_and_starts_next_by_default() {
        let mut state = AppState::default();
        state.pomodoro.toggle(1_000);

        state.complete_session(2_000);

        assert_eq!(state.pomodoro_mode, PomodoroMode::ShortBreak);
        assert_eq!(state.completed_pomodoros, 1);
        assert!(state.pomodoro.running);
        assert_eq!(state.pomodoro.started_at, Some(2_000));
        assert!(!state.pomodoro_done);
    }

    #[test]
    fn completing_a_session_can_wait_for_manual_start() {
        let mut state = AppState {
            auto_start_next: false,
            ..Default::default()
        };
        state.pomodoro.toggle(1_000);

        state.complete_session(2_000);

        assert_eq!(state.pomodoro_mode, PomodoroMode::ShortBreak);
        assert!(!state.pomodoro.running);
        assert!(state.pomodoro_done);
        assert_eq!(state.pomodoro.elapsed(10_000), 0.0);
    }

    #[test]
    fn app_state_defaults_enable_always_on_top_and_auto_start() {
        let state = AppState::default();
        assert!(!state.stopwatch.running);
        assert_eq!(state.stopwatch.elapsed_secs, 0.0);
        assert!(!state.pomodoro.running);
        assert_eq!(state.pomodoro.elapsed_secs, 0.0);
        assert_eq!(state.pomodoro_mode, PomodoroMode::Pomodoro);
        assert_eq!(state.completed_pomodoros, 0);
        assert!(state.auto_start_next);
        assert!(!state.pomodoro_done);
        assert!(state.always_on_top);
    }

    #[test]
    fn modes_have_labels_and_all_lists_every_variant() {
        assert_eq!(PomodoroMode::Pomodoro.label(), "Pomodoro");
        assert_eq!(PomodoroMode::ShortBreak.label(), "Short Break");
        assert_eq!(PomodoroMode::LongBreak.label(), "Long Break");
        assert_eq!(
            PomodoroMode::ALL,
            [
                PomodoroMode::Pomodoro,
                PomodoroMode::ShortBreak,
                PomodoroMode::LongBreak,
            ]
        );
    }

    #[test]
    fn next_mode_after_any_break_is_pomodoro() {
        let mut state = AppState {
            pomodoro_mode: PomodoroMode::ShortBreak,
            ..Default::default()
        };
        assert_eq!(state.next_mode(), PomodoroMode::Pomodoro);

        state.pomodoro_mode = PomodoroMode::LongBreak;
        assert_eq!(state.next_mode(), PomodoroMode::Pomodoro);
    }

    #[test]
    fn completing_a_break_does_not_increment_the_pomodoro_count() {
        let mut state = AppState {
            pomodoro_mode: PomodoroMode::ShortBreak,
            completed_pomodoros: 2,
            ..Default::default()
        };
        state.pomodoro.toggle(1_000);

        state.complete_session(2_000);

        assert_eq!(state.completed_pomodoros, 2);
        assert_eq!(state.pomodoro_mode, PomodoroMode::Pomodoro);
    }

    #[test]
    fn fourth_pomodoro_starts_the_long_break() {
        let mut state = AppState {
            completed_pomodoros: 3,
            ..Default::default()
        };
        state.pomodoro.toggle(1_000);

        state.complete_session(2_000);

        assert_eq!(state.completed_pomodoros, 4);
        assert_eq!(state.pomodoro_mode, PomodoroMode::LongBreak);
        assert!(state.pomodoro.running);
        assert_eq!(state.pomodoro.started_at, Some(2_000));
    }

    #[test]
    fn elapsed_is_clamped_when_the_clock_moves_backwards() {
        let timer = TimerState {
            running: true,
            elapsed_secs: 5.0,
            started_at: Some(10_000),
        };
        assert_eq!(timer.elapsed(9_000), 5.0);
        assert_eq!(timer.elapsed(10_000), 5.0);
    }

    #[test]
    fn a_resumed_timer_is_not_paused() {
        let mut timer = TimerState::default();
        timer.toggle(1_000);
        timer.toggle(4_000);
        timer.toggle(10_000);

        assert!(timer.running);
        assert!(timer.elapsed_secs > 0.0);
        assert!(!timer.is_paused());
    }

    #[test]
    fn stopwatch_format_handles_hours_and_boundaries() {
        assert_eq!(format_stopwatch(59.999), "00:00:59");
        assert_eq!(format_stopwatch(60.0), "00:01:00");
        assert_eq!(format_stopwatch(3_600.0), "01:00:00");
        assert_eq!(format_stopwatch(359_999.0), "99:59:59");
        assert_eq!(format_stopwatch(360_000.0), "100:00:00");
    }

    #[test]
    fn countdown_format_rounds_up_and_handles_long_sessions() {
        assert_eq!(format_countdown(59.1), "01:00");
        assert_eq!(format_countdown(60.1), "01:01");
        assert_eq!(format_countdown(899.9), "15:00");
        assert_eq!(format_countdown(900.0), "15:00");
        assert_eq!(format_countdown(3_600.0), "60:00");
    }

    #[test]
    fn serialization_preserves_the_pomodoro_timer_and_done_flag() {
        let mut state = AppState::default();
        state.pomodoro.toggle(1_000);
        state.pomodoro_done = true;

        let json = serde_json::to_string(&state).unwrap();
        let restored: AppState = serde_json::from_str(&json).unwrap();

        assert!(restored.pomodoro.running);
        assert_eq!(restored.pomodoro.started_at, Some(1_000));
        assert!(restored.pomodoro_done);
        assert!(restored.always_on_top);
    }

    #[test]
    fn new_app_tracks_saving_and_window_level_separately_from_state() {
        let state = AppState::default();
        let app = PomoworkApp::new(state.clone());

        assert_eq!(app.last_saved, 0);
        assert!(!app.applied_window_level);
        assert!(app.state.always_on_top);
        assert_eq!(app.state.pomodoro_mode, state.pomodoro_mode);
    }

    fn temp_path(name: &str) -> PathBuf {
        std::env::temp_dir().join(format!("pomowork-test-{}-{name}", std::process::id()))
    }

    #[test]
    fn loading_a_missing_state_file_uses_defaults() {
        let path = temp_path("missing.json");
        let _ = fs::remove_file(&path);

        let state = load_state_from(&path);

        assert!(state.always_on_top);
        assert!(state.auto_start_next);
        assert_eq!(state.completed_pomodoros, 0);
        assert_eq!(state.pomodoro_mode, PomodoroMode::Pomodoro);
    }

    #[test]
    fn loading_invalid_json_uses_defaults() {
        let path = temp_path("invalid.json");
        fs::write(&path, "not json").unwrap();

        let state = load_state_from(&path);

        assert!(state.always_on_top);
        assert!(state.auto_start_next);
        assert_eq!(state.pomodoro_mode, PomodoroMode::Pomodoro);
        fs::remove_file(&path).unwrap();
    }

    #[test]
    fn saved_state_loads_back_unchanged() {
        let dir = temp_path("roundtrip-dir");
        let _ = fs::remove_dir_all(&dir);
        let path = dir.join("nested").join("state.json");
        let mut expected = AppState::default();
        expected.stopwatch.toggle(1_000);
        expected.pomodoro_mode = PomodoroMode::LongBreak;
        expected.completed_pomodoros = 4;
        expected.auto_start_next = false;
        expected.pomodoro_done = true;
        expected.always_on_top = false;

        save_state_to(&path, &expected);
        let restored = load_state_from(&path);

        assert!(restored.stopwatch.running);
        assert_eq!(restored.stopwatch.started_at, Some(1_000));
        assert_eq!(restored.pomodoro_mode, PomodoroMode::LongBreak);
        assert_eq!(restored.completed_pomodoros, 4);
        assert!(!restored.auto_start_next);
        assert!(restored.pomodoro_done);
        assert!(!restored.always_on_top);
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn state_missing_older_fields_falls_back_to_defaults_entirely() {
        let state = AppState {
            pomodoro_mode: PomodoroMode::LongBreak,
            ..Default::default()
        };
        let mut value = serde_json::to_value(&state).unwrap();
        let object = value.as_object_mut().unwrap();
        object.remove("pomodoro_done");
        object.remove("always_on_top");
        let path = temp_path("legacy-fields.json");
        fs::write(&path, value.to_string()).unwrap();

        let restored = load_state_from(&path);

        assert_eq!(restored.pomodoro_mode, PomodoroMode::Pomodoro);
        assert!(restored.always_on_top);
        assert!(!restored.pomodoro_done);
        fs::remove_file(&path).unwrap();
    }

    #[test]
    fn session_completes_when_elapsed_reaches_the_mode_duration() {
        let mut app = PomoworkApp::new(AppState::default());
        app.state.pomodoro.toggle(1_000);
        let now = 1_000 + (POMODORO_SECONDS as u128 * 1_000);

        assert!(app.maybe_complete_session(now));
        assert_eq!(app.state.pomodoro_mode, PomodoroMode::ShortBreak);
        assert_eq!(app.state.completed_pomodoros, 1);
    }

    #[test]
    fn session_does_not_complete_before_the_mode_duration() {
        let mut app = PomoworkApp::new(AppState::default());
        app.state.pomodoro.toggle(1_000);
        let now = 1_000 + (POMODORO_SECONDS as u128 * 1_000) - 1;

        assert!(!app.maybe_complete_session(now));
        assert_eq!(app.state.pomodoro_mode, PomodoroMode::Pomodoro);
        assert!(app.state.pomodoro.running);
        assert_eq!(app.state.completed_pomodoros, 0);
    }

    #[test]
    fn session_does_not_complete_if_the_timer_is_not_running() {
        let mut app = PomoworkApp::new(AppState::default());
        app.state.pomodoro.elapsed_secs = POMODORO_SECONDS;

        assert!(!app.maybe_complete_session(10_000));
        assert_eq!(app.state.pomodoro_mode, PomodoroMode::Pomodoro);
        assert_eq!(app.state.completed_pomodoros, 0);
    }

    #[test]
    fn a_break_completes_at_its_own_duration() {
        let mut app = PomoworkApp::new(AppState::default());
        app.state.pomodoro_mode = PomodoroMode::ShortBreak;
        app.state.completed_pomodoros = 2;
        app.state.pomodoro.toggle(1_000);
        let now = 1_000 + (SHORT_BREAK_SECONDS as u128 * 1_000);

        assert!(app.maybe_complete_session(now));
        assert_eq!(app.state.pomodoro_mode, PomodoroMode::Pomodoro);
        assert_eq!(app.state.completed_pomodoros, 2);
    }
}
