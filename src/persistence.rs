use crate::state::AppState;
use std::fs;
use std::path::{Path, PathBuf};

pub(crate) const SAVE_INTERVAL_MS: u128 = 2_000;

pub(crate) fn state_file() -> Option<PathBuf> {
    dirs::data_dir().map(|dir| dir.join("pomowork").join("state.json"))
}

pub(crate) fn load_state_from(path: &Path) -> AppState {
    fs::read_to_string(path)
        .ok()
        .and_then(|json| serde_json::from_str::<AppState>(&json).ok())
        .unwrap_or_default()
}

pub(crate) fn load_state() -> AppState {
    state_file().map_or_else(AppState::default, |path| load_state_from(&path))
}

pub(crate) fn save_state_to(path: &Path, state: &AppState) {
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    if let Ok(json) = serde_json::to_string_pretty(state) {
        let _ = fs::write(path, json);
    }
}

pub(crate) fn save_state(state: &AppState) {
    if let Some(path) = state_file() {
        save_state_to(&path, state);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::{AppState, PomodoroMode};
    use std::fs;
    use std::path::PathBuf;

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
}
