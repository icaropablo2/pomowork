use crate::audio::{AudioDirector, AudioPlayer};
use crate::persistence::{save_state, SAVE_INTERVAL_MS};
use crate::state::{AppState, PomodoroMode};
use crate::timer::{format_countdown, format_stopwatch, now_millis, POMODOROS_PER_LONG_BREAK};
use crate::ui::*;
use eframe::egui::{self, Color32, RichText, ViewportCommand};
use std::time::Duration;

pub(crate) struct PomoworkApp {
    state: AppState,
    last_saved: u128,
    applied_window_level: bool,
    todo_input: String,
    audio: AudioPlayer,
    audio_director: AudioDirector,
}

impl PomoworkApp {
    pub(crate) fn new(state: AppState) -> Self {
        Self {
            state,
            last_saved: 0,
            applied_window_level: false,
            todo_input: String::new(),
            audio: AudioPlayer::new(),
            audio_director: AudioDirector::default(),
        }
    }

    fn submit_todo(&mut self, now: u128) {
        let text = std::mem::take(&mut self.todo_input);
        if self.state.add_todo(&text) {
            self.save(now);
        }
    }

    fn save(&mut self, now: u128) {
        save_state(&self.state);
        self.last_saved = now;
    }

    fn maybe_complete_session(&mut self, now: u128) -> bool {
        let mode_seconds = self.state.pomodoro_mode.seconds();
        if self.state.pomodoro.running && self.state.pomodoro.elapsed(now) >= mode_seconds {
            self.state.complete_session();
            return true;
        }
        false
    }

    fn sync_audio(&mut self, session_completed: bool) {
        let mut commands = Vec::new();
        self.audio_director
            .sync(&self.state, session_completed, &mut commands);
        for command in commands {
            self.audio.apply(command);
        }
    }

    fn toggle_always_on_top(&mut self, ctx: &egui::Context, now: u128) {
        self.state.always_on_top = !self.state.always_on_top;
        ctx.send_viewport_cmd(ViewportCommand::WindowLevel(window_level(
            self.state.always_on_top,
        )));
        self.save(now);
    }

    fn config_menu(&mut self, ui: &mut egui::Ui, ctx: &egui::Context, now: u128) {
        if ui
            .checkbox(&mut self.state.sound_enabled, "Sounds")
            .on_hover_text("Brown noise during work sessions, alarm when a session ends")
            .changed()
        {
            self.save(now);
        }
        if ui
            .button(pin_button_label(self.state.always_on_top))
            .clicked()
        {
            self.toggle_always_on_top(ctx, now);
        }
        ui.separator();
        if ui.button("Delete all to-dos").clicked() {
            self.state.clear_todos();
            self.save(now);
            ui.close();
        }
    }
}

impl eframe::App for PomoworkApp {
    fn logic(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let now = now_millis();

        if !self.applied_window_level {
            ctx.send_viewport_cmd(ViewportCommand::WindowLevel(window_level(
                self.state.always_on_top,
            )));
            self.applied_window_level = true;
        }

        // Finish the session once the selected mode's duration has elapsed.
        let session_completed = self.maybe_complete_session(now);
        if session_completed {
            self.save(now);
        }
        self.sync_audio(session_completed);

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
            let window = ui.max_rect();
            let mut content = ui.new_child(
                egui::UiBuilder::new()
                    .max_rect(todo_content_rect(window))
                    .layout(egui::Layout::top_down(egui::Align::Center)),
            );
            egui::ScrollArea::vertical()
                .auto_shrink(false)
                .show(&mut content, |ui| {
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
                            centered_buttons(
                                ui,
                                if show_reset { 2 } else { 1 },
                                BUTTON_WIDTH,
                                |ui| {
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
                                },
                            );
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
                                                        let selected =
                                                            self.state.pomodoro_mode == mode;
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
                                            egui::Frame::new()
                                                .inner_margin(timer_group_margin())
                                                .show(ui, |ui| {
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
                                                            RichText::new(format_countdown(
                                                                remaining,
                                                            ))
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
                                                                .color(Color32::from_rgb(
                                                                    255, 120, 120,
                                                                )),
                                                            );
                                                        } else {
                                                            progress_indicator(
                                                                ui,
                                                                self.state.completed_pomodoros,
                                                                POMODOROS_PER_LONG_BREAK,
                                                            );
                                                        }
                                                    });
                                                });
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

                        // ---- To-do list ----
                        ui.add_space(18.0);
                        let todo_width = todo_section_width(ui.available_width());
                        let todo_height = ui.available_height().max(0.0);
                        ui.allocate_ui_with_layout(
                            egui::vec2(todo_width, todo_height),
                            egui::Layout::top_down(egui::Align::LEFT),
                            |ui| {
                                ui.label(RichText::new("TO-DO LIST").size(13.0).weak());
                                ui.add_space(4.0);
                                let mut toggled = None;
                                let mut removed = None;
                                if self.state.todos.is_empty() {
                                    ui.label(RichText::new("Nothing to do yet").weak().italics());
                                } else {
                                    todo_list_scroll_area(ui.available_height()).show(ui, |ui| {
                                        for (index, item) in self.state.todos.iter().enumerate() {
                                            let response = todo_row(ui, item);
                                            if response.removed {
                                                removed = Some(index);
                                            } else if response.toggled {
                                                toggled = Some(index);
                                            }
                                        }
                                    });
                                }
                                if let Some(index) = removed {
                                    self.state.remove_todo(index);
                                    self.save(now);
                                } else if let Some(index) = toggled {
                                    self.state.toggle_todo(index);
                                    self.save(now);
                                }
                            },
                        );
                    });
                });

            let mut footer = ui.new_child(
                egui::UiBuilder::new()
                    .max_rect(todo_footer_rect(window))
                    .layout(egui::Layout::top_down(egui::Align::Center)),
            );
            footer.vertical_centered(|ui| {
                let input_width = todo_input_width(ui.available_width());
                ui.horizontal(|ui| {
                    ui.add_space(centered_left_pad(ui.available_width(), input_width));
                    let input = ui.add(
                        egui::TextEdit::singleline(&mut self.todo_input)
                            .hint_text("New to-do")
                            .desired_width(input_width)
                            .min_size(egui::vec2(input_width, todo_input_height()))
                            .frame(todo_input_frame())
                            .align(todo_input_align()),
                    );
                    if should_submit_todo(
                        input.lost_focus(),
                        ui.input(|keys| keys.key_pressed(egui::Key::Enter)),
                    ) {
                        self.submit_todo(now);
                        input.request_focus();
                    }
                });
                ui.add_space(todo_input_bottom_margin());
            });

            egui::Area::new(egui::Id::unique("settings_menu"))
                .order(egui::Order::Foreground)
                .anchor(egui::Align2::RIGHT_TOP, egui::vec2(-10.0, 10.0))
                .show(&ctx, |ui| {
                    ui.menu_button(RichText::new("☰").size(18.0), |ui| {
                        self.config_menu(ui, &ctx, now);
                    });
                });
        });
    }

    fn on_exit(&mut self) {
        save_state(&self.state);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::{AppState, PomodoroMode};
    use crate::timer::{POMODORO_SECONDS, SHORT_BREAK_SECONDS};
    use eframe::egui;

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
        assert!(!app.state.pomodoro.running);
        assert!(app.state.pomodoro_done);
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

    #[test]
    fn session_completes_when_elapsed_reaches_the_mode_duration() {
        let mut app = PomoworkApp::new(AppState::default());
        app.state.pomodoro.toggle(1_000);
        let now = 1_000 + (POMODORO_SECONDS as u128 * 1_000);

        assert!(app.maybe_complete_session(now));
        assert_eq!(app.state.pomodoro_mode, PomodoroMode::ShortBreak);
        assert_eq!(app.state.completed_pomodoros, 1);
        assert!(!app.state.pomodoro.running);
        assert!(app.state.pomodoro_done);
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
    fn submitting_blank_input_does_not_add_or_save() {
        let mut app = PomoworkApp::new(AppState::default());
        app.todo_input = "   ".to_owned();

        app.submit_todo(2_000);

        assert!(app.state.todos.is_empty());
        assert!(app.todo_input.is_empty());
        assert_eq!(app.last_saved, 0);
    }

    #[test]
    fn submitting_the_input_adds_a_todo_and_clears_the_field() {
        let mut app = PomoworkApp::new(AppState::default());
        app.todo_input = "  Write report  ".to_owned();

        app.submit_todo(1_000);

        assert_eq!(app.state.todos.len(), 1);
        assert_eq!(app.state.todos[0].text, "Write report");
        assert!(app.todo_input.is_empty());
        assert_eq!(app.last_saved, 1_000);
    }

    #[test]
    fn toggling_the_pin_flips_state_and_saves() {
        let ctx = egui::Context::default();
        let mut app = PomoworkApp::new(AppState::default());

        app.toggle_always_on_top(&ctx, 3_000);
        assert!(!app.state.always_on_top);
        assert_eq!(app.last_saved, 3_000);

        app.toggle_always_on_top(&ctx, 6_000);
        assert!(app.state.always_on_top);
        assert_eq!(app.last_saved, 6_000);
    }
}
