use bevy::prelude::*;

/// Marks an entity as clickable by the player.
#[derive(Component, Reflect, Debug, Clone, Default)]
#[reflect(Component, Default)]
pub struct Clickable {
    pub label: String,
    pub description: String,
}

/// A named camera position that the camera can tween to.
#[derive(Component, Reflect, Debug, Clone, Default)]
#[reflect(Component, Default)]
pub struct CameraSpot {
    pub name: String,
    pub look_at: Vec3,
}

/// Marks an entity as a collectible inventory item.
#[derive(Component, Reflect, Debug, Clone, Default)]
#[reflect(Component, Default)]
pub struct InventoryItem {
    pub name: String,
    pub description: String,
    pub item_id: String,
}

/// Per-entity state for interactive objects.
#[derive(Component, Reflect, Debug, Clone, Copy, PartialEq, Eq, Default)]
#[reflect(Component, Default)]
pub enum ObjectState {
    #[default]
    Default,
    Locked,
    Unlocked,
    Open,
    Closed,
    Collected,
}

/// Links a clickable object to the camera spot it should navigate to when clicked.
#[derive(Component, Reflect, Debug, Clone, Default)]
#[reflect(Component, Default)]
pub struct NavigatesTo {
    pub spot_name: String,
}

/// Marks the parent camera spot for back-navigation.
#[derive(Component, Reflect, Debug, Clone, Default)]
#[reflect(Component, Default)]
pub struct ParentSpot {
    pub spot_name: String,
}

/// Links an item entity to the container entity it's hidden inside.
/// Resolved at runtime from ContainedInName.
#[derive(Component, Debug, Clone)]
pub struct ContainedIn {
    pub container: Entity,
}

/// String-based container reference for Skein/glTF.
/// Resolved to ContainedIn(Entity) after scene spawn by matching Name components.
#[derive(Component, Reflect, Debug, Clone, Default)]
#[reflect(Component, Default)]
pub struct ContainedInName {
    pub container_name: String,
}

/// Specifies what inventory item is needed to interact with a Locked object.
#[derive(Component, Reflect, Debug, Clone, Default)]
#[reflect(Component, Default)]
pub struct RequiresItem {
    pub item_id: String,
    pub use_message: String,
    pub fail_message: String,
    /// Objective ID to complete when the item is successfully used.
    #[reflect(default)]
    pub completes_objective: String,
}

/// Tween configuration for objects that animate (e.g., drawer open/close).
#[derive(Component, Reflect, Debug, Clone, Default)]
#[reflect(Component, Default)]
pub struct TweenConfig {
    /// Offset to apply when opening (relative to initial position).
    pub open_offset: Vec3,
    /// Animation duration in milliseconds.
    pub duration_ms: u32,
}
