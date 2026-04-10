use std::collections::HashMap;

use bevy::prelude::*;

use crate::game_data::{GameDataHandles, GameDataReady, HintSet};
use crate::interaction::PlayerNavigated;
use crate::inventory::ItemPickedUp;
use crate::objectives::{ObjectiveCompleted, Objectives};
use crate::states::GameState;

/// Tracks hint timer and current availability.
#[derive(Resource, Default)]
pub struct HintState {
    pub idle_time: f32,
    pub available_tier: u8,
    pub tier_thresholds: [f32; 3],
}

/// Maps objective IDs to 3-tier hint strings.
#[derive(Resource, Default)]
pub struct HintDatabase {
    pub hints: HashMap<String, [String; 3]>,
}

/// The current hint text to display (empty = no hint).
#[derive(Resource, Default)]
pub struct HintText(pub String);

/// Load hints from RON data once assets are ready.
fn load_hints_from_data(
    ready: Res<GameDataReady>,
    handles: Res<GameDataHandles>,
    assets: Res<Assets<HintSet>>,
    mut db: ResMut<HintDatabase>,
    mut hint_state: ResMut<HintState>,
    mut loaded: Local<bool>,
) {
    if *loaded || !ready.0 {
        return;
    }

    let Some(handle) = &handles.hints else {
        return;
    };
    let Some(data) = assets.get(handle) else {
        return;
    };

    db.hints.clone_from(&data.hints);
    hint_state.tier_thresholds = data.tier_thresholds;

    *loaded = true;
}

/// Increment idle timer every frame while playing.
fn tick_hint_timer(time: Res<Time>, mut hint_state: ResMut<HintState>) {
    hint_state.idle_time += time.delta_secs();

    let [t1, t2, t3] = hint_state.tier_thresholds;
    let new_tier = if t3 > 0.0 && hint_state.idle_time >= t3 {
        3
    } else if t2 > 0.0 && hint_state.idle_time >= t2 {
        2
    } else if t1 > 0.0 && hint_state.idle_time >= t1 {
        1
    } else {
        0
    };

    if new_tier != hint_state.available_tier {
        hint_state.available_tier = new_tier;
    }
}

fn reset_on_navigation(
    mut reader: MessageReader<PlayerNavigated>,
    mut hint_state: ResMut<HintState>,
) {
    for _ in reader.read() {
        hint_state.idle_time = 0.0;
        hint_state.available_tier = 0;
    }
}

fn reset_on_pickup(
    mut reader: MessageReader<ItemPickedUp>,
    mut hint_state: ResMut<HintState>,
) {
    for _ in reader.read() {
        hint_state.idle_time = 0.0;
        hint_state.available_tier = 0;
    }
}

fn reset_on_objective(
    mut reader: MessageReader<ObjectiveCompleted>,
    mut hint_state: ResMut<HintState>,
) {
    for _ in reader.read() {
        hint_state.idle_time = 0.0;
        hint_state.available_tier = 0;
    }
}

/// Sync current hint text based on active objective and available tier.
fn sync_hint_text(
    hint_state: Res<HintState>,
    db: Res<HintDatabase>,
    objectives: Res<Objectives>,
    mut hint_text: ResMut<HintText>,
) {
    if hint_state.available_tier == 0 {
        if !hint_text.0.is_empty() {
            hint_text.0.clear();
        }
        return;
    }

    let active = objectives.active_visible();
    let Some(current) = active.first() else {
        hint_text.0.clear();
        return;
    };

    if let Some(hints) = db.hints.get(&current.id) {
        let tier_idx = (hint_state.available_tier as usize).min(3) - 1;
        let new_text = &hints[tier_idx];
        if hint_text.0 != *new_text {
            hint_text.0 = new_text.clone();
        }
    } else {
        hint_text.0.clear();
    }
}

pub struct HintsPlugin;

impl Plugin for HintsPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<HintState>()
            .init_resource::<HintDatabase>()
            .init_resource::<HintText>()
            .add_systems(
                Update,
                (
                    load_hints_from_data,
                    tick_hint_timer.run_if(in_state(GameState::Playing)),
                    reset_on_navigation,
                    reset_on_pickup,
                    reset_on_objective,
                    sync_hint_text,
                ),
            );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hint_tiers_progress() {
        let state = HintState {
            idle_time: 31.0,
            tier_thresholds: [30.0, 60.0, 90.0],
            ..default()
        };
        assert!(state.idle_time >= state.tier_thresholds[0]);
    }

    #[test]
    fn hint_database_has_entries() {
        let mut db = HintDatabase::default();
        db.hints
            .insert("test".into(), ["a".into(), "b".into(), "c".into()]);
        assert_eq!(db.hints["test"][0], "a");
        assert_eq!(db.hints["test"][2], "c");
    }
}
