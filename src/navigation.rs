use bevy::prelude::*;

use crate::camera::{start_camera_tween, start_camera_tween_to_pos, CameraController, PlayerCamera};
use crate::components::CameraSpot;
use crate::game_data::{GameDataHandles, GameDataReady, LevelManifest, RoomSet};
use crate::scene::{despawn_room_scene, load_room_scene, RoomSceneMarker};

/// Marks an entity as a portal to another room.
#[derive(Component, Reflect, Debug, Clone, Default)]
#[reflect(Component, Default)]
pub struct Portal {
    pub target_room: String,
    pub entry_spot: String,
}

/// Tracks which room the player is currently in.
#[derive(Resource, Debug, Default)]
pub struct CurrentRoom {
    pub name: String,
    pub display_name: String,
    pub description: String,
}

/// All room definitions loaded from data.
#[derive(Resource, Default)]
pub struct RoomRegistry {
    pub rooms: Vec<(String, String, String, Option<String>)>, // (name, display_name, description, starting_spot)
}

impl RoomRegistry {
    pub fn find(&self, name: &str) -> Option<(&str, &str)> {
        self.rooms
            .iter()
            .find(|(n, _, _, _)| n == name)
            .map(|(_, display, desc, _)| (display.as_str(), desc.as_str()))
    }

    pub fn starting_spot(&self, name: &str) -> Option<&str> {
        self.rooms
            .iter()
            .find(|(n, _, _, _)| n == name)
            .and_then(|(_, _, _, spot)| spot.as_deref())
    }
}

/// Message for room transitions (Stage 2: despawn old, load new).
#[derive(Message)]
pub struct RoomTransition {
    pub target_room: String,
    pub entry_spot: String,
}

/// Message fired when a portal is clicked (starts Stage 1: approach the door).
#[derive(Message)]
pub struct PortalApproachRequested {
    pub portal_entity: Entity,
    pub target_room: String,
    pub entry_spot: String,
}

/// Load room definitions from RON data.
fn load_rooms_from_data(
    ready: Res<GameDataReady>,
    handles: Res<GameDataHandles>,
    assets: Res<Assets<RoomSet>>,
    mut registry: ResMut<RoomRegistry>,
    mut current_room: ResMut<CurrentRoom>,
    mut loaded: Local<bool>,
) {
    if *loaded || !ready.0 {
        return;
    }

    let Some(handle) = &handles.rooms else {
        return;
    };
    let Some(data) = assets.get(handle) else {
        return;
    };

    registry.rooms = data
        .rooms
        .iter()
        .map(|r| (r.name.clone(), r.display_name.clone(), r.description.clone(), r.starting_spot.clone()))
        .collect();

    // Set starting room
    if let Some((display, desc)) = registry.find(&data.starting_room) {
        current_room.name = data.starting_room.clone();
        current_room.display_name = display.to_string();
        current_room.description = desc.to_string();
    }

    *loaded = true;
}

/// Tracks a pending camera snap after a room transition.
#[derive(Resource, Default)]
pub struct PendingRoomEntry {
    pub spot_name: Option<String>,
    /// Stored transition to fire after Stage 1 portal approach tween completes.
    pub pending_transition: Option<(String, String)>, // (target_room, entry_spot)
}

/// Process room transitions: despawn old scene, load new scene, update room info.
fn process_room_transitions(
    mut reader: MessageReader<RoomTransition>,
    mut current_room: ResMut<CurrentRoom>,
    registry: Res<RoomRegistry>,
    mut camera_ctrl: ResMut<CameraController>,
    handles: Res<GameDataHandles>,
    levels: Res<Assets<LevelManifest>>,
    scene_roots: Query<Entity, With<RoomSceneMarker>>,
    asset_server: Res<AssetServer>,
    mut pending: ResMut<PendingRoomEntry>,
    mut commands: Commands,
) {
    for ev in reader.read() {
        // Update room metadata
        if let Some((display, desc)) = registry.find(&ev.target_room) {
            current_room.name = ev.target_room.clone();
            current_room.display_name = display.to_string();
            current_room.description = desc.to_string();
        }

        // Despawn current room scene
        despawn_room_scene(&mut commands, &scene_roots);

        // Load new room scene from level manifest
        if let Some(handle) = &handles.level {
            if let Some(manifest) = levels.get(handle) {
                if let Some(room_ref) = manifest.rooms.get(&ev.target_room) {
                    let path = room_ref.scene.clone();
                    let scene_handle = asset_server.load(GltfAssetLabel::Scene(0).from_asset(path));
                    commands.spawn((SceneRoot(scene_handle), RoomSceneMarker));
                }
            }
        }

        // Reset camera and queue a tween to the entry spot
        camera_ctrl.history.clear();
        camera_ctrl.current_spot = Some(ev.entry_spot.clone());
        camera_ctrl.transitioning = true; // block clicks while scene loads + camera tweens
        pending.spot_name = Some(ev.entry_spot.clone());
    }
}

/// After a room loads, tween the camera to the entry spot once the CameraSpot entity exists.
fn tween_camera_to_entry(
    mut pending: ResMut<PendingRoomEntry>,
    spots: Query<(&CameraSpot, &GlobalTransform)>,
    camera_q: Query<(Entity, &Transform), With<PlayerCamera>>,
    mut camera_ctrl: ResMut<CameraController>,
    mut commands: Commands,
) {
    let Some(target_name) = &pending.spot_name else {
        return;
    };

    for (spot, gt) in spots.iter() {
        if spot.name == *target_name {
            let Ok((camera_entity, camera_transform)) = camera_q.single() else {
                return;
            };

            camera_ctrl.transitioning = true;
            start_camera_tween(
                &mut commands,
                camera_entity,
                camera_transform,
                spot,
                gt,
            );
            pending.spot_name = None;
            return;
        }
    }
}

/// Stage 1: Tween camera toward the portal door before transitioning rooms.
fn handle_portal_approach(
    mut reader: MessageReader<PortalApproachRequested>,
    portal_gt_q: Query<&GlobalTransform>,
    camera_q: Query<(Entity, &Transform), With<PlayerCamera>>,
    mut pending: ResMut<PendingRoomEntry>,
    mut camera_ctrl: ResMut<CameraController>,
    mut commands: Commands,
) {
    for ev in reader.read() {
        let Ok(portal_gt) = portal_gt_q.get(ev.portal_entity) else {
            continue;
        };
        let Ok((camera_entity, camera_transform)) = camera_q.single() else {
            continue;
        };

        // Store the transition for after Stage 1 completes
        pending.pending_transition = Some((ev.target_room.clone(), ev.entry_spot.clone()));
        pending.spot_name = Some(ev.entry_spot.clone());

        // Start Stage 1: position-only tween toward the portal door
        camera_ctrl.transitioning = true;
        start_camera_tween_to_pos(
            &mut commands,
            camera_entity,
            camera_transform,
            portal_gt.translation(),
        );
    }
}

/// Fire the deferred RoomTransition once Stage 1 portal approach tween completes.
fn fire_pending_room_transition(
    mut pending: ResMut<PendingRoomEntry>,
    camera_ctrl: Res<CameraController>,
    mut room_events: MessageWriter<RoomTransition>,
) {
    let Some(ref data) = pending.pending_transition else {
        return;
    };
    if camera_ctrl.transitioning {
        return;
    }

    // Stage 1 done — fire the actual room transition (despawn + load)
    room_events.write(RoomTransition {
        target_room: data.0.clone(),
        entry_spot: data.1.clone(),
    });
    pending.pending_transition = None;
}

pub struct NavigationPlugin;

impl Plugin for NavigationPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<CurrentRoom>()
            .init_resource::<RoomRegistry>()
            .init_resource::<PendingRoomEntry>()
            .register_type::<Portal>()
            .add_message::<RoomTransition>()
            .add_message::<PortalApproachRequested>()
            .add_systems(
                Update,
                (
                    load_rooms_from_data,
                    handle_portal_approach,
                    fire_pending_room_transition,
                    process_room_transitions,
                    tween_camera_to_entry,
                ),
            );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn room_registry_find() {
        let mut reg = RoomRegistry::default();
        reg.rooms.push(("study".into(), "The Study".into(), "A room.".into(), Some("room_overview".into())));

        let found = reg.find("study");
        assert!(found.is_some());
        assert_eq!(found.unwrap().0, "The Study");

        assert!(reg.find("nonexistent").is_none());
    }
}
