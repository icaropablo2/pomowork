use crate::state::{AppState, PomodoroMode};
use rodio::{source::Source, Decoder, DeviceSinkBuilder, Float, MixerDeviceSink, Player};
use std::io::Cursor;

const BROWN_NOISE_MP3: &[u8] = include_bytes!("../assets/ticking-brown.mp3");
const KITCHEN_ALARM_MP3: &[u8] = include_bytes!("../assets/alarm-kitchen.mp3");
const BROWN_NOISE_VOLUME: Float = 0.35;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum AudioCommand {
    StartBrownNoise,
    StopBrownNoise,
    PlayKitchenAlarm,
}

fn should_play_brown_noise(state: &AppState) -> bool {
    state.sound_enabled && state.pomodoro_mode == PomodoroMode::Pomodoro && state.pomodoro.running
}

#[derive(Default)]
pub(crate) struct AudioDirector {
    noise_playing: bool,
}

impl AudioDirector {
    pub(crate) fn sync(
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

pub(crate) struct AudioPlayer {
    // The sink must outlive the player, otherwise playback stops immediately.
    _sink: Option<MixerDeviceSink>,
    player: Option<Player>,
}

impl AudioPlayer {
    pub(crate) fn new() -> Self {
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

    pub(crate) fn apply(&mut self, command: AudioCommand) {
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::{AppState, PomodoroMode};

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
}
