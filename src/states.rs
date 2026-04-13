use bevy::prelude::*;

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
/// In a real game this would wait for assets to finish loading.
fn auto_start(
    state: Res<State<GameState>>,
    mut next: ResMut<NextState<GameState>>,
) {
    if *state.get() == GameState::Loading {
        next.set(GameState::Playing);
    }
}

/// Toggle pause with P key, or Escape when at room overview (no back nav available).
fn toggle_pause(
    state: Res<State<GameState>>,
    mut next: ResMut<NextState<GameState>>,
    input: Res<ButtonInput<KeyCode>>,
    camera_ctrl: Res<crate::camera::CameraController>,
) {
    let p_pressed = input.just_pressed(KeyCode::KeyP);
    let esc_pressed = input.just_pressed(KeyCode::Escape) && !camera_ctrl.can_go_back();

    if !p_pressed && !esc_pressed {
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
