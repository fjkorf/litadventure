//! Screenshot test for the procedural test scene.
//!
//! Usage:
//!   # First run: generate reference screenshot
//!   cargo run --example screenshot_test -- --save
//!
//!   # Subsequent runs: compare against reference
//!   cargo run --example screenshot_test
//!
//!   # Update reference (overwrite existing)
//!   cargo run --example screenshot_test -- --save
//!
//! The reference image is saved to `tests/screenshots/scene_reference.png`.
//! The comparison uses a per-pixel tolerance to account for minor GPU differences.

use bevy::{
    prelude::*,
    render::view::screenshot::{Screenshot, ScreenshotCaptured},
};
use std::path::Path;

use litadventure::camera::{CameraController, PlayerCamera};
use litadventure::components::*;

const REFERENCE_PATH: &str = "tests/screenshots/scene_reference.png";
const WARMUP_FRAMES: u32 = 10;
const WINDOW_WIDTH: u32 = 1280;
const WINDOW_HEIGHT: u32 = 720;
/// Maximum percentage of pixels that can differ before the test fails.
const MAX_DIFF_PERCENT: f64 = 2.0;
/// Per-channel tolerance for pixel comparison (0-255 scale).
const CHANNEL_TOLERANCE: u8 = 5;

#[derive(Resource)]
struct TestConfig {
    save_mode: bool,
}

#[derive(Resource, Default)]
struct FrameCounter(u32);

/// Set by the screenshot observer to signal the app should exit.
#[derive(Resource)]
struct TestResult(Option<i32>);

fn main() {
    let save_mode = std::env::args().any(|a| a == "--save");

    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Screenshot Test".into(),
                resolution: (WINDOW_WIDTH, WINDOW_HEIGHT).into(),
                ..default()
            }),
            ..default()
        }))
        .add_plugins(bevy_tweening::TweeningPlugin)
        .register_type::<Clickable>()
        .register_type::<CameraSpot>()
        .register_type::<InventoryItem>()
        .register_type::<ObjectState>()
        .register_type::<NavigatesTo>()
        .register_type::<ParentSpot>()
        .init_resource::<CameraController>()
        .init_resource::<FrameCounter>()
        .insert_resource(TestConfig { save_mode })
        .insert_resource(TestResult(None))
        .add_systems(Startup, spawn_test_scene)
        .add_systems(Update, (capture_after_warmup, check_exit).chain())
        .run();
}

/// Exit the process once the screenshot observer has set a result.
fn check_exit(result: Res<TestResult>) {
    if let Some(code) = result.0 {
        std::process::exit(code);
    }
}

/// Spawn the same test scene as the game, but without UI overlay.
fn spawn_test_scene(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut camera_ctrl: ResMut<CameraController>,
) {
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

    commands.spawn((
        Mesh3d(meshes.add(Plane3d::new(Vec3::Y, Vec2::splat(5.0)))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgb(0.3, 0.25, 0.2),
            ..default()
        })),
        Name::new("Floor"),
    ));

    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(10.0, 5.0, 0.1))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgb(0.6, 0.55, 0.5),
            ..default()
        })),
        Transform::from_xyz(0.0, 2.5, -5.0),
        Name::new("BackWall"),
    ));

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

    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(0.6, 0.25, 0.5))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgb(0.5, 0.35, 0.18),
            ..default()
        })),
        Transform::from_xyz(0.0, 0.55, -1.8),
        Name::new("Drawer"),
        Clickable {
            label: "Drawer".into(),
            description: "A small desk drawer.".into(),
        },
        ObjectState::Closed,
    ));

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
            description: "Rows of old books.".into(),
        },
    ));

    camera_ctrl.current_spot = Some("room_overview".into());
}

fn capture_after_warmup(
    mut commands: Commands,
    mut counter: ResMut<FrameCounter>,
    config: Res<TestConfig>,
    result: Res<TestResult>,
) {
    // Don't capture if we already have a result or haven't warmed up
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

/// Process the captured screenshot: save or compare. Returns exit code.
fn process_screenshot(image: &Image, save_mode: bool) -> i32 {
    let captured = image
        .clone()
        .try_into_dynamic()
        .expect("Failed to convert screenshot to dynamic image");
    let captured_rgba = captured.to_rgba8();

    if save_mode {
        std::fs::create_dir_all("tests/screenshots")
            .expect("Failed to create screenshots directory");
        captured_rgba
            .save(REFERENCE_PATH)
            .expect("Failed to save reference screenshot");
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
        println!("No reference screenshot found at {}.", REFERENCE_PATH);
        println!("Run with --save first to generate a reference:");
        println!("  cargo run --example screenshot_test -- --save");
        return 1;
    }

    let reference = image::open(ref_path)
        .expect("Failed to open reference screenshot")
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
        let diff_path = "tests/screenshots/scene_diff.png";
        captured_rgba
            .save(diff_path)
            .expect("Failed to save diff screenshot");
        println!(
            "FAIL: {:.2}% of pixels differ (threshold: {:.1}%).",
            diff_pct, MAX_DIFF_PERCENT
        );
        println!("Captured frame saved to {} for inspection.", diff_path);
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
