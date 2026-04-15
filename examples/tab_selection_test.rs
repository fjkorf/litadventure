//! Screenshot test verifying the visual appearance of Tab-selected (keyboard-focused) objects.
//!
//! Usage:
//!   cargo run --example tab_selection_test -- --save   # generate reference
//!   cargo run --example tab_selection_test             # compare against reference
//!
//! The Desk entity is programmatically set as keyboard-focused with the warm yellow
//! emissive glow (0.2, 0.2, 0.05, 1.0). Non-focused entities (Drawer, Bookshelf)
//! should have no emissive.

use bevy::{
    prelude::*,
    render::view::screenshot::{Screenshot, ScreenshotCaptured},
};
use std::path::Path;

use litadventure::camera::PlayerCamera;
use litadventure::components::*;
use litadventure::input_intent::{FocusedClickable, KeyboardFocusHighlight};

const REFERENCE_PATH: &str = "tests/screenshots/tab_selection_reference.png";
const WARMUP_FRAMES: u32 = 10;
const WINDOW_WIDTH: u32 = 1280;
const WINDOW_HEIGHT: u32 = 720;
const MAX_DIFF_PERCENT: f64 = 2.0;
const CHANNEL_TOLERANCE: u8 = 5;

#[derive(Resource)]
struct TestConfig {
    save_mode: bool,
}

#[derive(Resource, Default)]
struct FrameCounter(u32);

#[derive(Resource)]
struct TestResult(Option<i32>);

fn main() {
    let save_mode = std::env::args().any(|a| a == "--save");

    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Tab Selection Test".into(),
                resolution: (WINDOW_WIDTH, WINDOW_HEIGHT).into(),
                ..default()
            }),
            ..default()
        }))
        .add_plugins(bevy_tweening::TweeningPlugin)
        .register_type::<Clickable>()
        .register_type::<CameraSpot>()
        .register_type::<ObjectState>()
        .init_resource::<FocusedClickable>()
        .init_resource::<FrameCounter>()
        .insert_resource(TestConfig { save_mode })
        .insert_resource(TestResult(None))
        .add_systems(Startup, (spawn_scene, apply_selection).chain())
        .add_systems(Update, (capture_after_warmup, check_exit).chain())
        .run();
}

fn spawn_scene(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // Camera
    commands.spawn((
        PlayerCamera,
        Camera3d::default(),
        Transform::from_xyz(0.0, 3.0, 6.0).looking_at(Vec3::new(0.0, 1.0, 0.0), Vec3::Y),
    ));

    // Lighting
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

    // Floor
    commands.spawn((
        Mesh3d(meshes.add(Plane3d::new(Vec3::Y, Vec2::splat(5.0)))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgb(0.3, 0.25, 0.2),
            ..default()
        })),
        Name::new("Floor"),
    ));

    // Back wall
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(10.0, 5.0, 0.1))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgb(0.6, 0.55, 0.5),
            ..default()
        })),
        Transform::from_xyz(0.0, 2.5, -5.0),
        Name::new("BackWall"),
    ));

    // Desk — will be Tab-selected (focused)
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(2.0, 0.8, 1.0))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgb(0.45, 0.3, 0.15),
            ..default()
        })),
        Transform::from_xyz(0.0, 0.8, -2.0),
        Name::new("Desk"),
        Clickable {
            label: "Desk".into(),
            description: "A sturdy wooden desk.".into(),
        },
    ));

    // Drawer — NOT selected (no emissive)
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
            description: "A drawer.".into(),
        },
        ObjectState::Closed,
    ));

    // Bookshelf — NOT selected (no emissive)
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
            description: "Old books.".into(),
        },
    ));
}

/// Programmatically apply Tab-selection state to the Desk entity.
fn apply_selection(
    mut commands: Commands,
    query: Query<(Entity, &Name, &MeshMaterial3d<StandardMaterial>)>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut focused: ResMut<FocusedClickable>,
) {
    for (entity, name, mat_handle) in &query {
        if name.as_str() == "Desk" {
            // Apply the exact emissive the Tab cycle system uses
            if let Some(mat) = materials.get_mut(&mat_handle.0) {
                mat.emissive = LinearRgba::new(0.2, 0.2, 0.05, 1.0);
            }
            commands.entity(entity).insert(KeyboardFocusHighlight);
            focused.entity = Some(entity);
            focused.ordered = vec![entity];
        }
    }
}

fn capture_after_warmup(
    mut commands: Commands,
    mut counter: ResMut<FrameCounter>,
    config: Res<TestConfig>,
    result: Res<TestResult>,
) {
    if result.0.is_some() {
        return;
    }

    counter.0 += 1;
    if counter.0 != WARMUP_FRAMES {
        return;
    }

    let save_mode = config.save_mode;
    commands
        .spawn(Screenshot::primary_window())
        .observe(move |event: On<ScreenshotCaptured>, mut result: ResMut<TestResult>| {
            let exit_code = process_screenshot(&event.image, save_mode);
            result.0 = Some(exit_code);
        });
}

fn check_exit(result: Res<TestResult>) {
    if let Some(code) = result.0 {
        std::process::exit(code);
    }
}

fn process_screenshot(image: &Image, save_mode: bool) -> i32 {
    let captured = image
        .clone()
        .try_into_dynamic()
        .expect("Failed to convert screenshot");
    let captured_rgba = captured.to_rgba8();

    if save_mode {
        std::fs::create_dir_all("tests/screenshots").ok();
        captured_rgba
            .save(REFERENCE_PATH)
            .expect("Failed to save reference");
        println!(
            "Reference screenshot saved to {} ({}x{})",
            REFERENCE_PATH,
            captured_rgba.width(),
            captured_rgba.height()
        );
        return 0;
    }

    let ref_path = Path::new(REFERENCE_PATH);
    if !ref_path.exists() {
        println!("No reference found at {}.", REFERENCE_PATH);
        println!("Run with --save first.");
        return 1;
    }

    let reference = image::open(ref_path)
        .expect("Failed to open reference")
        .to_rgba8();

    if captured_rgba.dimensions() != reference.dimensions() {
        println!(
            "FAIL: Resolution mismatch. Captured: {:?}, Reference: {:?}",
            captured_rgba.dimensions(),
            reference.dimensions()
        );
        return 1;
    }

    let total_pixels = captured_rgba.width() as u64 * captured_rgba.height() as u64;
    let mut diff_pixels = 0u64;

    for (c, r) in captured_rgba.pixels().zip(reference.pixels()) {
        if !pixels_similar(c, r, CHANNEL_TOLERANCE) {
            diff_pixels += 1;
        }
    }

    let diff_pct = diff_pixels as f64 / total_pixels as f64 * 100.0;

    if diff_pct > MAX_DIFF_PERCENT {
        let diff_path = "tests/screenshots/tab_selection_diff.png";
        captured_rgba.save(diff_path).ok();
        println!(
            "FAIL: {:.2}% of pixels differ (threshold: {:.1}%).",
            diff_pct, MAX_DIFF_PERCENT
        );
        println!("Captured saved to {} for inspection.", diff_path);
        1
    } else {
        println!(
            "PASS: {:.2}% of pixels differ (threshold: {:.1}%).",
            diff_pct, MAX_DIFF_PERCENT
        );
        0
    }
}

fn pixels_similar(a: &image::Rgba<u8>, b: &image::Rgba<u8>, tolerance: u8) -> bool {
    a.0.iter()
        .zip(b.0.iter())
        .all(|(ac, bc)| ac.abs_diff(*bc) <= tolerance)
}
