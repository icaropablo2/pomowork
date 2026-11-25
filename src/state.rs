use crate::timer::{
    TimerState, LONG_BREAK_SECONDS, POMODOROS_PER_LONG_BREAK, POMODORO_SECONDS, SHORT_BREAK_SECONDS,
};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct TodoItem {
    pub(crate) text: String,
    pub(crate) done: bool,
    #[serde(default)]
    pub(crate) sessions: u32,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) enum PomodoroMode {
    #[default]
    Pomodoro,
    ShortBreak,
    LongBreak,
}

impl PomodoroMode {
    pub(crate) const ALL: [Self; 3] = [Self::Pomodoro, Self::ShortBreak, Self::LongBreak];

    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::Pomodoro => "Pomodoro",
            Self::ShortBreak => "Short Break",
            Self::LongBreak => "Long Break",
        }
    }

    pub(crate) fn seconds(self) -> f64 {
        match self {
            Self::Pomodoro => POMODORO_SECONDS,
            Self::ShortBreak => SHORT_BREAK_SECONDS,
            Self::LongBreak => LONG_BREAK_SECONDS,
        }
    }
}

fn default_sound_enabled() -> bool {
    true
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct AppState {
    pub(crate) stopwatch: TimerState,
    pub(crate) pomodoro: TimerState,
    #[serde(default)]
    pub(crate) pomodoro_mode: PomodoroMode,
    #[serde(default)]
    pub(crate) completed_pomodoros: u32,
    #[serde(default = "default_sound_enabled")]
    pub(crate) sound_enabled: bool,
    #[serde(default)]
    pub(crate) todos: Vec<TodoItem>,
    #[serde(default)]
    pub(crate) focused_todo: Option<usize>,
    pub(crate) pomodoro_done: bool,
    pub(crate) always_on_top: bool,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            stopwatch: TimerState::default(),
            pomodoro: TimerState::default(),
            pomodoro_mode: PomodoroMode::default(),
            completed_pomodoros: 0,
            sound_enabled: default_sound_enabled(),
            todos: Vec::new(),
            focused_todo: None,
            pomodoro_done: false,
            always_on_top: true,
        }
    }
}

impl AppState {
    pub(crate) fn select_pomodoro_mode(&mut self, mode: PomodoroMode) {
        if self.pomodoro_mode == mode {
            return;
        }
        self.pomodoro_mode = mode;
        self.pomodoro.reset();
        self.pomodoro_done = false;
    }

    pub(crate) fn toggle_pomodoro(&mut self, now: u128) {
        let starting = !self.pomodoro.running;
        if self.pomodoro_done {
            self.pomodoro.reset();
            self.pomodoro_done = false;
        }
        self.pomodoro.toggle(now);
        if starting && !self.stopwatch.running {
            self.stopwatch.toggle(now);
        }
    }

    pub(crate) fn next_mode(&self) -> PomodoroMode {
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

    pub(crate) fn add_todo(&mut self, text: &str) -> bool {
        let text = text.trim();
        if text.is_empty() {
            return false;
        }
        self.todos.push(TodoItem {
            text: text.to_owned(),
            done: false,
            sessions: 0,
        });
        true
    }

    pub(crate) fn toggle_todo(&mut self, index: usize) {
        if let Some(item) = self.todos.get_mut(index) {
            item.done = !item.done;
        }
    }

    pub(crate) fn toggle_focus_todo(&mut self, index: usize) {
        if index >= self.todos.len() {
            return;
        }
        self.focused_todo = if self.focused_todo == Some(index) {
            None
        } else {
            Some(index)
        };
    }

    pub(crate) fn remove_todo(&mut self, index: usize) -> bool {
        if index < self.todos.len() {
            self.todos.remove(index);
            self.adjust_focus_after_removal(index);
            true
        } else {
            false
        }
    }

    fn adjust_focus_after_removal(&mut self, removed: usize) {
        self.focused_todo = match self.focused_todo {
            Some(focused) if focused == removed => None,
            Some(focused) if focused > removed => Some(focused - 1),
            other => other,
        };
    }

    pub(crate) fn clear_todos(&mut self) {
        self.todos.clear();
        self.focused_todo = None;
    }

    pub(crate) fn complete_session(&mut self) {
        let next = self.next_mode();
        if self.pomodoro_mode == PomodoroMode::Pomodoro {
            self.completed_pomodoros += 1;
            if let Some(item) = self
                .focused_todo
                .and_then(|index| self.todos.get_mut(index))
            {
                item.sessions += 1;
            }
        }
        self.pomodoro_mode = next;
        self.pomodoro.reset();
        self.pomodoro_done = true;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn app_state_defaults_enable_always_on_top() {
        let state = AppState::default();
        assert!(!state.stopwatch.running);
        assert_eq!(state.stopwatch.elapsed_secs, 0.0);
        assert!(!state.pomodoro.running);
        assert_eq!(state.pomodoro.elapsed_secs, 0.0);
        assert_eq!(state.pomodoro_mode, PomodoroMode::Pomodoro);
        assert_eq!(state.completed_pomodoros, 0);
        assert!(!state.pomodoro_done);
        assert!(state.always_on_top);
    }

    #[test]
    fn session_modes_have_expected_durations() {
        assert_eq!(PomodoroMode::Pomodoro.seconds(), 25.0 * 60.0);
        assert_eq!(PomodoroMode::ShortBreak.seconds(), 5.0 * 60.0);
        assert_eq!(PomodoroMode::LongBreak.seconds(), 15.0 * 60.0);
        assert_eq!(PomodoroMode::default(), PomodoroMode::Pomodoro);
    }

    #[test]
    fn state_survives_serialization() {
        let mut state = AppState::default();
        state.stopwatch.toggle(1_000);
        state.select_pomodoro_mode(PomodoroMode::LongBreak);
        state.completed_pomodoros = 4;
        state.always_on_top = false;
        let json = serde_json::to_string(&state).unwrap();
        let restored: AppState = serde_json::from_str(&json).unwrap();
        assert!(restored.stopwatch.running);
        assert_eq!(restored.stopwatch.started_at, Some(1_000));
        assert_eq!(restored.pomodoro_mode, PomodoroMode::LongBreak);
        assert_eq!(restored.completed_pomodoros, 4);
        assert!(!restored.always_on_top);
    }

    #[test]
    fn old_state_uses_new_field_defaults() {
        let json = serde_json::to_string(&AppState::default()).unwrap();
        let mut value: serde_json::Value = serde_json::from_str(&json).unwrap();
        let object = value.as_object_mut().unwrap();
        object.remove("pomodoro_mode");
        object.remove("completed_pomodoros");
        let restored: AppState = serde_json::from_value(value).unwrap();
        assert_eq!(restored.pomodoro_mode, PomodoroMode::Pomodoro);
        assert_eq!(restored.completed_pomodoros, 0);
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
    fn starting_a_pomodoro_starts_an_idle_stopwatch() {
        let mut state = AppState::default();

        state.toggle_pomodoro(1_000);

        assert!(state.pomodoro.running);
        assert!(state.stopwatch.running);
        assert_eq!(state.stopwatch.started_at, Some(1_000));
        assert_eq!(state.stopwatch.elapsed(1_000), 0.0);
    }

    #[test]
    fn starting_a_pomodoro_resumes_a_paused_stopwatch() {
        let mut state = AppState::default();
        state.stopwatch.toggle(1_000);
        state.stopwatch.toggle(4_000);
        assert_eq!(state.stopwatch.elapsed(4_000), 3.0);

        state.toggle_pomodoro(10_000);

        assert!(state.stopwatch.running);
        assert_eq!(state.stopwatch.elapsed(10_000), 3.0);
        assert_eq!(state.stopwatch.elapsed(12_000), 5.0);
    }

    #[test]
    fn starting_any_mode_starts_an_idle_stopwatch() {
        for mode in PomodoroMode::ALL {
            let mut state = AppState {
                pomodoro_mode: mode,
                ..Default::default()
            };

            state.toggle_pomodoro(1_000);

            assert!(state.pomodoro.running, "{mode:?}");
            assert!(state.stopwatch.running, "{mode:?}");
        }
    }

    #[test]
    fn pausing_a_pomodoro_does_not_start_the_stopwatch() {
        let mut state = AppState::default();
        state.pomodoro.toggle(1_000);

        state.toggle_pomodoro(4_000);

        assert!(!state.pomodoro.running);
        assert!(!state.stopwatch.running);
        assert_eq!(state.stopwatch.started_at, None);
    }

    #[test]
    fn starting_a_pomodoro_leaves_an_already_running_stopwatch_untouched() {
        let mut state = AppState::default();
        state.stopwatch.toggle(1_000);

        state.toggle_pomodoro(2_000);

        assert!(state.stopwatch.running);
        assert_eq!(state.stopwatch.started_at, Some(1_000));
    }

    #[test]
    fn starting_a_completed_session_starts_an_idle_stopwatch() {
        let mut state = AppState {
            pomodoro_done: true,
            ..Default::default()
        };

        state.toggle_pomodoro(1_000);

        assert!(!state.pomodoro_done);
        assert!(state.pomodoro.running);
        assert!(state.stopwatch.running);
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
    fn completing_a_session_switches_to_a_break_without_starting_it() {
        let mut state = AppState::default();
        state.pomodoro.toggle(1_000);

        state.complete_session();

        assert_eq!(state.pomodoro_mode, PomodoroMode::ShortBreak);
        assert_eq!(state.completed_pomodoros, 1);
        assert!(!state.pomodoro.running);
        assert_eq!(state.pomodoro.started_at, None);
        assert_eq!(state.pomodoro.elapsed(10_000), 0.0);
        assert!(state.pomodoro_done);
    }

    #[test]
    fn a_saved_legacy_auto_start_setting_is_ignored() {
        let mut value = serde_json::to_value(AppState::default()).unwrap();
        value
            .as_object_mut()
            .unwrap()
            .insert("auto_start_next".to_owned(), serde_json::json!(true));
        let mut state: AppState = serde_json::from_value(value).unwrap();
        state.pomodoro.toggle(1_000);

        state.complete_session();

        assert!(!state.pomodoro.running);
        assert!(state.pomodoro_done);
    }

    #[test]
    fn auto_start_is_not_part_of_the_saved_state() {
        let value = serde_json::to_value(AppState::default()).unwrap();

        assert!(value.get("auto_start_next").is_none());
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

        state.complete_session();

        assert_eq!(state.completed_pomodoros, 2);
        assert_eq!(state.pomodoro_mode, PomodoroMode::Pomodoro);
        assert!(!state.pomodoro.running);
        assert!(state.pomodoro_done);
    }

    #[test]
    fn fourth_pomodoro_switches_to_the_long_break_without_starting_it() {
        let mut state = AppState {
            completed_pomodoros: 3,
            ..Default::default()
        };
        state.pomodoro.toggle(1_000);

        state.complete_session();

        assert_eq!(state.completed_pomodoros, 4);
        assert_eq!(state.pomodoro_mode, PomodoroMode::LongBreak);
        assert!(!state.pomodoro.running);
        assert!(state.pomodoro_done);
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
                    sessions: 0,
                },
                TodoItem {
                    text: "Walk the dog".to_owned(),
                    done: false,
                    sessions: 0,
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
    fn old_state_without_todos_loads_an_empty_list() {
        let json = serde_json::to_string(&AppState::default()).unwrap();
        let mut value: serde_json::Value = serde_json::from_str(&json).unwrap();
        value.as_object_mut().unwrap().remove("todos");

        let restored: AppState = serde_json::from_value(value).unwrap();

        assert!(restored.todos.is_empty());
    }

    #[test]
    fn focus_is_optional_and_defaults_to_none() {
        let state = AppState::default();

        assert_eq!(state.focused_todo, None);
    }

    #[test]
    fn new_todos_have_no_focus_sessions() {
        let mut state = AppState::default();
        state.add_todo("Buy milk");

        assert_eq!(state.todos[0].sessions, 0);
    }

    #[test]
    fn toggling_focus_selects_then_clears_an_item() {
        let mut state = AppState::default();
        state.add_todo("Buy milk");
        state.add_todo("Walk the dog");

        state.toggle_focus_todo(1);
        assert_eq!(state.focused_todo, Some(1));

        state.toggle_focus_todo(1);
        assert_eq!(state.focused_todo, None);

        state.toggle_focus_todo(0);
        assert_eq!(state.focused_todo, Some(0));
    }

    #[test]
    fn focusing_a_different_item_moves_the_focus() {
        let mut state = AppState::default();
        state.add_todo("Buy milk");
        state.add_todo("Walk the dog");

        state.toggle_focus_todo(0);
        state.toggle_focus_todo(1);

        assert_eq!(state.focused_todo, Some(1));
    }

    #[test]
    fn focusing_an_unknown_todo_is_a_noop() {
        let mut state = AppState::default();
        state.add_todo("Buy milk");

        state.toggle_focus_todo(7);

        assert_eq!(state.focused_todo, None);
        assert_eq!(state.todos.len(), 1);
    }

    #[test]
    fn completing_a_pomodoro_credits_only_the_focused_todo() {
        let mut state = AppState::default();
        state.add_todo("Buy milk");
        state.add_todo("Write report");
        state.toggle_focus_todo(1);

        state.complete_session();

        assert_eq!(state.todos[0].sessions, 0);
        assert_eq!(state.todos[1].sessions, 1);
    }

    #[test]
    fn each_completed_pomodoro_appends_a_focus_circle() {
        let mut state = AppState::default();
        state.add_todo("Write report");
        state.toggle_focus_todo(0);

        for _ in 0..3 {
            state.select_pomodoro_mode(PomodoroMode::Pomodoro);
            state.complete_session();
        }

        assert_eq!(state.todos[0].sessions, 3);
    }

    #[test]
    fn completing_a_break_does_not_credit_the_focused_todo() {
        let mut state = AppState {
            pomodoro_mode: PomodoroMode::ShortBreak,
            ..Default::default()
        };
        state.add_todo("Write report");
        state.toggle_focus_todo(0);

        state.complete_session();

        assert_eq!(state.todos[0].sessions, 0);
    }

    #[test]
    fn running_a_pomodoro_without_focus_does_not_credit_any_todo() {
        let mut state = AppState::default();
        state.add_todo("Buy milk");
        state.add_todo("Walk the dog");

        state.complete_session();

        assert!(state.todos.iter().all(|item| item.sessions == 0));
    }

    #[test]
    fn removing_the_focused_todo_clears_the_focus() {
        let mut state = AppState::default();
        state.add_todo("Buy milk");
        state.add_todo("Walk the dog");
        state.toggle_focus_todo(0);

        assert!(state.remove_todo(0));

        assert_eq!(state.focused_todo, None);
        assert_eq!(state.todos.len(), 1);
    }

    #[test]
    fn removing_an_item_before_the_focus_shifts_the_focus() {
        let mut state = AppState::default();
        state.add_todo("Buy milk");
        state.add_todo("Walk the dog");
        state.add_todo("Write report");
        state.toggle_focus_todo(2);

        assert!(state.remove_todo(0));

        assert_eq!(state.focused_todo, Some(1));
    }

    #[test]
    fn removing_an_item_after_the_focus_keeps_the_focus() {
        let mut state = AppState::default();
        state.add_todo("Buy milk");
        state.add_todo("Walk the dog");
        state.toggle_focus_todo(0);

        assert!(state.remove_todo(1));

        assert_eq!(state.focused_todo, Some(0));
    }

    #[test]
    fn a_shifted_focus_still_credits_the_right_todo_after_removal() {
        let mut state = AppState::default();
        state.add_todo("Buy milk");
        state.add_todo("Write report");
        state.toggle_focus_todo(1);

        assert!(state.remove_todo(0));
        state.complete_session();

        assert_eq!(state.todos[0].sessions, 1);
    }

    #[test]
    fn clearing_todos_clears_the_focus() {
        let mut state = AppState::default();
        state.add_todo("Buy milk");
        state.toggle_focus_todo(0);

        state.clear_todos();

        assert_eq!(state.focused_todo, None);
    }

    #[test]
    fn focus_and_sessions_survive_serialization() {
        let mut state = AppState::default();
        state.add_todo("Buy milk");
        state.add_todo("Write report");
        state.toggle_focus_todo(1);
        state.complete_session();

        let json = serde_json::to_string(&state).unwrap();
        let restored: AppState = serde_json::from_str(&json).unwrap();

        assert_eq!(restored.focused_todo, Some(1));
        assert_eq!(restored.todos[0].sessions, 0);
        assert_eq!(restored.todos[1].sessions, 1);
    }

    #[test]
    fn old_state_without_focus_or_sessions_loads_with_defaults() {
        let mut value = serde_json::to_value(AppState::default()).unwrap();
        let object = value.as_object_mut().unwrap();
        object.remove("focused_todo");
        object.insert(
            "todos".to_owned(),
            serde_json::json!([{ "text": "Buy milk", "done": false }]),
        );

        let restored: AppState = serde_json::from_value(value).unwrap();

        assert_eq!(restored.focused_todo, None);
        assert_eq!(restored.todos.len(), 1);
        assert_eq!(restored.todos[0].sessions, 0);
    }
}
