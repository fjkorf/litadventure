use bevy::prelude::*;
use bevy::picking::events::{Click, Out, Over, Pointer};
use bevy_egui::input::EguiWantsInput;

use crate::camera::{
    find_spot_by_name, start_camera_tween, CameraController, PlayerCamera,
};
use crate::components::*;
use crate::input_intent::{InputIntent, InputMode, input_mode};
use crate::inventory::{Inventory, ItemPickedUp};
use crate::navigation::{Portal, PortalApproachRequested};
use crate::objectives::ObjectiveCompleted;

/// Current player interaction state.
#[derive(Resource, Debug, Clone, PartialEq)]
pub enum PlayState {
    Exploring,
    Examining(Entity),
    Transitioning,
}

impl Default for PlayState {
    fn default() -> Self {
        Self::Exploring
    }
}

/// The last interaction feedback message, displayed in the UI.
#[derive(Resource, Default, Debug)]
pub struct FeedbackText(pub String);

/// Fired when the player navigates to a camera spot.
#[derive(Message)]
pub struct PlayerNavigated {
    pub spot_name: String,
}

/// Fired when the player wins the game.
#[derive(Message)]
pub struct GameWon;

/// Fired when the player is permanently stuck (no portal, failed interaction).
#[derive(Message)]
pub struct GameOver;

/// Original emissive stored on hover so we can restore it.
#[derive(Component)]
pub struct OriginalEmissive(pub LinearRgba);

/// Handles clicks on any entity — filters for Clickable component.
fn on_click(
    event: On<Pointer<Click>>,
    egui_input: Res<EguiWantsInput>,
    clickable_q: Query<(
        &Clickable,
        Option<&NavigatesTo>,
        Option<&InventoryItem>,
        Option<&Portal>,
        Option<&RequiresItem>,
        Option<&TweenConfig>,
        &Transform,
    )>,
    camera_q: Query<(Entity, &Transform), With<PlayerCamera>>,
    spot_q: Query<(&CameraSpot, &GlobalTransform, Entity)>,
    inv: Res<Inventory>,
    mut play_state: ResMut<PlayState>,
    mut feedback: ResMut<FeedbackText>,
    mut camera_ctrl: ResMut<CameraController>,
    mut pickup_events: MessageWriter<ItemPickedUp>,
    mut portal_approach_events: MessageWriter<PortalApproachRequested>,
    mut nav_events: MessageWriter<PlayerNavigated>,
    mut objective_events: MessageWriter<ObjectiveCompleted>,
    mut win_events: MessageWriter<GameWon>,
    mut contained_q: Query<(Entity, &ContainedIn, &mut Visibility)>,
    mut state_q: Query<&mut ObjectState>,
    mut commands: Commands,
) {
    // Don't process clicks when egui is consuming pointer input
    if egui_input.wants_any_pointer_input() {
        return;
    }

    let entity = event.event_target();
    let Ok((clickable, nav, inv_item, portal, requires, tween_cfg, entity_transform)) =
        clickable_q.get(entity)
    else {
        return;
    };
    let obj_state = state_q.get(entity).ok().map(|s| *s);

    // Don't process clicks while camera is moving
    if camera_ctrl.transitioning || *play_state == PlayState::Transitioning {
        return;
    }

    // Portal: transition to another room
    if let Some(portal) = portal {
        feedback.0 = clickable.description.clone();
        portal_approach_events.write(PortalApproachRequested {
            portal_entity: entity,
            target_room: portal.target_room.clone(),
            entry_spot: portal.entry_spot.clone(),
        });
        return;
    }

    // Collectible item: pick it up
    if let Some(item) = inv_item {
        feedback.0 = format!("You pick up the {}.", item.name);
        pickup_events.write(ItemPickedUp {
            item_id: item.item_id.clone(),
            name: item.name.clone(),
            description: item.description.clone(),
        });
        commands
            .entity(entity)
            .insert(Visibility::Hidden)
            .remove::<Clickable>()
            .remove::<ContainedIn>();
        return;
    }

    // Stateful object (drawer, locked door, etc.)
    if let Some(obj_state) = obj_state {
        match obj_state {
            ObjectState::Locked => {
                if let Some(req) = requires {
                    if inv.has_item(&req.item_id) {
                        // Unlock!
                        if let Ok(mut state) = state_q.get_mut(entity) {
                            *state = ObjectState::Unlocked;
                        }
                        feedback.0 = req.use_message.clone();
                        if !req.completes_objective.is_empty() {
                            objective_events.write(ObjectiveCompleted {
                                id: req.completes_objective.clone(),
                            });
                        }
                    } else {
                        feedback.0 = req.fail_message.clone();
                    }
                } else {
                    feedback.0 = clickable.description.clone();
                }
                return;
            }
            ObjectState::Unlocked => {
                // Clicking an unlocked door triggers the win
                feedback.0 = "You step through the door into the light.".into();
                win_events.write(GameWon);
                return;
            }
            ObjectState::Closed => {
                if let Ok(mut state) = state_q.get_mut(entity) {
                    *state = ObjectState::Open;
                }
                feedback.0 = "You open it.".into();

                for (_, contained, mut vis) in contained_q.iter_mut() {
                    if contained.container == entity {
                        *vis = Visibility::Inherited;
                    }
                }

                if let Some(tc) = tween_cfg {
                    let start = entity_transform.translation;
                    let end = start + tc.open_offset;
                    commands.entity(entity).insert(
                        bevy_tweening::TweenAnim::new(bevy_tweening::Tween::new(
                            EaseFunction::CubicOut,
                            std::time::Duration::from_millis(tc.duration_ms as u64),
                            bevy_tweening::lens::TransformPositionLens { start, end },
                        )),
                    );
                }
                return;
            }
            ObjectState::Open => {
                if let Ok(mut state) = state_q.get_mut(entity) {
                    *state = ObjectState::Closed;
                }
                feedback.0 = "You close it.".into();

                for (_, contained, mut vis) in contained_q.iter_mut() {
                    if contained.container == entity {
                        *vis = Visibility::Hidden;
                    }
                }

                if let Some(tc) = tween_cfg {
                    let start = entity_transform.translation;
                    let end = start - tc.open_offset;
                    commands.entity(entity).insert(
                        bevy_tweening::TweenAnim::new(bevy_tweening::Tween::new(
                            EaseFunction::CubicOut,
                            std::time::Duration::from_millis(tc.duration_ms as u64),
                            bevy_tweening::lens::TransformPositionLens { start, end },
                        )),
                    );
                }
                return;
            }
            _ => {
                feedback.0 = clickable.description.clone();
                return;
            }
        }
    }

    // Default feedback
    feedback.0 = clickable.description.clone();

    // Navigate to camera spot
    if let Some(nav) = nav {
        let Ok((camera_entity, camera_transform)) = camera_q.single() else {
            return;
        };

        let Some((spot, spot_gt, _)) =
            find_spot_by_name(spot_q.iter(), &nav.spot_name)
        else {
            return;
        };

        camera_ctrl.navigate_to(&nav.spot_name);
        *play_state = PlayState::Transitioning;

        nav_events.write(PlayerNavigated {
            spot_name: nav.spot_name.clone(),
        });

        start_camera_tween(
            &mut commands,
            camera_entity,
            camera_transform,
            &spot,
            &spot_gt,
        );
    } else {
        *play_state = PlayState::Examining(entity);
    }
}

/// When camera tween completes, transition from Transitioning to Exploring.
fn finish_transition(
    mut play_state: ResMut<PlayState>,
    camera_ctrl: Res<CameraController>,
) {
    if *play_state == PlayState::Transitioning && !camera_ctrl.transitioning {
        *play_state = PlayState::Exploring;
    }
}

/// Handle back-navigation via InputIntent::CancelOrBack.
fn handle_back_navigation(
    mut intents: MessageReader<InputIntent>,
    camera_q: Query<(Entity, &Transform), With<PlayerCamera>>,
    spot_q: Query<(&CameraSpot, &GlobalTransform, Entity)>,
    mut play_state: ResMut<PlayState>,
    mut camera_ctrl: ResMut<CameraController>,
    mut feedback: ResMut<FeedbackText>,
    mut commands: Commands,
) {
    let has_back = intents
        .read()
        .any(|i| matches!(i, InputIntent::CancelOrBack));

    if !has_back || camera_ctrl.transitioning || !camera_ctrl.can_go_back() {
        return;
    }

    let Some(target_name) = camera_ctrl.navigate_back() else {
        return;
    };

    let Ok((camera_entity, camera_transform)) = camera_q.single() else {
        return;
    };

    let Some((spot, spot_gt, _)) =
        find_spot_by_name(spot_q.iter(), &target_name)
    else {
        return;
    };

    *play_state = PlayState::Transitioning;
    feedback.0 = "You step back.".into();

    start_camera_tween(
        &mut commands,
        camera_entity,
        camera_transform,
        &spot,
        &spot_gt,
    );
}

/// Return camera to room center via InputIntent::ReturnToCenter.
fn handle_return_to_center(
    mut intents: MessageReader<InputIntent>,
    camera_q: Query<(Entity, &Transform), With<PlayerCamera>>,
    spot_q: Query<(&CameraSpot, &GlobalTransform, Entity)>,
    mut play_state: ResMut<PlayState>,
    mut camera_ctrl: ResMut<CameraController>,
    mut feedback: ResMut<FeedbackText>,
    room_registry: Res<crate::navigation::RoomRegistry>,
    current_room: Res<crate::navigation::CurrentRoom>,
    mut commands: Commands,
) {
    let has_center = intents
        .read()
        .any(|i| matches!(i, InputIntent::ReturnToCenter));

    if !has_center || camera_ctrl.transitioning {
        return;
    }

    // Find the starting spot for the current room
    let starting_spot = room_registry
        .starting_spot(&current_room.name)
        .unwrap_or("room_overview");

    // Already at the center
    if camera_ctrl.current_spot.as_deref() == Some(starting_spot) {
        return;
    }

    let Ok((camera_entity, camera_transform)) = camera_q.single() else {
        return;
    };

    let Some((spot, spot_gt, _)) = find_spot_by_name(spot_q.iter(), starting_spot) else {
        return;
    };

    // Clear history and go to center
    camera_ctrl.history.clear();
    camera_ctrl.current_spot = Some(starting_spot.to_string());
    *play_state = PlayState::Transitioning;
    feedback.0 = "You look around the room.".into();

    start_camera_tween(
        &mut commands,
        camera_entity,
        camera_transform,
        &spot,
        &spot_gt,
    );
}

/// Detect stuck state: when a RequiresItem interaction fails AND no portal exists.
fn check_stuck(
    requires_q: Query<&RequiresItem>,
    portal_q: Query<(), With<Portal>>,
    inv: Res<Inventory>,
    feedback: Res<FeedbackText>,
    mut game_over_events: MessageWriter<GameOver>,
) {
    // Only check when feedback just changed (avoid re-triggering)
    if !feedback.is_changed() || feedback.0.is_empty() {
        return;
    }

    // Check if the feedback matches any RequiresItem fail message
    let is_fail = requires_q
        .iter()
        .any(|req| feedback.0 == req.fail_message && !inv.has_item(&req.item_id));

    if !is_fail {
        return;
    }

    if portal_q.is_empty() {
        game_over_events.write(GameOver);
    }
    // If portals exist, the fail_message alone is sufficient — player can go back
}

/// Highlight clickable objects on hover.
fn on_hover_start(
    event: On<Pointer<Over>>,
    clickable_q: Query<&MeshMaterial3d<StandardMaterial>, With<Clickable>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut commands: Commands,
) {
    let entity = event.event_target();
    let Ok(mat_handle) = clickable_q.get(entity) else {
        return;
    };

    if let Some(mat) = materials.get_mut(&mat_handle.0) {
        commands
            .entity(entity)
            .insert(OriginalEmissive(mat.emissive));
        mat.emissive = LinearRgba::new(0.15, 0.15, 0.15, 1.0);
    }
}

/// Remove highlight when pointer leaves.
fn on_hover_end(
    event: On<Pointer<Out>>,
    q: Query<(&MeshMaterial3d<StandardMaterial>, &OriginalEmissive)>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut commands: Commands,
) {
    let entity = event.event_target();
    let Ok((mat_handle, original)) = q.get(entity) else {
        return;
    };

    if let Some(mat) = materials.get_mut(&mat_handle.0) {
        mat.emissive = original.0;
    }

    commands.entity(entity).remove::<OriginalEmissive>();
}

pub struct InteractionPlugin;

impl Plugin for InteractionPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<PlayState>()
            .init_resource::<FeedbackText>()
            .add_message::<PlayerNavigated>()
            .add_message::<GameWon>()
            .add_message::<GameOver>()
            .add_observer(on_click)
            .add_observer(on_hover_start)
            .add_observer(on_hover_end)
            .add_systems(
                Update,
                (
                    finish_transition,
                    handle_back_navigation.run_if(input_mode(InputMode::Playing)),
                    handle_return_to_center.run_if(input_mode(InputMode::Playing)),
                    check_stuck,
                ),
            );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn play_state_defaults_to_exploring() {
        assert_eq!(PlayState::default(), PlayState::Exploring);
    }

    #[test]
    fn feedback_text_defaults_to_empty() {
        assert!(FeedbackText::default().0.is_empty());
    }

    #[test]
    fn play_state_examining_holds_entity() {
        let entity = Entity::from_bits(42);
        assert_eq!(
            PlayState::Examining(entity),
            PlayState::Examining(entity)
        );
    }
}
