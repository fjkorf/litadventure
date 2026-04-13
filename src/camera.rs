use bevy::prelude::*;
use bevy_tweening::{lens::*, *};
use std::time::Duration;

use crate::components::CameraSpot;

/// Marker for the player camera.
#[derive(Component)]
pub struct PlayerCamera;

/// Tracks camera navigation state.
#[derive(Resource, Default)]
pub struct CameraController {
    /// Stack of visited spot names for back-navigation.
    pub history: Vec<String>,
    /// Name of the current camera spot.
    pub current_spot: Option<String>,
    /// True while a camera transition is in progress.
    pub transitioning: bool,
}

impl CameraController {
    pub fn navigate_to(&mut self, spot_name: &str) {
        if let Some(current) = &self.current_spot {
            self.history.push(current.clone());
        }
        self.current_spot = Some(spot_name.to_string());
        self.transitioning = true;
    }

    pub fn navigate_back(&mut self) -> Option<String> {
        if let Some(previous) = self.history.pop() {
            self.current_spot = Some(previous.clone());
            self.transitioning = true;
            Some(previous)
        } else {
            None
        }
    }

    pub fn can_go_back(&self) -> bool {
        !self.history.is_empty()
    }
}

/// Duration of camera transitions in seconds.
const CAMERA_TWEEN_DURATION: f32 = 1.2;

/// Starts a camera tween toward the target CameraSpot.
pub fn start_camera_tween(
    commands: &mut Commands,
    camera_entity: Entity,
    camera_transform: &Transform,
    spot: &CameraSpot,
    spot_transform: &GlobalTransform,
) {
    let start_pos = camera_transform.translation;
    let end_pos = spot_transform.translation();

    let start_rot = camera_transform.rotation;
    let end_rot = Transform::from_translation(end_pos)
        .looking_at(spot.look_at, Vec3::Y)
        .rotation;

    let move_tween = Tween::new(
        EaseFunction::CubicInOut,
        Duration::from_secs_f32(CAMERA_TWEEN_DURATION),
        TransformPositionLens {
            start: start_pos,
            end: end_pos,
        },
    );

    let rotate_tween = Tween::new(
        EaseFunction::CubicInOut,
        Duration::from_secs_f32(CAMERA_TWEEN_DURATION),
        TransformRotationLens {
            start: start_rot,
            end: end_rot,
        },
    );

    // Remove any existing tweens on the camera, then add new ones
    commands.entity(camera_entity).insert((
        TweenAnim::new(move_tween),
    ));

    // Spawn a separate entity for rotation tween targeting the camera
    commands.spawn((
        TweenAnim::new(rotate_tween),
        AnimTarget::component::<Transform>(camera_entity),
    ));
}

/// Starts a position-only camera tween (no rotation change).
/// Used for the portal approach stage where we just move toward the door.
pub fn start_camera_tween_to_pos(
    commands: &mut Commands,
    camera_entity: Entity,
    camera_transform: &Transform,
    end_pos: Vec3,
) {
    let tween = Tween::new(
        EaseFunction::CubicInOut,
        Duration::from_secs_f32(CAMERA_TWEEN_DURATION * 0.6), // slightly faster for approach
        TransformPositionLens {
            start: camera_transform.translation,
            end: end_pos,
        },
    );

    commands.entity(camera_entity).insert(TweenAnim::new(tween));
}

/// Finds a CameraSpot entity by name.
pub fn find_spot_by_name<'a>(
    spots: impl Iterator<Item = (&'a CameraSpot, &'a GlobalTransform, Entity)>,
    name: &str,
) -> Option<(CameraSpot, GlobalTransform, Entity)> {
    spots
        .filter(|(spot, _, _)| spot.name == name)
        .map(|(spot, gt, e)| (spot.clone(), *gt, e))
        .next()
}

/// System that detects camera tween completion by checking if TweenAnim is gone.
/// bevy_tweening removes TweenAnim on completion by default (destroy_on_completed).
pub fn detect_tween_complete(
    camera_q: Query<(), (With<PlayerCamera>, Without<TweenAnim>)>,
    mut controller: ResMut<CameraController>,
) {
    if controller.transitioning && camera_q.single().is_ok() {
        controller.transitioning = false;
    }
}

pub struct CameraPlugin;

impl Plugin for CameraPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<CameraController>()
            .add_systems(Update, detect_tween_complete);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn camera_controller_navigate_to() {
        let mut ctrl = CameraController::default();
        assert!(!ctrl.can_go_back());
        assert!(ctrl.current_spot.is_none());

        ctrl.navigate_to("room_overview");
        assert_eq!(ctrl.current_spot.as_deref(), Some("room_overview"));
        assert!(!ctrl.can_go_back());
        assert!(ctrl.transitioning);

        ctrl.transitioning = false;
        ctrl.navigate_to("desk_closeup");
        assert_eq!(ctrl.current_spot.as_deref(), Some("desk_closeup"));
        assert!(ctrl.can_go_back());
        assert_eq!(ctrl.history.len(), 1);
        assert_eq!(ctrl.history[0], "room_overview");
    }

    #[test]
    fn camera_controller_navigate_back() {
        let mut ctrl = CameraController::default();
        ctrl.navigate_to("room_overview");
        ctrl.transitioning = false;
        ctrl.navigate_to("desk_closeup");
        ctrl.transitioning = false;
        ctrl.navigate_to("drawer_detail");
        ctrl.transitioning = false;

        assert_eq!(ctrl.history.len(), 2);

        let back = ctrl.navigate_back();
        assert_eq!(back.as_deref(), Some("desk_closeup"));
        assert_eq!(ctrl.current_spot.as_deref(), Some("desk_closeup"));
        assert!(ctrl.transitioning);

        ctrl.transitioning = false;
        let back = ctrl.navigate_back();
        assert_eq!(back.as_deref(), Some("room_overview"));
        assert!(!ctrl.can_go_back());

        let back = ctrl.navigate_back();
        assert!(back.is_none());
    }

    #[test]
    fn find_spot_by_name_works() {
        let spots = vec![
            (
                CameraSpot {
                    name: "room".into(),
                    look_at: Vec3::ZERO,
                },
                GlobalTransform::from_xyz(0.0, 5.0, 10.0),
                Entity::from_bits(1),
            ),
            (
                CameraSpot {
                    name: "desk".into(),
                    look_at: Vec3::new(1.0, 0.5, 0.0),
                },
                GlobalTransform::from_xyz(1.0, 2.0, 3.0),
                Entity::from_bits(2),
            ),
        ];

        let iter = spots
            .iter()
            .map(|(s, g, e)| (s, g, *e));

        let found = find_spot_by_name(iter, "desk");
        assert!(found.is_some());
        let (spot, gt, _) = found.unwrap();
        assert_eq!(spot.name, "desk");
        assert_eq!(gt.translation(), Vec3::new(1.0, 2.0, 3.0));

        let iter2 = spots
            .iter()
            .map(|(s, g, e)| (s, g, *e));
        let not_found = find_spot_by_name(iter2, "nonexistent");
        assert!(not_found.is_none());
    }
}
