use bevy::prelude::*;
use bevy::picking::events::{Click, Out, Over, Pointer};

use crate::camera::{
    find_spot_by_name, start_camera_tween, CameraController, PlayerCamera,
};
use crate::components::*;
use crate::inventory::{Inventory, ItemPickedUp};
use crate::navigation::{Portal, RoomTransition};
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

/// Original emissive stored on hover so we can restore it.
#[derive(Component)]
struct OriginalEmissive(LinearRgba);

/// Handles clicks on any entity — filters for Clickable component.
fn on_click(
    event: On<Pointer<Click>>,
    clickable_q: Query<(
        &Clickable,
        Option<&NavigatesTo>,
        Option<&InventoryItem>,
        Option<&Portal>,
        Option<&RequiresItem>,
    )>,
    camera_q: Query<(Entity, &Transform), With<PlayerCamera>>,
    spot_q: Query<(&CameraSpot, &GlobalTransform, Entity)>,
    inv: Res<Inventory>,
    mut play_state: ResMut<PlayState>,
    mut feedback: ResMut<FeedbackText>,
    mut camera_ctrl: ResMut<CameraController>,
    mut pickup_events: MessageWriter<ItemPickedUp>,
    mut room_events: MessageWriter<RoomTransition>,
    mut nav_events: MessageWriter<PlayerNavigated>,
    mut objective_events: MessageWriter<ObjectiveCompleted>,
    mut win_events: MessageWriter<GameWon>,
    mut contained_q: Query<(Entity, &ContainedIn, &mut Visibility)>,
    mut state_q: Query<&mut ObjectState>,
    tween_q: Query<(&Transform, &TweenConfig)>,
    mut commands: Commands,
) {
    let entity = event.event_target();
    let Ok((clickable, nav, inv_item, portal, requires)) = clickable_q.get(entity) else {
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
        room_events.write(RoomTransition {
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
                        objective_events.write(ObjectiveCompleted {
                            id: "unlock_door".into(),
                        });
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

                if let Ok((transform, tween_cfg)) = tween_q.get(entity) {
                    let start = transform.translation;
                    let end = start + tween_cfg.open_offset;
                    commands.entity(entity).insert(
                        bevy_tweening::TweenAnim::new(bevy_tweening::Tween::new(
                            EaseFunction::CubicOut,
                            std::time::Duration::from_millis(tween_cfg.duration_ms as u64),
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

                if let Ok((transform, tween_cfg)) = tween_q.get(entity) {
                    let start = transform.translation;
                    let end = start - tween_cfg.open_offset;
                    commands.entity(entity).insert(
                        bevy_tweening::TweenAnim::new(bevy_tweening::Tween::new(
                            EaseFunction::CubicOut,
                            std::time::Duration::from_millis(tween_cfg.duration_ms as u64),
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

/// Handle back-navigation (Escape key or right-click).
fn handle_back_navigation(
    input: Res<ButtonInput<KeyCode>>,
    mouse: Res<ButtonInput<MouseButton>>,
    camera_q: Query<(Entity, &Transform), With<PlayerCamera>>,
    spot_q: Query<(&CameraSpot, &GlobalTransform, Entity)>,
    mut play_state: ResMut<PlayState>,
    mut camera_ctrl: ResMut<CameraController>,
    mut feedback: ResMut<FeedbackText>,
    mut commands: Commands,
) {
    if camera_ctrl.transitioning {
        return;
    }

    let go_back = input.just_pressed(KeyCode::Escape)
        || mouse.just_pressed(MouseButton::Right);

    if !go_back || !camera_ctrl.can_go_back() {
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
            .add_observer(on_click)
            .add_observer(on_hover_start)
            .add_observer(on_hover_end)
            .add_systems(
                Update,
                (finish_transition, handle_back_navigation),
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
