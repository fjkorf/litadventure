//! Test loading study.gltf and study.glb to verify Skein component data.
//!
//! Usage:
//!   cargo run --example load_gltf_test -- --gltf   # test .gltf format
//!   cargo run --example load_gltf_test -- --glb    # test .glb format
//!   cargo run --example load_gltf_test             # defaults to .glb

use bevy::prelude::*;
use bevy_skein::SkeinPlugin;

use litadventure::camera::PlayerCamera;
use litadventure::components::*;
use litadventure::navigation::Portal;

#[derive(Resource)]
struct TestConfig {
    asset_path: String,
}

#[derive(Resource, Default)]
struct FrameCount(u32);

fn main() {
    let use_gltf = std::env::args().any(|a| a == "--gltf");
    let asset_path = if use_gltf {
        "scenes/study.gltf"
    } else {
        "scenes/study.glb"
    };

    println!("Loading: {asset_path}");

    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: format!("glTF Load Test: {asset_path}"),
                ..default()
            }),
            ..default()
        }))
        .add_plugins(MeshPickingPlugin)
        .add_plugins(SkeinPlugin { handle_brp: false })
        .register_type::<Clickable>()
        .register_type::<CameraSpot>()
        .register_type::<InventoryItem>()
        .register_type::<ObjectState>()
        .register_type::<NavigatesTo>()
        .register_type::<ParentSpot>()
        .register_type::<ContainedInName>()
        .register_type::<RequiresItem>()
        .register_type::<TweenConfig>()
        .register_type::<Portal>()
        .insert_resource(TestConfig {
            asset_path: asset_path.to_string(),
        })
        .init_resource::<FrameCount>()
        .add_systems(Startup, setup)
        .add_systems(Update, check_components)
        .run();
}

fn setup(mut commands: Commands, asset_server: Res<AssetServer>, config: Res<TestConfig>) {
    // Camera
    commands.spawn((
        PlayerCamera,
        Camera3d::default(),
        Transform::from_xyz(0.0, 3.0, 6.0).looking_at(Vec3::new(0.0, 1.0, 0.0), Vec3::Y),
    ));

    // Light
    commands.spawn((
        PointLight {
            shadows_enabled: true,
            intensity: 800_000.0,
            ..default()
        },
        Transform::from_xyz(2.0, 5.0, 3.0),
    ));

    // Load the glTF scene
    let path = config.asset_path.clone();
    commands.spawn(SceneRoot(
        asset_server.load(
            GltfAssetLabel::Scene(0).from_asset(path),
        ),
    ));
}

fn check_components(
    mut frame_count: ResMut<FrameCount>,
    clickables: Query<(&Name, &Clickable)>,
    spots: Query<(&Name, &CameraSpot)>,
    items: Query<(&Name, &InventoryItem)>,
    portals: Query<(&Name, &Portal)>,
    states: Query<(&Name, &ObjectState)>,
    nav: Query<(&Name, &NavigatesTo)>,
    requires: Query<(&Name, &RequiresItem)>,
    contained: Query<(&Name, &ContainedInName)>,
    tweens: Query<(&Name, &TweenConfig)>,
) {
    frame_count.0 += 1;

    // Wait for scene to load
    if frame_count.0 != 30 {
        return;
    }

    println!("\n=== Component Check (frame 30) ===\n");

    println!("Clickable entities:");
    for (name, c) in clickables.iter() {
        println!("  {} - label: {:?}", name, c.label);
    }

    println!("\nCameraSpot entities:");
    for (name, s) in spots.iter() {
        println!("  {} - name: {:?}, look_at: {:?}", name, s.name, s.look_at);
    }

    println!("\nInventoryItem entities:");
    for (name, i) in items.iter() {
        println!("  {} - item_id: {:?}", name, i.item_id);
    }

    println!("\nPortal entities:");
    for (name, p) in portals.iter() {
        println!("  {} - target: {:?}, entry: {:?}", name, p.target_room, p.entry_spot);
    }

    println!("\nObjectState entities:");
    for (name, s) in states.iter() {
        println!("  {} - {:?}", name, s);
    }

    println!("\nNavigatesTo entities:");
    for (name, n) in nav.iter() {
        println!("  {} - spot: {:?}", name, n.spot_name);
    }

    println!("\nRequiresItem entities:");
    for (name, r) in requires.iter() {
        println!("  {} - item: {:?}", name, r.item_id);
    }

    println!("\nContainedInName entities:");
    for (name, c) in contained.iter() {
        println!("  {} - container: {:?}", name, c.container_name);
    }

    println!("\nTweenConfig entities:");
    for (name, t) in tweens.iter() {
        println!("  {} - offset: {:?}, duration: {}ms", name, t.open_offset, t.duration_ms);
    }

    // Verify expected counts
    let n_clickable = clickables.iter().count();
    let n_spots = spots.iter().count();
    let n_items = items.iter().count();
    let n_portals = portals.iter().count();

    println!("\n=== Summary ===");
    println!("Clickables: {n_clickable} (expected 10)");
    println!("CameraSpots: {n_spots} (expected 4)");
    println!("InventoryItems: {n_items} (expected 3)");
    println!("Portals: {n_portals} (expected 2)");

    let pass = n_clickable == 10 && n_spots == 4 && n_items == 3 && n_portals == 2;
    if pass {
        println!("\nPASS: All components loaded correctly.");
    } else {
        println!("\nFAIL: Component counts don't match expected values.");
    }

    println!();
    std::process::exit(if pass { 0 } else { 1 });
}
