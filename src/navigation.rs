use bevy::prelude::*;

use crate::camera::{
    find_spot_by_name, start_camera_tween, CameraController, PlayerCamera,
};
use crate::components::CameraSpot;
use crate::game_data::{GameDataHandles, GameDataReady, RoomSet};

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
    pub rooms: Vec<(String, String, String)>, // (name, display_name, description)
}

impl RoomRegistry {
    pub fn find(&self, name: &str) -> Option<(&str, &str)> {
        self.rooms
            .iter()
            .find(|(n, _, _)| n == name)
            .map(|(_, display, desc)| (display.as_str(), desc.as_str()))
    }
}

/// Message for room transitions.
#[derive(Message)]
pub struct RoomTransition {
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
        .map(|r| (r.name.clone(), r.display_name.clone(), r.description.clone()))
        .collect();

    // Set starting room
    if let Some((display, desc)) = registry.find(&data.starting_room) {
        current_room.name = data.starting_room.clone();
        current_room.display_name = display.to_string();
        current_room.description = desc.to_string();
    }

    *loaded = true;
}

/// Process room transitions: update CurrentRoom and tween camera to entry spot.
fn process_room_transitions(
    mut reader: MessageReader<RoomTransition>,
    mut current_room: ResMut<CurrentRoom>,
    registry: Res<RoomRegistry>,
    mut camera_ctrl: ResMut<CameraController>,
    camera_q: Query<(Entity, &Transform), With<PlayerCamera>>,
    spot_q: Query<(&CameraSpot, &GlobalTransform, Entity)>,
    mut commands: Commands,
) {
    for ev in reader.read() {
        if let Some((display, desc)) = registry.find(&ev.target_room) {
            current_room.name = ev.target_room.clone();
            current_room.display_name = display.to_string();
            current_room.description = desc.to_string();
        }

        camera_ctrl.history.clear();
        camera_ctrl.current_spot = Some(ev.entry_spot.clone());

        let Ok((camera_entity, camera_transform)) = camera_q.single() else {
            continue;
        };

        if let Some((spot, spot_gt, _)) =
            find_spot_by_name(spot_q.iter(), &ev.entry_spot)
        {
            camera_ctrl.transitioning = true;
            start_camera_tween(
                &mut commands,
                camera_entity,
                camera_transform,
                &spot,
                &spot_gt,
            );
        }
    }
}

pub struct NavigationPlugin;

impl Plugin for NavigationPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<CurrentRoom>()
            .init_resource::<RoomRegistry>()
            .register_type::<Portal>()
            .add_message::<RoomTransition>()
            .add_systems(Update, (load_rooms_from_data, process_room_transitions));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn room_registry_find() {
        let mut reg = RoomRegistry::default();
        reg.rooms.push(("study".into(), "The Study".into(), "A room.".into()));

        let found = reg.find("study");
        assert!(found.is_some());
        assert_eq!(found.unwrap().0, "The Study");

        assert!(reg.find("nonexistent").is_none());
    }
}
