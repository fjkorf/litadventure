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
    game_state: Res<State<crate::states::GameState>>,
    mut intents: MessageWriter<InputIntent>,
) {
    let kbd_ok = !egui_input.wants_any_keyboard_input();
    let ptr_ok = !egui_input.wants_any_pointer_input();

    // Overlay states (Title, Paused, Won, GameOver) have egui buttons —
    // let egui handle Tab/Enter natively for button navigation.
    // Playing state has 3D clickable objects — route Tab/Enter to the intent system.
    let is_overlay = matches!(
        game_state.get(),
        crate::states::GameState::Title
            | crate::states::GameState::Paused
            | crate::states::GameState::Won
            | crate::states::GameState::GameOver
    );

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

    // Tab/Enter: 3D scene during gameplay, egui buttons during overlays
    if !is_overlay {
        if keys.just_pressed(config.cycle_next) {
            if keys.pressed(KeyCode::ShiftLeft) || keys.pressed(KeyCode::ShiftRight) {
                intents.write(InputIntent::CyclePrev);
            } else {
                intents.write(InputIntent::CycleNext);
            }
        }
        if keys.just_pressed(config.confirm) {
            intents.write(InputIntent::ConfirmFocused);
        }
    }
}

/// Tracks which Clickable entity has keyboard focus for Tab cycling.
#[derive(Resource, Default, Reflect)]
#[reflect(Resource)]
pub struct FocusedClickable {
    pub entity: Option<Entity>,
    pub ordered: Vec<Entity>,
}

/// Marker component for the currently keyboard-focused entity's highlight.
#[derive(Component, Reflect)]
#[reflect(Component)]
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

/// Find the MeshMaterial3d handle on an entity or its children (glTF hierarchy).
fn find_material(
    entity: Entity,
    mat_q: &Query<&MeshMaterial3d<StandardMaterial>>,
    children_q: &Query<&Children>,
) -> Option<Handle<StandardMaterial>> {
    // Try the entity itself first
    if let Ok(mat) = mat_q.get(entity) {
        return Some(mat.0.clone());
    }
    // Walk children (glTF puts mesh on child entities)
    if let Ok(children) = children_q.get(entity) {
        for child in children.iter() {
            if let Ok(mat) = mat_q.get(child) {
                return Some(mat.0.clone());
            }
        }
    }
    None
}

/// Set emissive on an entity's material (handles glTF child hierarchy).
fn set_emissive(
    entity: Entity,
    emissive: LinearRgba,
    mat_q: &Query<&MeshMaterial3d<StandardMaterial>>,
    children_q: &Query<&Children>,
    materials: &mut Assets<StandardMaterial>,
) {
    if let Some(handle) = find_material(entity, mat_q, children_q) {
        if let Some(mat) = materials.get_mut(&handle) {
            mat.emissive = emissive;
        }
    }
}

/// Cycle through clickable entities on Tab/Shift-Tab.
fn handle_cycle_intent(
    mut intents: MessageReader<InputIntent>,
    mut focused: ResMut<FocusedClickable>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mat_q: Query<&MeshMaterial3d<StandardMaterial>>,
    children_q: Query<&Children>,
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
        set_emissive(prev, LinearRgba::NONE, &mat_q, &children_q, &mut materials);
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

    // Apply highlight — bright enough to be clearly visible
    set_emissive(
        next_entity,
        LinearRgba::new(0.8, 0.6, 0.1, 1.0),
        &mat_q,
        &children_q,
        &mut materials,
    );
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
        app.register_type::<FocusedClickable>()
            .register_type::<KeyboardFocusHighlight>()
            .add_message::<InputIntent>()
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
