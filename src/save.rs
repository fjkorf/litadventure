use bevy::prelude::*;
use serde::{Deserialize, Serialize};

use crate::camera::CameraController;
use crate::components::{Clickable, ObjectState};
use crate::interaction::FeedbackText;
use crate::inventory::Inventory;
use crate::navigation::CurrentRoom;
use crate::objectives::Objectives;

// -- Platform-specific save I/O --

#[cfg(not(target_arch = "wasm32"))]
const SAVE_PATH: &str = "assets/saves/game_save.ron";

#[cfg(not(target_arch = "wasm32"))]
fn save_exists() -> bool {
    std::path::Path::new(SAVE_PATH).exists()
}

#[cfg(not(target_arch = "wasm32"))]
fn write_save(data: &str) -> Result<(), String> {
    std::fs::write(SAVE_PATH, data).map_err(|e| e.to_string())
}

#[cfg(not(target_arch = "wasm32"))]
fn read_save() -> Result<String, String> {
    std::fs::read_to_string(SAVE_PATH).map_err(|e| e.to_string())
}

#[cfg(target_arch = "wasm32")]
const SAVE_KEY: &str = "litadventure_save";

#[cfg(target_arch = "wasm32")]
fn get_local_storage() -> Option<web_sys::Storage> {
    web_sys::window()?.local_storage().ok()?
}

#[cfg(target_arch = "wasm32")]
fn save_exists() -> bool {
    get_local_storage()
        .and_then(|s| s.get_item(SAVE_KEY).ok())
        .flatten()
        .is_some()
}

#[cfg(target_arch = "wasm32")]
fn write_save(data: &str) -> Result<(), String> {
    let storage = get_local_storage().ok_or_else(|| "No localStorage".to_string())?;
    storage.set_item(SAVE_KEY, data).map_err(|_| "Failed to write localStorage".to_string())
}

#[cfg(target_arch = "wasm32")]
fn read_save() -> Result<String, String> {
    let storage = get_local_storage().ok_or_else(|| "No localStorage".to_string())?;
    storage
        .get_item(SAVE_KEY)
        .map_err(|_| "Failed to read localStorage".to_string())?
        .ok_or_else(|| "No save found".to_string())
}

/// Pending entity states to apply after a room scene finishes loading.
#[derive(Resource, Default)]
pub struct PendingEntityStates {
    pub entity_states: Vec<(String, String)>,
    pub collected_entities: Vec<String>,
}

fn parse_object_state(s: &str) -> Option<ObjectState> {
    match s {
        "Default" => Some(ObjectState::Default),
        "Locked" => Some(ObjectState::Locked),
        "Unlocked" => Some(ObjectState::Unlocked),
        "Open" => Some(ObjectState::Open),
        "Closed" => Some(ObjectState::Closed),
        "Collected" => Some(ObjectState::Collected),
        _ => None,
    }
}

// -- Save data structures --

#[derive(Serialize, Deserialize, Debug)]
pub struct SaveGame {
    pub current_room: String,
    pub camera_spot: Option<String>,
    pub camera_history: Vec<String>,
    pub items: Vec<SavedItem>,
    pub objective_states: Vec<SavedObjective>,
    pub entity_states: Vec<(String, String)>,
    pub collected_entities: Vec<String>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct SavedItem {
    pub item_id: String,
    pub name: String,
    pub qty: u32,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct SavedObjective {
    pub id: String,
    pub visible: bool,
    pub completed: bool,
}

// -- Resource to trigger save/load --

#[derive(Message)]
pub struct SaveRequested;

#[derive(Message)]
pub struct LoadRequested;

/// Whether a save file exists (checked at startup and after saves).
#[derive(Resource, Default)]
pub struct SaveExists(pub bool);

// -- Systems --

fn check_save_exists(mut save_exists_res: ResMut<SaveExists>) {
    save_exists_res.0 = save_exists();
}

fn perform_save(
    mut reader: MessageReader<SaveRequested>,
    inv: Res<Inventory>,
    objectives: Res<Objectives>,
    room: Res<CurrentRoom>,
    camera: Res<CameraController>,
    states_q: Query<(&Name, &ObjectState)>,
    collected_q: Query<&Name, (With<crate::components::InventoryItem>, Without<Clickable>)>,
    mut save_exists: ResMut<SaveExists>,
    mut feedback: ResMut<FeedbackText>,
) {
    for _ in reader.read() {
        let save = SaveGame {
            current_room: room.name.clone(),
            camera_spot: camera.current_spot.clone(),
            camera_history: camera.history.clone(),
            items: inv
                .items
                .iter()
                .map(|i| SavedItem {
                    item_id: i.item_id.clone(),
                    name: i.name.clone(),
                    qty: i.qty,
                })
                .collect(),
            objective_states: objectives
                .all
                .iter()
                .map(|o| SavedObjective {
                    id: o.id.clone(),
                    visible: o.visible,
                    completed: o.completed,
                })
                .collect(),
            entity_states: states_q
                .iter()
                .map(|(name, state)| (name.to_string(), format!("{:?}", state)))
                .collect(),
            collected_entities: collected_q.iter().map(|n| n.to_string()).collect(),
        };

        match ron::ser::to_string_pretty(&save, Default::default()) {
            Ok(ron_str) => match write_save(&ron_str) {
                Ok(()) => {
                    save_exists.0 = true;
                    feedback.0 = "Game saved.".into();
                }
                Err(e) => {
                    feedback.0 = format!("Save failed: {e}");
                }
            },
            Err(e) => {
                feedback.0 = format!("Save failed: {e}");
            }
        }
    }
}

fn perform_load(
    mut reader: MessageReader<LoadRequested>,
    mut inv: ResMut<Inventory>,
    mut objectives: ResMut<Objectives>,
    mut room: ResMut<CurrentRoom>,
    mut camera: ResMut<CameraController>,
    mut feedback: ResMut<FeedbackText>,
    mut pending_entry: ResMut<crate::navigation::PendingRoomEntry>,
    mut room_events: MessageWriter<crate::navigation::RoomTransition>,
    mut pending_states: ResMut<PendingEntityStates>,
) {
    for _ in reader.read() {
        let Ok(data) = read_save() else {
            feedback.0 = "No save file found.".into();
            return;
        };

        let Ok(save) = ron::from_str::<SaveGame>(&data) else {
            feedback.0 = "Save file is corrupted.".into();
            return;
        };

        // Restore inventory
        inv.items.clear();
        for item in &save.items {
            inv.add_item(&item.item_id, &item.name, "");
            // Set correct quantity
            if let Some(existing) = inv.items.iter_mut().find(|i| i.item_id == item.item_id) {
                existing.qty = item.qty;
            }
        }

        // Restore objective states (overlay onto loaded data)
        for saved in &save.objective_states {
            if let Some(obj) = objectives.all.iter_mut().find(|o| o.id == saved.id) {
                obj.visible = saved.visible;
                obj.completed = saved.completed;
            }
        }

        // Restore camera
        camera.history = save.camera_history.clone();
        camera.current_spot = save.camera_spot.clone();

        // Trigger room transition to load the saved room
        // (this will despawn current scene and load the saved room's .glb)
        if room.name != save.current_room {
            let entry_spot = save.camera_spot.clone().unwrap_or_default();
            room_events.write(crate::navigation::RoomTransition {
                target_room: save.current_room.clone(),
                entry_spot,
            });
        } else if let Some(ref spot) = save.camera_spot {
            pending_entry.spot_name = Some(spot.clone());
        }

        // Store entity states for application after the room scene loads
        pending_states.entity_states = save.entity_states;
        pending_states.collected_entities = save.collected_entities;

        feedback.0 = "Game loaded.".into();
    }
}

/// Apply saved entity states after the room scene finishes loading.
fn apply_pending_entity_states(
    mut pending: ResMut<PendingEntityStates>,
    names_q: Query<(Entity, &Name)>,
    mut state_q: Query<&mut ObjectState>,
    mut vis_q: Query<&mut Visibility>,
    contained_q: Query<(Entity, &crate::components::ContainedIn)>,
    mut commands: Commands,
) {
    if pending.entity_states.is_empty() && pending.collected_entities.is_empty() {
        return;
    }

    let mut applied = 0;

    // Collect name→entity mapping for matching
    let entities: Vec<(Entity, String)> = names_q
        .iter()
        .map(|(e, n)| (e, n.as_str().to_string()))
        .collect();

    // Apply ObjectState changes
    for (entity, name) in &entities {
        if let Some(pos) = pending
            .entity_states
            .iter()
            .position(|(n, _)| n == name)
        {
            let state_str = pending.entity_states[pos].1.clone();
            if let Some(new_state) = parse_object_state(&state_str) {
                if let Ok(mut obj_state) = state_q.get_mut(*entity) {
                    *obj_state = new_state;

                    // If container is Open, reveal contained items
                    if new_state == ObjectState::Open {
                        for (item_entity, contained) in contained_q.iter() {
                            if contained.container == *entity {
                                if let Ok(mut item_vis) = vis_q.get_mut(item_entity) {
                                    *item_vis = Visibility::Inherited;
                                }
                            }
                        }
                    }
                }
            }
            applied += 1;
        }

        // Apply collected (hidden + remove clickable)
        if pending.collected_entities.iter().any(|n| n == name) {
            if let Ok(mut vis) = vis_q.get_mut(*entity) {
                *vis = Visibility::Hidden;
            }
            commands.entity(*entity).remove::<Clickable>();
            applied += 1;
        }
    }

    // Clear once entities are present in the scene
    if applied > 0 || entities.len() > 5 {
        pending.entity_states.clear();
        pending.collected_entities.clear();
    }
}

pub struct SavePlugin;

impl Plugin for SavePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<SaveExists>()
            .add_message::<SaveRequested>()
            .add_message::<LoadRequested>()
            .add_systems(Startup, check_save_exists)
            .init_resource::<PendingEntityStates>()
            .add_systems(Update, (perform_save, perform_load, apply_pending_entity_states));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn save_game_serializes() {
        let save = SaveGame {
            current_room: "study".into(),
            camera_spot: Some("room_overview".into()),
            camera_history: vec!["room_overview".into()],
            items: vec![SavedItem {
                item_id: "flashlight".into(),
                name: "Flashlight".into(),
                qty: 1,
            }],
            objective_states: vec![SavedObjective {
                id: "explore".into(),
                visible: true,
                completed: true,
            }],
            entity_states: vec![("Drawer".into(), "Open".into())],
            collected_entities: vec!["Flashlight".into()],
        };

        let ron = ron::ser::to_string_pretty(&save, Default::default());
        assert!(ron.is_ok());

        let parsed: Result<SaveGame, _> = ron::from_str(&ron.unwrap());
        assert!(parsed.is_ok());
        assert_eq!(parsed.unwrap().current_room, "study");
    }
}
