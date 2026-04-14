use bevy::prelude::*;
use bevy_egui::input::EguiWantsInput;

use crate::components::Clickable;
use crate::input_config::InputConfig;

/// Input-agnostic player intent. Produced by raw input systems,
/// consumed by game logic systems. Decouples keyboard/mouse/gamepad
/// from game actions.
#[derive(Message, Clone, Debug)]
pub enum InputIntent {
    /// Confirm action on the currently focused/hovered object (Enter, click)
    ConfirmFocused,
    /// Go back to previous camera spot (Escape, right-click)
    CancelOrBack,
    /// Return camera to room's starting spot (Space)
    ReturnToCenter,
    /// Toggle pause (P, Escape at root)
    TogglePause,
    /// Focus next clickable object (Tab)
    CycleNext,
    /// Focus previous clickable object (Shift+Tab)
    CyclePrev,
}

/// Reads raw keyboard + mouse input, guards against egui, emits InputIntent messages.
fn produce_input_intents(
    keys: Res<ButtonInput<KeyCode>>,
    mouse: Res<ButtonInput<MouseButton>>,
    config: Res<InputConfig>,
    egui_input: Res<EguiWantsInput>,
    mut intents: MessageWriter<InputIntent>,
) {
    // Guard: don't produce keyboard intents while egui has focus
    let kbd_ok = !egui_input.wants_any_keyboard_input();
    let ptr_ok = !egui_input.wants_any_pointer_input();

    if kbd_ok && keys.just_pressed(config.back) {
        intents.write(InputIntent::CancelOrBack);
    }
    if ptr_ok && mouse.just_pressed(MouseButton::Right) {
        intents.write(InputIntent::CancelOrBack);
    }
    if kbd_ok && keys.just_pressed(config.return_to_center) {
        intents.write(InputIntent::ReturnToCenter);
    }
    if kbd_ok && keys.just_pressed(config.pause) {
        intents.write(InputIntent::TogglePause);
    }
    if kbd_ok && keys.just_pressed(config.cycle_next) {
        if keys.pressed(KeyCode::ShiftLeft) || keys.pressed(KeyCode::ShiftRight) {
            intents.write(InputIntent::CyclePrev);
        } else {
            intents.write(InputIntent::CycleNext);
        }
    }
    if kbd_ok && keys.just_pressed(config.confirm) {
        intents.write(InputIntent::ConfirmFocused);
    }
}

/// Tracks which Clickable entity has keyboard focus for Tab cycling.
#[derive(Resource, Default)]
pub struct FocusedClickable {
    pub entity: Option<Entity>,
    pub ordered: Vec<Entity>,
}

/// Marker component for the currently keyboard-focused entity's highlight.
#[derive(Component)]
pub struct KeyboardFocusHighlight;

/// Rebuild the ordered list of clickable entities when the scene changes.
fn update_clickable_focus_list(
    clickables: Query<Entity, (With<crate::components::Clickable>, With<Visibility>)>,
    mut focused: ResMut<FocusedClickable>,
    mut needs_rebuild: Local<usize>,
) {
    let count = clickables.iter().count();
    if count == *needs_rebuild {
        return;
    }
    *needs_rebuild = count;

    focused.ordered = clickables.iter().collect();
    // If the currently focused entity is gone, clear focus
    if let Some(e) = focused.entity {
        if !focused.ordered.contains(&e) {
            focused.entity = None;
        }
    }
}

/// Cycle through clickable entities on Tab/Shift-Tab.
fn handle_cycle_intent(
    mut intents: MessageReader<InputIntent>,
    mut focused: ResMut<FocusedClickable>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mat_q: Query<&MeshMaterial3d<StandardMaterial>>,
    mut commands: Commands,
) {
    let mut direction: Option<i32> = None;
    for intent in intents.read() {
        match intent {
            InputIntent::CycleNext => direction = Some(1),
            InputIntent::CyclePrev => direction = Some(-1),
            _ => {}
        }
    }

    let Some(dir) = direction else { return };
    if focused.ordered.is_empty() { return; }

    // Remove highlight from previous
    if let Some(prev) = focused.entity {
        if let Ok(mat_handle) = mat_q.get(prev) {
            if let Some(mat) = materials.get_mut(&mat_handle.0) {
                mat.emissive = LinearRgba::NONE;
            }
        }
        commands.entity(prev).remove::<KeyboardFocusHighlight>();
    }

    // Advance index
    let current_idx = focused.entity
        .and_then(|e| focused.ordered.iter().position(|&o| o == e))
        .map(|i| i as i32)
        .unwrap_or(-1);

    let len = focused.ordered.len() as i32;
    let next_idx = ((current_idx + dir).rem_euclid(len)) as usize;
    let next_entity = focused.ordered[next_idx];

    focused.entity = Some(next_entity);

    // Apply highlight
    if let Ok(mat_handle) = mat_q.get(next_entity) {
        if let Some(mat) = materials.get_mut(&mat_handle.0) {
            mat.emissive = LinearRgba::new(0.2, 0.2, 0.05, 1.0);
        }
    }
    commands.entity(next_entity).insert(KeyboardFocusHighlight);
}

/// Confirm action on the keyboard-focused entity (same effect as clicking it).
fn handle_confirm_intent(
    mut intents: MessageReader<InputIntent>,
    focused: Res<FocusedClickable>,
    clickable_q: Query<(
        &crate::components::Clickable,
        Option<&crate::components::NavigatesTo>,
        Option<&crate::components::InventoryItem>,
        Option<&crate::navigation::Portal>,
        Option<&crate::components::RequiresItem>,
    )>,
    inv: Res<crate::inventory::Inventory>,
    mut feedback: ResMut<crate::interaction::FeedbackText>,
    mut camera_ctrl: ResMut<crate::camera::CameraController>,
    mut pickup_events: MessageWriter<crate::inventory::ItemPickedUp>,
    mut portal_events: MessageWriter<crate::navigation::PortalApproachRequested>,
    mut nav_events: MessageWriter<crate::interaction::PlayerNavigated>,
    mut objective_events: MessageWriter<crate::objectives::ObjectiveCompleted>,
    mut win_events: MessageWriter<crate::interaction::GameWon>,
    camera_q: Query<(Entity, &Transform), With<crate::camera::PlayerCamera>>,
    spot_q: Query<(&crate::components::CameraSpot, &GlobalTransform, Entity)>,
    state_q: Query<&crate::components::ObjectState>,
    mut play_state: ResMut<crate::interaction::PlayState>,
    mut commands: Commands,
) {
    let has_confirm = intents.read().any(|i| matches!(i, InputIntent::ConfirmFocused));
    if !has_confirm { return; }

    let Some(entity) = focused.entity else { return };
    if camera_ctrl.transitioning { return; }

    let Ok((clickable, nav, inv_item, portal, requires)) = clickable_q.get(entity) else {
        return;
    };
    let obj_state = state_q.get(entity).ok().copied();

    // Portal
    if let Some(portal) = portal {
        feedback.0 = clickable.description.clone();
        portal_events.write(crate::navigation::PortalApproachRequested {
            portal_entity: entity,
            target_room: portal.target_room.clone(),
            entry_spot: portal.entry_spot.clone(),
        });
        return;
    }

    // Item pickup
    if let Some(item) = inv_item {
        feedback.0 = format!("You pick up the {}.", item.name);
        pickup_events.write(crate::inventory::ItemPickedUp {
            item_id: item.item_id.clone(),
            name: item.name.clone(),
        });
        commands
            .entity(entity)
            .insert(Visibility::Hidden)
            .remove::<crate::components::Clickable>()
            .remove::<crate::components::ContainedIn>();
        return;
    }

    // Locked + RequiresItem
    if let Some(crate::components::ObjectState::Locked) = obj_state {
        if let Some(req) = requires {
            if inv.has_item(&req.item_id) {
                commands.entity(entity).insert(crate::components::ObjectState::Unlocked);
                feedback.0 = req.use_message.clone();
                if !req.completes_objective.is_empty() {
                    objective_events.write(crate::objectives::ObjectiveCompleted {
                        id: req.completes_objective.clone(),
                    });
                }
            } else {
                feedback.0 = req.fail_message.clone();
            }
            return;
        }
    }

    // Unlocked → GameWon
    if let Some(crate::components::ObjectState::Unlocked) = obj_state {
        feedback.0 = "You step through the door into the light.".into();
        win_events.write(crate::interaction::GameWon);
        return;
    }

    // Default: feedback
    feedback.0 = clickable.description.clone();

    // NavigatesTo
    if let Some(nav) = nav {
        let Ok((camera_entity, camera_transform)) = camera_q.single() else { return };
        let Some((spot, spot_gt, _)) = crate::camera::find_spot_by_name(spot_q.iter(), &nav.spot_name) else { return };

        camera_ctrl.navigate_to(&nav.spot_name);
        *play_state = crate::interaction::PlayState::Transitioning;
        nav_events.write(crate::interaction::PlayerNavigated { spot_name: nav.spot_name.clone() });
        crate::camera::start_camera_tween(&mut commands, camera_entity, camera_transform, &spot, &spot_gt);
    }
}

/// Dwell-click: hovering over a clickable object for N seconds auto-triggers a click.
#[derive(Resource)]
pub struct DwellClickSettings {
    pub enabled: bool,
    pub duration_secs: f32,
}

impl Default for DwellClickSettings {
    fn default() -> Self {
        Self {
            enabled: false,
            duration_secs: 1.5,
        }
    }
}

/// Tracks which entity is being dwelled on and for how long.
#[derive(Resource, Default)]
struct DwellState {
    entity: Option<Entity>,
    timer: f32,
}

/// Update dwell timer based on hover state. Fire ConfirmFocused when threshold reached.
fn update_dwell_click(
    time: Res<Time>,
    settings: Res<DwellClickSettings>,
    hovered: Query<Entity, (With<Clickable>, With<super::interaction::OriginalEmissive>)>,
    mut dwell: ResMut<DwellState>,
    mut focused: ResMut<FocusedClickable>,
    mut intents: MessageWriter<InputIntent>,
) {
    if !settings.enabled {
        return;
    }

    // Find the currently hovered clickable (has OriginalEmissive = hover active)
    let current_hover = hovered.iter().next();

    match (current_hover, dwell.entity) {
        (Some(hovered_e), Some(dwell_e)) if hovered_e == dwell_e => {
            // Same entity — accumulate time
            dwell.timer += time.delta_secs();
            if dwell.timer >= settings.duration_secs {
                // Dwell complete — set focus and fire confirm
                focused.entity = Some(hovered_e);
                intents.write(InputIntent::ConfirmFocused);
                dwell.timer = 0.0;
                dwell.entity = None;
            }
        }
        (Some(hovered_e), _) => {
            // New entity — reset timer
            dwell.entity = Some(hovered_e);
            dwell.timer = 0.0;
        }
        (None, _) => {
            // Nothing hovered — reset
            dwell.entity = None;
            dwell.timer = 0.0;
        }
    }
}

pub struct InputIntentPlugin;

impl Plugin for InputIntentPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<InputIntent>()
            .init_resource::<FocusedClickable>()
            .init_resource::<DwellClickSettings>()
            .init_resource::<DwellState>()
            .add_systems(PreUpdate, produce_input_intents)
            .add_systems(
                Update,
                (
                    update_clickable_focus_list,
                    handle_cycle_intent,
                    handle_confirm_intent,
                    update_dwell_click,
                ),
            );
    }
}
