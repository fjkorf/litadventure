use bevy::prelude::*;

use crate::camera::{CameraController, PlayerCamera};
use crate::components::*;
use crate::navigation::{CurrentRoom, Portal};

/// Marker component on the scene root for room cleanup on transitions.
#[derive(Component)]
pub struct RoomSceneMarker;

/// Resolves ContainedInName(String) → ContainedIn(Entity) by matching Name components.
pub fn resolve_contained_in(
    mut commands: Commands,
    unresolved: Query<(Entity, &ContainedInName), Without<ContainedIn>>,
    names: Query<(Entity, &Name)>,
) {
    for (item_entity, cin) in unresolved.iter() {
        for (candidate, name) in names.iter() {
            if name.as_str() == cin.container_name {
                commands
                    .entity(item_entity)
                    .insert(ContainedIn { container: candidate });
                break;
            }
        }
    }
}

/// Hide items that are inside closed containers (initial state after scene load).
pub fn hide_contained_items(
    contained_q: Query<(Entity, &ContainedIn), Added<ContainedIn>>,
    state_q: Query<&ObjectState>,
    mut vis_q: Query<&mut Visibility>,
) {
    for (item_entity, contained) in contained_q.iter() {
        if let Ok(state) = state_q.get(contained.container) {
            if *state == ObjectState::Closed {
                if let Ok(mut vis) = vis_q.get_mut(item_entity) {
                    *vis = Visibility::Hidden;
                }
            }
        }
    }
}

/// Spawn camera and lights (persist across room changes).
fn setup_camera_and_lights(mut commands: Commands) {
    commands.spawn((
        PlayerCamera,
        Camera3d::default(),
        Transform::from_xyz(0.0, 3.0, 6.0).looking_at(Vec3::new(0.0, 1.0, 0.0), Vec3::Y),
    ));

    commands.spawn((
        PointLight {
            shadows_enabled: true,
            intensity: 800_000.0,
            ..default()
        },
        Transform::from_xyz(2.0, 5.0, 3.0),
    ));

    commands.spawn((
        DirectionalLight {
            illuminance: 2_000.0,
            shadows_enabled: true,
            ..default()
        },
        Transform::from_rotation(Quat::from_euler(EulerRot::XYZ, -0.8, 0.3, 0.0)),
    ));
}

/// Load a room's glTF scene. Falls back to procedural scene if .glb doesn't exist.
pub fn load_room_scene(
    commands: &mut Commands,
    asset_server: &AssetServer,
    room_name: &str,
) {
    let path = format!("scenes/{}.glb", room_name);
    let handle = asset_server.load(GltfAssetLabel::Scene(0).from_asset(path));
    commands.spawn((SceneRoot(handle), RoomSceneMarker));
}

/// Despawn all entities belonging to the current room scene.
pub fn despawn_room_scene(commands: &mut Commands, scene_roots: &Query<Entity, With<RoomSceneMarker>>) {
    for entity in scene_roots.iter() {
        commands.entity(entity).despawn();
    }
}

/// Load the initial room scene on startup.
fn load_initial_scene(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    room: Res<CurrentRoom>,
    mut camera_ctrl: ResMut<CameraController>,
) {
    if !room.name.is_empty() {
        load_room_scene(&mut commands, &asset_server, &room.name);
    } else {
        // Fallback: load study if no room set yet
        load_room_scene(&mut commands, &asset_server, "study");
    }
    camera_ctrl.current_spot = Some("room_overview".into());
}

pub struct ScenePlugin;

impl Plugin for ScenePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup_camera_and_lights)
            .add_systems(Startup, load_initial_scene.after(setup_camera_and_lights))
            .add_systems(Update, (resolve_contained_in, hide_contained_items).chain());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn room_scene_marker_is_default() {
        // Verify the marker component can be created
        let _marker = RoomSceneMarker;
    }
}
