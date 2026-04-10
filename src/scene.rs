use bevy::prelude::*;

use crate::camera::{CameraController, PlayerCamera};
use crate::components::*;
use crate::navigation::Portal;

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

/// Spawns a procedural test scene: a study with desk, drawer, and flashlight.
/// This is a placeholder until real Blender/Skein assets are available.
pub fn spawn_test_scene(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut camera_ctrl: ResMut<CameraController>,
) {
    // -- Camera --
    commands.spawn((
        PlayerCamera,
        Camera3d::default(),
        Transform::from_xyz(0.0, 3.0, 6.0).looking_at(Vec3::new(0.0, 1.0, 0.0), Vec3::Y),
    ));

    // -- Lighting --
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

    // -- Floor --
    commands.spawn((
        Mesh3d(meshes.add(Plane3d::new(Vec3::Y, Vec2::splat(5.0)))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgb(0.3, 0.25, 0.2),
            ..default()
        })),
        Transform::IDENTITY,
        Name::new("Floor"),
    ));

    // -- Back wall --
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(10.0, 5.0, 0.1))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgb(0.6, 0.55, 0.5),
            ..default()
        })),
        Transform::from_xyz(0.0, 2.5, -5.0),
        Name::new("BackWall"),
    ));

    // -- Desk (clickable, navigates to desk_closeup) --
    let desk_material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.45, 0.3, 0.15),
        ..default()
    });

    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(2.0, 0.8, 1.0))),
        MeshMaterial3d(desk_material.clone()),
        Transform::from_xyz(0.0, 0.8, -2.0),
        Name::new("Desk"),
        Clickable {
            label: "Desk".into(),
            description: "A sturdy wooden desk. Its surface is worn smooth.".into(),
        },
        NavigatesTo {
            spot_name: "desk_closeup".into(),
        },
    ));

    // -- Drawer (clickable, toggles open/closed, protrudes from desk front) --
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(0.6, 0.2, 0.4))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgb(0.5, 0.35, 0.18),
            ..default()
        })),
        Transform::from_xyz(0.0, 0.55, -1.3),
        Name::new("Drawer"),
        Clickable {
            label: "Drawer".into(),
            description: "A small desk drawer with a brass handle.".into(),
        },
        ObjectState::Closed,
        TweenConfig {
            open_offset: Vec3::new(0.0, 0.0, 0.4),
            duration_ms: 400,
        },
    ));

    // -- Flashlight (inventory item, hidden inside drawer) --
    commands.spawn((
        Mesh3d(meshes.add(Cylinder::new(0.05, 0.3))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgb(0.7, 0.7, 0.2),
            emissive: LinearRgba::new(0.5, 0.5, 0.1, 1.0),
            ..default()
        })),
        Transform::from_xyz(0.0, 0.7, -1.4),
        Visibility::Hidden,
        Name::new("Flashlight"),
        Clickable {
            label: "Flashlight".into(),
            description: "A small flashlight. It still works.".into(),
        },
        InventoryItem {
            name: "Flashlight".into(),
            description: "A small flashlight. It still works.".into(),
            item_id: "flashlight".into(),
        },
        ContainedInName {
            container_name: "Drawer".into(),
        },
    ));

    // -- Bookshelf (clickable, background detail) --
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(1.5, 3.0, 0.4))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgb(0.35, 0.22, 0.1),
            ..default()
        })),
        Transform::from_xyz(-3.0, 1.5, -4.5),
        Name::new("Bookshelf"),
        Clickable {
            label: "Bookshelf".into(),
            description: "Rows of old books. Most are too faded to read.".into(),
        },
    ));

    // -- Camera Spots --
    commands.spawn((
        CameraSpot {
            name: "room_overview".into(),
            look_at: Vec3::new(0.0, 1.0, 0.0),
        },
        Transform::from_xyz(0.0, 3.0, 6.0),
        GlobalTransform::default(),
        Name::new("CameraSpot_RoomOverview"),
    ));

    commands.spawn((
        CameraSpot {
            name: "desk_closeup".into(),
            look_at: Vec3::new(0.0, 0.8, -2.0),
        },
        Transform::from_xyz(0.0, 1.8, 0.5),
        GlobalTransform::default(),
        ParentSpot {
            spot_name: "room_overview".into(),
        },
        Name::new("CameraSpot_DeskCloseup"),
    ));

    commands.spawn((
        CameraSpot {
            name: "drawer_detail".into(),
            look_at: Vec3::new(0.0, 0.55, -1.8),
        },
        Transform::from_xyz(0.0, 1.2, -0.5),
        GlobalTransform::default(),
        ParentSpot {
            spot_name: "desk_closeup".into(),
        },
        Name::new("CameraSpot_DrawerDetail"),
    ));

    // -- Door to hallway (portal) --
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(0.8, 2.0, 0.1))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgb(0.4, 0.28, 0.12),
            ..default()
        })),
        Transform::from_xyz(3.0, 1.0, -4.9),
        Name::new("Door"),
        Clickable {
            label: "Door".into(),
            description: "A wooden door leading to the hallway.".into(),
        },
        Portal {
            target_room: "hallway".into(),
            entry_spot: "hallway_overview".into(),
        },
    ));

    // -- Hallway geometry --
    commands.spawn((
        Mesh3d(meshes.add(Plane3d::new(Vec3::Y, Vec2::new(3.0, 8.0)))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgb(0.25, 0.2, 0.18),
            ..default()
        })),
        Transform::from_xyz(0.0, 0.0, -14.0),
        Name::new("HallwayFloor"),
    ));

    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(6.0, 4.0, 0.1))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgb(0.5, 0.48, 0.44),
            ..default()
        })),
        Transform::from_xyz(0.0, 2.0, -22.0),
        Name::new("HallwayEndWall"),
    ));

    // -- Painting (clickable, hallway decoration) --
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(0.8, 0.6, 0.05))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgb(0.2, 0.3, 0.5),
            ..default()
        })),
        Transform::from_xyz(-2.5, 1.8, -14.0),
        Name::new("Painting"),
        Clickable {
            label: "Painting".into(),
            description: "A faded landscape painting. Something is written on the back: '42'.".into(),
        },
    ));

    // -- Lens (collectible, near painting) --
    commands.spawn((
        Mesh3d(meshes.add(Cylinder::new(0.08, 0.02))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgb(0.6, 0.8, 0.9),
            ..default()
        })),
        Transform::from_xyz(-2.0, 0.8, -13.5),
        Name::new("Lens"),
        Clickable {
            label: "Lens".into(),
            description: "A small glass lens, slightly dusty.".into(),
        },
        InventoryItem {
            name: "Lens".into(),
            description: "A small glass lens.".into(),
            item_id: "lens".into(),
        },
    ));

    // -- Frame (collectible, on hallway floor) --
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(0.12, 0.02, 0.06))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgb(0.5, 0.4, 0.2),
            ..default()
        })),
        Transform::from_xyz(1.0, 0.05, -12.0),
        Name::new("Frame"),
        Clickable {
            label: "Frame".into(),
            description: "A small brass frame. Looks like it once held a lens.".into(),
        },
        InventoryItem {
            name: "Frame".into(),
            description: "A small brass frame.".into(),
            item_id: "frame".into(),
        },
    ));

    // -- Locked door (needs flashlight to see keyhole) --
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(0.8, 2.0, 0.1))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgb(0.3, 0.2, 0.1),
            ..default()
        })),
        Transform::from_xyz(2.5, 1.0, -18.0),
        Name::new("LockedDoor"),
        Clickable {
            label: "Locked Door".into(),
            description: "A heavy door. It's too dark to see the lock clearly.".into(),
        },
        ObjectState::Locked,
        RequiresItem {
            item_id: "flashlight".into(),
            use_message: "You shine the flashlight on the lock. It clicks open.".into(),
            fail_message: "It's too dark to see the lock clearly.".into(),
        },
    ));

    // -- Hallway camera spots --
    commands.spawn((
        CameraSpot {
            name: "hallway_overview".into(),
            look_at: Vec3::new(0.0, 1.0, -14.0),
        },
        Transform::from_xyz(0.0, 2.5, -8.0),
        GlobalTransform::default(),
        Name::new("CameraSpot_HallwayOverview"),
    ));

    // -- Door back to study (portal) --
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(0.8, 2.0, 0.1))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgb(0.4, 0.28, 0.12),
            ..default()
        })),
        Transform::from_xyz(-2.5, 1.0, -8.5),
        Name::new("DoorToStudy"),
        Clickable {
            label: "Door".into(),
            description: "The door back to the study.".into(),
        },
        Portal {
            target_room: "study".into(),
            entry_spot: "room_overview".into(),
        },
    ));

    // Set initial camera spot
    camera_ctrl.current_spot = Some("room_overview".into());
}

pub struct ScenePlugin;

impl Plugin for ScenePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, spawn_test_scene)
            .add_systems(Update, resolve_contained_in);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scene_spawns_expected_entities() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.add_plugins(AssetPlugin::default());
        app.init_asset::<Mesh>();
        app.init_asset::<StandardMaterial>();
        app.init_asset::<Image>();
        app.init_resource::<CameraController>();
        app.add_systems(Startup, spawn_test_scene);
        app.update();

        let world = app.world_mut();

        // Verify camera exists
        let cameras: Vec<_> = world
            .query_filtered::<Entity, With<PlayerCamera>>()
            .iter(world)
            .collect();
        assert_eq!(cameras.len(), 1, "Expected 1 player camera");

        // Verify clickable entities
        let clickables: Vec<_> = world
            .query_filtered::<&Name, With<Clickable>>()
            .iter(world)
            .collect();
        let clickable_names: Vec<&str> = clickables.iter().map(|n| n.as_str()).collect();
        assert!(
            clickable_names.contains(&"Desk"),
            "Desk should be clickable"
        );
        assert!(
            clickable_names.contains(&"Drawer"),
            "Drawer should be clickable"
        );
        assert!(
            clickable_names.contains(&"Flashlight"),
            "Flashlight should be clickable"
        );
        assert!(
            clickable_names.contains(&"Bookshelf"),
            "Bookshelf should be clickable"
        );

        // Verify camera spots
        let spots: Vec<_> = world
            .query::<(&CameraSpot, &Name)>()
            .iter(world)
            .collect();
        let spot_names: Vec<&str> = spots.iter().map(|(s, _)| s.name.as_str()).collect();
        assert!(spot_names.contains(&"room_overview"));
        assert!(spot_names.contains(&"desk_closeup"));
        assert!(spot_names.contains(&"drawer_detail"));

        // Verify inventory items
        let items: Vec<_> = world
            .query_filtered::<&InventoryItem, ()>()
            .iter(world)
            .collect();
        assert_eq!(items.len(), 3, "Expected 3 inventory items (flashlight, lens, frame)");
        let item_ids: Vec<&str> = items.iter().map(|i| i.item_id.as_str()).collect();
        assert!(item_ids.contains(&"flashlight"));
        assert!(item_ids.contains(&"lens"));
        assert!(item_ids.contains(&"frame"));

        // Verify camera controller initialized
        let ctrl = world.resource::<CameraController>();
        assert_eq!(ctrl.current_spot.as_deref(), Some("room_overview"));
    }
}
