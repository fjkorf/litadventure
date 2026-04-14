use bevy::prelude::*;

use crate::input_intent::InputIntent;

/// Top-level game state.
#[derive(States, Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum GameState {
    #[default]
    Title,
    Loading,
    Playing,
    Paused,
    Won,
    GameOver,
}

/// System that transitions from Loading to Playing after one frame.
fn auto_start(
    state: Res<State<GameState>>,
    mut next: ResMut<NextState<GameState>>,
) {
    if *state.get() == GameState::Loading {
        next.set(GameState::Playing);
    }
}

/// Toggle pause via InputIntent::TogglePause or InputIntent::CancelOrBack (at room root).
fn toggle_pause(
    state: Res<State<GameState>>,
    mut next: ResMut<NextState<GameState>>,
    mut intents: MessageReader<InputIntent>,
    camera_ctrl: Res<crate::camera::CameraController>,
) {
    let mut should_toggle = false;

    for intent in intents.read() {
        match intent {
            InputIntent::TogglePause => {
                should_toggle = true;
            }
            InputIntent::CancelOrBack if !camera_ctrl.can_go_back() => {
                // Escape at room overview = toggle pause
                should_toggle = true;
            }
            _ => {}
        }
    }

    if !should_toggle {
        return;
    }

    match state.get() {
        GameState::Playing => next.set(GameState::Paused),
        GameState::Paused => next.set(GameState::Playing),
        _ => {}
    }
}

pub struct StatesPlugin;

impl Plugin for StatesPlugin {
    fn build(&self, app: &mut App) {
        app.init_state::<GameState>()
            .add_systems(Update, auto_start.run_if(in_state(GameState::Loading)))
            .add_systems(Update, toggle_pause);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn game_state_default_is_title() {
        let state = GameState::default();
        assert_eq!(state, GameState::Title);
    }
}
