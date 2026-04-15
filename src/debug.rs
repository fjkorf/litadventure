use bevy::math::bounding::Aabb3d;
use bevy::prelude::*;

use crate::camera::CameraController;
use crate::components::*;
use crate::interaction::PlayState;
use crate::navigation::Portal;

/// Toggle for the debug overlay.
#[derive(Resource, Default)]
pub struct DebugOverlay {
    pub enabled: bool,
}

/// Toggle debug mode with configurable key (default F1).
fn toggle_debug(
    input: Res<ButtonInput<KeyCode>>,
    config: Res<crate::input_config::InputConfig>,
    mut debug: ResMut<DebugOverlay>,
) {
    if input.just_pressed(config.debug_overlay) {
        debug.enabled = !debug.enabled;
    }
}

/// Draw wireframe boxes around all Clickable entities with color-coded types.
fn draw_click_zones(
    debug: Res<DebugOverlay>,
    clickable_q: Query<(
        &Clickable,
        &GlobalTransform,
        &Mesh3d,
        Option<&ObjectState>,
        Option<&InventoryItem>,
        Option<&NavigatesTo>,
        Option<&Portal>,
        Option<&Visibility>,
    )>,
    meshes: Res<Assets<Mesh>>,
    mut gizmos: Gizmos,
) {
    if !debug.enabled {
        return;
    }

    for (clickable, gt, mesh_handle, obj_state, inv_item, nav, portal, vis) in clickable_q.iter() {
        if vis == Some(&Visibility::Hidden) {
            continue;
        }

        // Color by interaction type
        let color = if portal.is_some() {
            Color::srgb(1.0, 0.5, 0.0) // Orange: portal
        } else if inv_item.is_some() {
            Color::srgb(0.0, 1.0, 0.0) // Green: collectible
        } else if nav.is_some() {
            Color::srgb(0.0, 0.5, 1.0) // Blue: navigable
        } else if obj_state.is_some() {
            Color::srgb(1.0, 1.0, 0.0) // Yellow: stateful
        } else {
            Color::srgb(1.0, 1.0, 1.0) // White: examinable
        };

        let transform = gt.compute_transform();

        // Get AABB from mesh, or use a default
        // Get AABB from mesh, or use a default
        let aabb = if let Some(mesh) = meshes.get(&mesh_handle.0) {
            mesh.final_aabb
                .unwrap_or(Aabb3d::new(Vec3::ZERO, Vec3::splat(0.3)))
        } else {
            Aabb3d::new(Vec3::ZERO, Vec3::splat(0.3))
        };

        gizmos.aabb_3d(aabb, transform, color);

        let half = Vec3::from(aabb.max - aabb.min) * 0.5;

        // Label marker cross above the box
        let label_pos = transform.translation + Vec3::Y * (half.y * transform.scale.y + 0.2);
        gizmos.cross(Isometry3d::from_translation(label_pos), 0.1, color);

        // State indicator: small sphere for stateful objects
        if let Some(state) = obj_state {
            let state_color = match state {
                ObjectState::Closed => Color::srgb(1.0, 0.0, 0.0),
                ObjectState::Open => Color::srgb(0.0, 1.0, 0.0),
                ObjectState::Locked => Color::srgb(1.0, 0.5, 0.0),
                _ => Color::srgb(0.5, 0.5, 0.5),
            };
            gizmos.sphere(
                Isometry3d::from_translation(label_pos + Vec3::Y * 0.15),
                0.05,
                state_color,
            );
        }
    }
}

/// Draw camera spot positions and look-at lines.
fn draw_camera_spots(
    debug: Res<DebugOverlay>,
    spots: Query<(&CameraSpot, &GlobalTransform)>,
    camera_ctrl: Res<CameraController>,
    mut gizmos: Gizmos,
) {
    if !debug.enabled {
        return;
    }

    for (spot, gt) in spots.iter() {
        let pos = gt.translation();
        let is_current = camera_ctrl.current_spot.as_deref() == Some(spot.name.as_str());

        let color = if is_current {
            Color::srgb(0.0, 1.0, 1.0) // Cyan: current
        } else {
            Color::srgb(0.4, 0.4, 0.4) // Gray: other
        };

        gizmos.cross(Isometry3d::from_translation(pos), 0.25, color);
        gizmos.line(pos, spot.look_at, color.with_alpha(0.3));
    }
}

/// Draw play state indicator at world origin.
fn draw_state_indicator(
    debug: Res<DebugOverlay>,
    play_state: Res<PlayState>,
    mut gizmos: Gizmos,
) {
    if !debug.enabled {
        return;
    }

    let color = match *play_state {
        PlayState::Exploring => Color::srgb(0.0, 1.0, 0.0),
        PlayState::Examining(_) => Color::srgb(1.0, 1.0, 0.0),
        PlayState::Transitioning => Color::srgb(1.0, 0.0, 0.0),
    };

    gizmos.sphere(Isometry3d::from_translation(Vec3::ZERO), 0.08, color);
}

/// Draw a bright ring around the keyboard-focused entity.
fn draw_focus_indicator(
    debug: Res<DebugOverlay>,
    focused: Res<crate::input_intent::FocusedClickable>,
    transforms: Query<&GlobalTransform>,
    mut gizmos: Gizmos,
) {
    if !debug.enabled {
        return;
    }

    if let Some(entity) = focused.entity {
        if let Ok(gt) = transforms.get(entity) {
            let pos = gt.translation();
            // Bright gold ring around the focused entity
            gizmos.sphere(Isometry3d::from_translation(pos), 0.6, Color::srgb(1.0, 0.85, 0.0));
            gizmos.cross(Isometry3d::from_translation(pos + Vec3::Y * 0.8), 0.15, Color::srgb(1.0, 0.85, 0.0));
        }
    }
}

pub struct DebugPlugin;

impl Plugin for DebugPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<DebugOverlay>().add_systems(
            Update,
            (
                toggle_debug,
                draw_click_zones,
                draw_camera_spots,
                draw_state_indicator,
                draw_focus_indicator,
            ),
        );
    }
}
