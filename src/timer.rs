use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};

pub(crate) const POMODORO_SECONDS: f64 = 25.0 * 60.0;
pub(crate) const SHORT_BREAK_SECONDS: f64 = 5.0 * 60.0;
pub(crate) const LONG_BREAK_SECONDS: f64 = 15.0 * 60.0;
pub(crate) const POMODOROS_PER_LONG_BREAK: u32 = 4;

pub(crate) fn now_millis() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
}

pub(crate) fn format_stopwatch(secs: f64) -> String {
    let total_secs = secs as u64;
    let seconds = total_secs % 60;
    let minutes = (total_secs / 60) % 60;
    let hours = total_secs / 3600;
    format!("{hours:02}:{minutes:02}:{seconds:02}")
}

pub(crate) fn format_countdown(secs: f64) -> String {
    let total = secs.ceil() as u64;
    format!("{:02}:{:02}", total / 60, total % 60)
}

/// A timer that stores its accumulated time plus the wall-clock time of the
/// current run. This lets it resume correctly after the app is closed.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct TimerState {
    pub(crate) running: bool,
    pub(crate) elapsed_secs: f64,
    pub(crate) started_at: Option<u128>,
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
    pub(crate) fn elapsed(&self, now: u128) -> f64 {
        let mut elapsed = self.elapsed_secs;
        if let Some(started_at) = self.started_at {
            elapsed += now.saturating_sub(started_at) as f64 / 1000.0;
        }
        elapsed
    }

    pub(crate) fn toggle(&mut self, now: u128) {
        if self.running {
            self.elapsed_secs = self.elapsed(now);
            self.started_at = None;
            self.running = false;
        } else {
            self.started_at = Some(now);
            self.running = true;
        }
    }

    pub(crate) fn reset(&mut self) {
        *self = Self::default();
    }

    pub(crate) fn is_paused(&self) -> bool {
        !self.running && self.elapsed_secs > 0.0
    }
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
}
