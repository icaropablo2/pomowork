use eframe::egui::{self, Color32, RichText, ViewportCommand, WindowLevel};
use rodio::{source::Source, Decoder, DeviceSinkBuilder, Float, MixerDeviceSink, Player};
use serde::{Deserialize, Serialize};
use std::{
    fs,
    io::Cursor,
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
const TODO_INPUT_BOTTOM_MARGIN: f32 = 80.0;
const TODO_INPUT_HEIGHT: f32 = 44.0;
const TODO_LIST_MIN_HEIGHT: f32 = 84.0;
const TODO_ROW_HEIGHT: f32 = 28.0;
const TODO_ROW_CORNER_RADIUS: f32 = 6.0;
const BROWN_NOISE_MP3: &[u8] = include_bytes!("../assets/ticking-brown.mp3");
const KITCHEN_ALARM_MP3: &[u8] = include_bytes!("../assets/alarm-kitchen.mp3");
const BROWN_NOISE_VOLUME: Float = 0.35;

fn now_millis() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
}

fn centered_left_pad(available_width: f32, content_width: f32) -> f32 {
    ((available_width - content_width) / 2.0).max(0.0)
}

fn centered_buttons<R>(
    ui: &mut egui::Ui,
    count: usize,
    button_width: f32,
    add_buttons: impl FnOnce(&mut egui::Ui) -> R,
) -> R {
    let spacing = ui.spacing().item_spacing.x;
    let row_width = count as f32 * button_width + spacing * count.saturating_sub(1) as f32;
    let left_pad = centered_left_pad(ui.available_width(), row_width);
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

fn pomodoro_section_width(available_width: f32) -> f32 {
    pomodoro_panel_side(available_width) + 2.0 * POMODORO_PANEL_MARGIN
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

fn window_inner_size() -> [f32; 2] {
    [390.0, 800.0]
}

fn window_level(always_on_top: bool) -> WindowLevel {
    if always_on_top {
        WindowLevel::AlwaysOnTop
    } else {
        WindowLevel::Normal
    }
}

fn pin_button_label(always_on_top: bool) -> &'static str {
    if always_on_top {
        "Unpin window"
    } else {
        "Pin on top"
    }
}

fn todo_list_max_height(available_height: f32) -> f32 {
    available_height.max(TODO_LIST_MIN_HEIGHT)
}

fn todo_input_bottom_margin() -> f32 {
    TODO_INPUT_BOTTOM_MARGIN
}

fn todo_input_background() -> Color32 {
    Color32::from_gray(70)
}

fn todo_input_margin() -> egui::Margin {
    egui::Margin::symmetric(14, 10)
}

fn todo_input_align() -> egui::Align2 {
    egui::Align2::CENTER_CENTER
}

fn todo_section_width(available_width: f32) -> f32 {
    pomodoro_section_width(available_width).min(available_width)
}

fn todo_input_width(available_width: f32) -> f32 {
    todo_section_width(available_width)
}

fn todo_input_height() -> f32 {
    TODO_INPUT_HEIGHT
}

fn todo_list_auto_shrink() -> egui::Vec2b {
    egui::Vec2b::new(false, true)
}

fn todo_list_scroll_area(available_height: f32) -> egui::ScrollArea {
    egui::ScrollArea::vertical()
        .max_height(todo_list_max_height(available_height))
        .auto_shrink(todo_list_auto_shrink())
}

fn todo_footer_height() -> f32 {
    todo_input_height() + todo_input_bottom_margin()
}

fn todo_content_rect(window: egui::Rect) -> egui::Rect {
    egui::Rect::from_min_max(
        window.min,
        egui::pos2(window.max.x, window.max.y - todo_footer_height()),
    )
}

fn todo_footer_rect(window: egui::Rect) -> egui::Rect {
    egui::Rect::from_min_max(
        egui::pos2(window.min.x, window.max.y - todo_footer_height()),
        window.max,
    )
}

fn todo_input_shadow() -> egui::Shadow {
    egui::Shadow {
        offset: [0, 4],
        blur: 12,
        spread: 0,
        color: Color32::from_black_alpha(90),
    }
}

fn todo_input_frame() -> egui::Frame {
    egui::Frame::new()
        .fill(todo_input_background())
        .inner_margin(todo_input_margin())
        .corner_radius(12.0)
        .shadow(todo_input_shadow())
}

fn should_submit_todo(lost_focus: bool, enter_pressed: bool) -> bool {
    lost_focus && enter_pressed
}

fn todo_text(item: &TodoItem) -> RichText {
    let text = RichText::new(item.text.as_str());
    if item.done {
        text.strikethrough().weak()
    } else {
        text
    }
}

fn todo_row_background(hovered: bool) -> Color32 {
    if hovered {
        Color32::from_white_alpha(18)
    } else {
        Color32::TRANSPARENT
    }
}

fn todo_row_hovered(ui: &egui::Ui, rect: egui::Rect) -> bool {
    ui.rect_contains_pointer(rect)
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct TodoRowResponse {
    toggled: bool,
    removed: bool,
}

fn todo_row(ui: &mut egui::Ui, item: &TodoItem) -> TodoRowResponse {
    let (_, rect) = ui.allocate_space(egui::vec2(ui.available_width(), TODO_ROW_HEIGHT));
    let hovered = todo_row_hovered(ui, rect);
    let background = todo_row_background(hovered);
    ui.painter()
        .rect_filled(rect, TODO_ROW_CORNER_RADIUS, background);

    let mut row_ui = ui.new_child(
        egui::UiBuilder::new()
            .max_rect(rect)
            .layout(egui::Layout::left_to_right(egui::Align::Center)),
    );
    let mut checked = item.done;
    let toggled = row_ui.checkbox(&mut checked, todo_text(item)).changed();

    let mut removed = false;
    row_ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
        let delete = ui.add(
            egui::Button::new(RichText::new("✕").size(14.0).weak())
                .frame(false)
                .small(),
        );
        if delete.clicked() {
            removed = true;
        }
    });

    TodoRowResponse { toggled, removed }
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

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
struct TodoItem {
    text: String,
    done: bool,
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

fn default_sound_enabled() -> bool {
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
    #[serde(default = "default_sound_enabled")]
    sound_enabled: bool,
    #[serde(default)]
    todos: Vec<TodoItem>,
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
            sound_enabled: default_sound_enabled(),
            todos: Vec::new(),
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

    fn add_todo(&mut self, text: &str) -> bool {
        let text = text.trim();
        if text.is_empty() {
            return false;
        }
        self.todos.push(TodoItem {
            text: text.to_owned(),
            done: false,
        });
        true
    }

    fn toggle_todo(&mut self, index: usize) {
        if let Some(item) = self.todos.get_mut(index) {
            item.done = !item.done;
        }
    }

    fn remove_todo(&mut self, index: usize) -> bool {
        if index < self.todos.len() {
            self.todos.remove(index);
            true
        } else {
            false
        }
    }

    fn clear_todos(&mut self) {
        self.todos.clear();
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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum AudioCommand {
    StartBrownNoise,
    StopBrownNoise,
    PlayKitchenAlarm,
}

fn should_play_brown_noise(state: &AppState) -> bool {
    state.sound_enabled && state.pomodoro_mode == PomodoroMode::Pomodoro && state.pomodoro.running
}

#[derive(Default)]
struct AudioDirector {
    noise_playing: bool,
}

impl AudioDirector {
    fn sync(
        &mut self,
        state: &AppState,
        session_completed: bool,
        commands: &mut Vec<AudioCommand>,
    ) {
        let wanted = should_play_brown_noise(state);
        if wanted && !self.noise_playing {
            commands.push(AudioCommand::StartBrownNoise);
            self.noise_playing = true;
        } else if !wanted && self.noise_playing {
            commands.push(AudioCommand::StopBrownNoise);
            self.noise_playing = false;
        }
        if session_completed && state.sound_enabled {
            commands.push(AudioCommand::PlayKitchenAlarm);
        }
    }
}

struct AudioPlayer {
    // The sink must outlive the player, otherwise playback stops immediately.
    _sink: Option<MixerDeviceSink>,
    player: Option<Player>,
}

impl AudioPlayer {
    fn new() -> Self {
        Self {
            _sink: None,
            player: None,
        }
    }

    fn player(&mut self) -> Option<&Player> {
        if self.player.is_none() {
            let sink = DeviceSinkBuilder::open_default_sink().ok()?;
            self.player = Some(Player::connect_new(sink.mixer()));
            self._sink = Some(sink);
        }
        self.player.as_ref()
    }

    fn apply(&mut self, command: AudioCommand) {
        let Some(player) = self.player() else {
            return;
        };
        match command {
            AudioCommand::StartBrownNoise => {
                player.clear();
                if let Ok(source) = Decoder::new_looped(Cursor::new(BROWN_NOISE_MP3)) {
                    player.append(source.amplify(BROWN_NOISE_VOLUME));
                    player.play();
                }
            }
            AudioCommand::StopBrownNoise => player.clear(),
            AudioCommand::PlayKitchenAlarm => {
                player.clear();
                if let Ok(source) = Decoder::new(Cursor::new(KITCHEN_ALARM_MP3)) {
                    player.append(source);
                    player.play();
                }
            }
        }
    }
}

struct PomoworkApp {
    state: AppState,
    last_saved: u128,
    applied_window_level: bool,
    todo_input: String,
    audio: AudioPlayer,
    audio_director: AudioDirector,
}

impl PomoworkApp {
    fn new(state: AppState) -> Self {
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
            self.state.complete_session(now);
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
            .checkbox(&mut self.state.auto_start_next, "Auto-start next session")
            .changed()
        {
            self.save(now);
        }
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
            .with_inner_size(window_inner_size())
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
        expected.add_todo("Buy milk");
        expected.toggle_todo(0);

        save_state_to(&path, &expected);
        let restored = load_state_from(&path);

        assert!(restored.stopwatch.running);
        assert_eq!(restored.stopwatch.started_at, Some(1_000));
        assert_eq!(restored.pomodoro_mode, PomodoroMode::LongBreak);
        assert_eq!(restored.completed_pomodoros, 4);
        assert!(!restored.auto_start_next);
        assert!(restored.pomodoro_done);
        assert!(!restored.always_on_top);
        assert_eq!(restored.todos, expected.todos);
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

    #[test]
    fn todos_start_empty() {
        assert!(AppState::default().todos.is_empty());
    }

    #[test]
    fn adding_a_todo_appends_an_unchecked_item() {
        let mut state = AppState::default();

        assert!(state.add_todo("  Buy milk  "));
        assert!(state.add_todo("Walk the dog"));

        assert_eq!(
            state.todos,
            vec![
                TodoItem {
                    text: "Buy milk".to_owned(),
                    done: false,
                },
                TodoItem {
                    text: "Walk the dog".to_owned(),
                    done: false,
                },
            ]
        );
    }

    #[test]
    fn blank_todos_are_ignored() {
        let mut state = AppState::default();

        assert!(!state.add_todo(""));
        assert!(!state.add_todo("   \t "));

        assert!(state.todos.is_empty());
    }

    #[test]
    fn toggling_a_todo_changes_its_done_state() {
        let mut state = AppState::default();
        state.add_todo("Buy milk");

        state.toggle_todo(0);
        assert!(state.todos[0].done);

        state.toggle_todo(0);
        assert!(!state.todos[0].done);
    }

    #[test]
    fn toggling_an_unknown_todo_is_a_noop() {
        let mut state = AppState::default();
        state.add_todo("Buy milk");

        state.toggle_todo(7);

        assert_eq!(state.todos.len(), 1);
        assert!(!state.todos[0].done);
    }

    #[test]
    fn removing_a_todo_deletes_it() {
        let mut state = AppState::default();
        state.add_todo("Buy milk");
        state.add_todo("Walk the dog");

        assert!(state.remove_todo(0));

        assert_eq!(state.todos.len(), 1);
        assert_eq!(state.todos[0].text, "Walk the dog");
    }

    #[test]
    fn removing_an_unknown_todo_is_a_noop() {
        let mut state = AppState::default();
        state.add_todo("Buy milk");

        assert!(!state.remove_todo(5));

        assert_eq!(state.todos.len(), 1);
    }

    #[test]
    fn clearing_todos_removes_all_items() {
        let mut state = AppState::default();
        state.add_todo("Buy milk");
        state.add_todo("Walk the dog");

        state.clear_todos();

        assert!(state.todos.is_empty());
    }

    #[test]
    fn completed_todos_render_with_strikethrough() {
        let pending = TodoItem {
            text: "Buy milk".to_owned(),
            done: false,
        };
        let done = TodoItem {
            text: "Buy milk".to_owned(),
            done: true,
        };

        assert_eq!(todo_text(&pending), RichText::new("Buy milk"));
        assert_eq!(
            todo_text(&done),
            RichText::new("Buy milk").strikethrough().weak()
        );
        assert_eq!(todo_text(&done).text(), "Buy milk");
    }

    #[test]
    fn todo_row_background_appears_only_on_hover() {
        let idle = todo_row_background(false);
        let hovered = todo_row_background(true);

        assert_eq!(idle, Color32::TRANSPARENT);
        assert_ne!(hovered, idle);
        assert!(hovered.a() > 0);
        assert!(hovered.r() < 100);
    }

    #[test]
    fn todo_row_hover_detection_covers_the_whole_row() {
        let ctx = egui::Context::default();
        ctx.set_fonts(egui::FontDefinitions::empty());
        let row =
            egui::Rect::from_min_size(egui::pos2(0.0, 0.0), egui::vec2(300.0, TODO_ROW_HEIGHT));
        let over_checkbox = egui::pos2(10.0, TODO_ROW_HEIGHT / 2.0);

        let output = ctx.run_ui(
            egui::RawInput {
                events: vec![egui::Event::PointerMoved(over_checkbox)],
                ..Default::default()
            },
            |ui| {
                let mut checked = false;
                ui.checkbox(&mut checked, "Buy milk");
                assert!(todo_row_hovered(ui, row));
                assert!(!todo_row_hovered(
                    ui,
                    row.translate(egui::vec2(0.0, TODO_ROW_HEIGHT * 2.0))
                ));
            },
        );
        output.drop_without_applying_deltas();
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
    fn submitting_blank_input_does_not_add_or_save() {
        let mut app = PomoworkApp::new(AppState::default());
        app.todo_input = "   ".to_owned();

        app.submit_todo(2_000);

        assert!(app.state.todos.is_empty());
        assert!(app.todo_input.is_empty());
        assert_eq!(app.last_saved, 0);
    }

    #[test]
    fn todos_survive_serialization() {
        let mut state = AppState::default();
        state.add_todo("Buy milk");
        state.toggle_todo(0);
        state.add_todo("Walk the dog");

        let json = serde_json::to_string(&state).unwrap();
        let restored: AppState = serde_json::from_str(&json).unwrap();

        assert_eq!(restored.todos, state.todos);
    }

    #[test]
    fn old_state_without_todos_loads_an_empty_list() {
        let json = serde_json::to_string(&AppState::default()).unwrap();
        let mut value: serde_json::Value = serde_json::from_str(&json).unwrap();
        value.as_object_mut().unwrap().remove("todos");

        let restored: AppState = serde_json::from_value(value).unwrap();

        assert!(restored.todos.is_empty());
    }

    #[test]
    fn window_is_tall_enough_to_show_the_todo_section() {
        let [width, height] = window_inner_size();

        assert!(width >= 340.0);
        assert!(height >= 680.0);
    }

    #[test]
    fn todo_list_uses_the_space_above_the_footer() {
        let content_height = window_inner_size()[1] - todo_footer_height();

        assert_eq!(todo_list_max_height(content_height), content_height);
        assert!(content_height > TODO_LIST_MIN_HEIGHT);
    }

    #[test]
    fn todo_list_keeps_a_usable_minimum_when_space_is_tight() {
        assert_eq!(todo_list_max_height(0.0), TODO_LIST_MIN_HEIGHT);
        assert_eq!(todo_list_max_height(10.0), TODO_LIST_MIN_HEIGHT);
        assert_eq!(todo_list_max_height(120.0), 120.0);
    }

    #[test]
    fn todo_list_shows_every_item_when_they_fit_in_the_available_space() {
        egui::__run_test_ui(|ui| {
            let area = egui::Rect::from_min_size(egui::pos2(0.0, 0.0), egui::vec2(300.0, 300.0));
            let mut list = ui.new_child(
                egui::UiBuilder::new()
                    .max_rect(area)
                    .layout(egui::Layout::top_down(egui::Align::LEFT)),
            );

            let output = todo_list_scroll_area(list.available_height()).show(&mut list, |ui| {
                for number in 0..8 {
                    let item = TodoItem {
                        text: format!("Task {number}"),
                        done: false,
                    };
                    todo_row(ui, &item);
                }
            });

            assert!(
                output.inner_rect.height() >= output.content_size.y,
                "list viewport is {}px but content is {}px",
                output.inner_rect.height(),
                output.content_size.y
            );
        });
    }

    #[test]
    fn todo_list_still_scrolls_when_items_exceed_the_available_space() {
        egui::__run_test_ui(|ui| {
            let area = egui::Rect::from_min_size(egui::pos2(0.0, 0.0), egui::vec2(300.0, 300.0));
            let mut list = ui.new_child(
                egui::UiBuilder::new()
                    .max_rect(area)
                    .layout(egui::Layout::top_down(egui::Align::LEFT)),
            );

            let output = todo_list_scroll_area(list.available_height()).show(&mut list, |ui| {
                for number in 0..20 {
                    let item = TodoItem {
                        text: format!("Task {number}"),
                        done: false,
                    };
                    todo_row(ui, &item);
                }
            });

            assert!(output.content_size.y > output.inner_rect.height());
            assert!(output.inner_rect.height() <= area.height());
        });
    }

    #[test]
    fn centered_content_has_equal_padding_and_never_goes_negative() {
        assert_eq!(centered_left_pad(330.0, 240.0), 45.0);
        assert_eq!(centered_left_pad(240.0, 240.0), 0.0);
        assert_eq!(centered_left_pad(200.0, 240.0), 0.0);
    }

    #[test]
    fn todo_is_submitted_only_when_enter_ends_the_input() {
        assert!(should_submit_todo(true, true));
        assert!(!should_submit_todo(false, true));
        assert!(!should_submit_todo(true, false));
    }

    #[test]
    fn todo_input_has_an_eighty_pixel_bottom_margin() {
        assert_eq!(todo_input_bottom_margin(), 80.0);
        assert!(window_inner_size()[1] >= 680.0 + todo_input_bottom_margin());
    }

    #[test]
    fn todo_input_has_a_grey_background() {
        let color = todo_input_background();

        assert_eq!(color.r(), color.g());
        assert_eq!(color.g(), color.b());
        assert!(color.r() > 0 && color.r() < 255);
    }

    #[test]
    fn todo_input_has_extra_padding() {
        let margin = todo_input_margin();

        assert!(margin.left >= 10);
        assert!(margin.right >= 10);
        assert!(margin.top >= 6);
        assert!(margin.bottom >= 6);
    }

    #[test]
    fn todo_input_text_is_centered() {
        assert!(todo_input_align() == egui::Align2::CENTER_CENTER);
    }

    #[test]
    fn todo_input_matches_the_pomodoro_panel_width() {
        let panel_width = POMODORO_PANEL_SIZE + 2.0 * POMODORO_PANEL_MARGIN;

        assert_eq!(todo_input_width(500.0), panel_width);
        assert_eq!(todo_input_width(300.0), 300.0);
    }

    #[test]
    fn todo_input_has_a_drop_shadow() {
        let shadow = todo_input_shadow();

        assert!(shadow.offset[1] > 0);
        assert!(shadow.blur > 0);
        assert!(shadow.color.a() > 0);
    }

    #[test]
    fn todo_input_frame_combines_grey_fill_padding_and_shadow() {
        let frame = todo_input_frame();

        assert_eq!(frame.fill, todo_input_background());
        assert_eq!(frame.inner_margin, todo_input_margin());
        assert_eq!(frame.shadow.offset, todo_input_shadow().offset);
    }

    #[test]
    fn pomodoro_section_width_includes_its_frame_margins() {
        let panel_width = POMODORO_PANEL_SIZE + 2.0 * POMODORO_PANEL_MARGIN;

        assert_eq!(pomodoro_section_width(500.0), panel_width);
        assert_eq!(pomodoro_section_width(300.0), 300.0);
    }

    #[test]
    fn todo_section_width_matches_the_pomodoro_section() {
        let panel_width = POMODORO_PANEL_SIZE + 2.0 * POMODORO_PANEL_MARGIN;

        assert_eq!(todo_section_width(500.0), panel_width);
        assert_eq!(todo_section_width(390.0), panel_width);
        assert_eq!(todo_input_width(500.0), todo_section_width(500.0));
    }

    #[test]
    fn todo_section_fits_narrow_windows_and_stays_centered() {
        assert_eq!(todo_section_width(10.0), 10.0);
        assert_eq!(todo_section_width(300.0), 300.0);

        let available = 390.0;
        let width = todo_section_width(available);
        let left = centered_left_pad(available, width);
        assert_eq!(left, available - left - width);
    }

    #[test]
    fn todo_section_container_is_centered_and_content_is_left_aligned() {
        let window = egui::Rect::from_min_size(egui::pos2(0.0, 0.0), egui::vec2(390.0, 800.0));

        egui::__run_test_ui(|ui| {
            let mut content = ui.new_child(
                egui::UiBuilder::new()
                    .max_rect(window)
                    .layout(egui::Layout::top_down(egui::Align::Center)),
            );
            let width = todo_section_width(content.available_width());
            let mut container = egui::Rect::NOTHING;
            let mut label_x = Vec::new();

            content.allocate_ui_with_layout(
                egui::vec2(width, 0.0),
                egui::Layout::top_down(egui::Align::LEFT),
                |ui| {
                    container = ui.max_rect();
                    label_x.push(ui.label("TO-DO LIST").rect.min.x);
                    label_x.push(ui.label("Nothing to do yet").rect.min.x);
                },
            );

            assert_eq!(container.width(), width);
            assert!((container.center().x - window.center().x).abs() < 1.0);
            assert!(label_x.iter().all(|x| (*x - container.min.x).abs() < 1.0));
        });
    }

    #[test]
    fn todo_list_scroll_area_reaches_the_container_edge() {
        let window = egui::Rect::from_min_size(egui::pos2(0.0, 0.0), egui::vec2(390.0, 800.0));

        egui::__run_test_ui(|ui| {
            let mut content = ui.new_child(
                egui::UiBuilder::new()
                    .max_rect(window)
                    .layout(egui::Layout::top_down(egui::Align::Center)),
            );
            let width = todo_section_width(content.available_width());
            let mut container_width = 0.0;
            let mut final_scroll_width = 0.0;

            content.allocate_ui_with_layout(
                egui::vec2(width, 0.0),
                egui::Layout::top_down(egui::Align::LEFT),
                |ui| {
                    container_width = ui.max_rect().width();
                    let output = todo_list_scroll_area(300.0).show(ui, |ui| {
                        for text in ["Buy milk", "Walk the dog", "Write report"] {
                            let mut checked = false;
                            ui.checkbox(&mut checked, text);
                        }
                    });

                    // A vertical ScrollArea shrinks to content width when auto_shrink.x is set.
                    let auto_shrink = todo_list_auto_shrink();
                    final_scroll_width = if auto_shrink.x {
                        output.content_size.x
                    } else {
                        output.inner_rect.width()
                    };
                },
            );

            assert!(
                (final_scroll_width - container_width).abs() < 1.0,
                "scroll area is {}px wide, container is {}px wide",
                final_scroll_width,
                container_width
            );
        });
    }

    #[test]
    fn todo_input_footer_is_anchored_to_the_bottom_of_the_window() {
        let window = egui::Rect::from_min_size(egui::pos2(0.0, 0.0), egui::vec2(390.0, 800.0));

        let footer = todo_footer_rect(window);
        let content = todo_content_rect(window);

        assert_eq!(footer.max, window.max);
        assert_eq!(footer.height(), todo_footer_height());
        assert_eq!(content.max.y, footer.min.y);
        assert_eq!(content.min, window.min);
    }

    #[test]
    fn sounds_are_enabled_by_default_and_persisted() {
        let value = serde_json::to_value(AppState::default()).unwrap();

        assert_eq!(
            value.get("sound_enabled").and_then(|v| v.as_bool()),
            Some(true)
        );
    }

    #[test]
    fn old_state_without_sound_enabled_starts_with_sounds_on() {
        let mut object = serde_json::to_value(AppState::default())
            .unwrap()
            .as_object()
            .unwrap()
            .clone();
        object.remove("sound_enabled");

        let restored: AppState = serde_json::from_value(serde_json::Value::Object(object)).unwrap();
        let restored_value = serde_json::to_value(restored).unwrap();

        assert_eq!(
            restored_value
                .get("sound_enabled")
                .and_then(|v| v.as_bool()),
            Some(true)
        );
    }

    #[test]
    fn brown_noise_is_wanted_only_during_a_running_work_session() {
        let mut state = AppState::default();
        assert!(!should_play_brown_noise(&state));

        state.pomodoro.toggle(1_000);
        assert!(should_play_brown_noise(&state));

        state.pomodoro.toggle(4_000);
        assert!(!should_play_brown_noise(&state));

        state.pomodoro.toggle(10_000);
        state.sound_enabled = false;
        assert!(!should_play_brown_noise(&state));

        state.sound_enabled = true;
        state.select_pomodoro_mode(PomodoroMode::ShortBreak);
        assert!(!should_play_brown_noise(&state));

        state.select_pomodoro_mode(PomodoroMode::LongBreak);
        assert!(!should_play_brown_noise(&state));

        state.select_pomodoro_mode(PomodoroMode::Pomodoro);
        state.pomodoro.toggle(20_000);
        assert!(should_play_brown_noise(&state));
    }

    #[test]
    fn director_starts_and_stops_brown_noise_on_session_transitions() {
        let mut director = AudioDirector::default();
        let mut state = AppState::default();
        let mut commands = Vec::new();

        director.sync(&state, false, &mut commands);
        assert!(commands.is_empty());

        state.pomodoro.toggle(1_000);
        director.sync(&state, false, &mut commands);
        assert_eq!(commands, vec![AudioCommand::StartBrownNoise]);

        commands.clear();
        director.sync(&state, false, &mut commands);
        assert!(commands.is_empty(), "start must not repeat every frame");

        state.pomodoro.toggle(4_000);
        director.sync(&state, false, &mut commands);
        assert_eq!(commands, vec![AudioCommand::StopBrownNoise]);

        commands.clear();
        director.sync(&state, false, &mut commands);
        assert!(commands.is_empty(), "stop must not repeat every frame");
    }

    #[test]
    fn director_reacts_to_the_sound_toggle_during_a_work_session() {
        let mut director = AudioDirector::default();
        let mut state = AppState::default();
        state.pomodoro.toggle(1_000);
        let mut commands = Vec::new();

        director.sync(&state, false, &mut commands);
        assert_eq!(commands, vec![AudioCommand::StartBrownNoise]);

        commands.clear();
        state.sound_enabled = false;
        director.sync(&state, false, &mut commands);
        assert_eq!(commands, vec![AudioCommand::StopBrownNoise]);

        commands.clear();
        state.sound_enabled = true;
        director.sync(&state, false, &mut commands);
        assert_eq!(commands, vec![AudioCommand::StartBrownNoise]);
    }

    #[test]
    fn director_stops_noise_before_playing_the_alarm() {
        let mut director = AudioDirector::default();
        let mut state = AppState::default();
        state.pomodoro.toggle(1_000);
        let mut commands = Vec::new();

        director.sync(&state, false, &mut commands);
        assert_eq!(commands, vec![AudioCommand::StartBrownNoise]);

        commands.clear();
        state.complete_session(2_000);
        director.sync(&state, true, &mut commands);

        assert_eq!(
            commands,
            vec![AudioCommand::StopBrownNoise, AudioCommand::PlayKitchenAlarm]
        );
    }

    #[test]
    fn director_plays_the_alarm_when_any_session_completes() {
        let mut work = AppState::default();
        work.pomodoro.toggle(1_000);
        work.complete_session(2_000);
        let mut work_commands = Vec::new();
        AudioDirector::default().sync(&work, true, &mut work_commands);
        assert_eq!(work_commands, vec![AudioCommand::PlayKitchenAlarm]);

        let mut break_state = AppState {
            pomodoro_mode: PomodoroMode::ShortBreak,
            ..Default::default()
        };
        break_state.pomodoro.toggle(1_000);
        break_state.complete_session(2_000);
        let mut break_commands = Vec::new();
        AudioDirector::default().sync(&break_state, true, &mut break_commands);
        assert_eq!(
            break_commands,
            vec![
                AudioCommand::StartBrownNoise,
                AudioCommand::PlayKitchenAlarm
            ]
        );
    }

    #[test]
    fn muted_sessions_do_not_play_the_alarm() {
        let mut state = AppState {
            sound_enabled: false,
            ..Default::default()
        };
        state.pomodoro.toggle(1_000);
        state.complete_session(2_000);
        let mut commands = Vec::new();

        AudioDirector::default().sync(&state, true, &mut commands);

        assert!(commands.is_empty());
    }

    #[test]
    fn embedded_sounds_are_decodable_audio() {
        use rodio::{source::Source, Decoder};
        use std::io::Cursor;

        for bytes in [BROWN_NOISE_MP3, KITCHEN_ALARM_MP3] {
            assert!(!bytes.is_empty());
            let decoder = Decoder::new(Cursor::new(bytes)).unwrap();
            assert!(decoder.channels().get() > 0);
            assert!(decoder.sample_rate().get() > 0);
            assert_eq!(decoder.take(1_024).count(), 1_024);
        }
    }

    #[test]
    fn window_level_follows_the_pin_state() {
        assert_eq!(window_level(true), WindowLevel::AlwaysOnTop);
        assert_eq!(window_level(false), WindowLevel::Normal);
    }

    #[test]
    fn pin_button_label_follows_the_pin_state() {
        assert_eq!(pin_button_label(true), "Unpin window");
        assert_eq!(pin_button_label(false), "Pin on top");
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
