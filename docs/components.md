# Component Reference

All game components are defined in `src/components.rs` and `src/navigation.rs`. They derive `Reflect` + `Component` + `Default` for bevy_skein compatibility, meaning they can be attached to objects in Blender and exported via glTF.

## Clickable

Marks an entity as interactive. Required for any object the player can click.

```rust
Clickable {
    label: String,       // Short name shown in debug overlay
    description: String, // Feedback text when clicked
}
```

**Used by**: Desk, Drawer, Flashlight, Bookshelf, Doors, Painting, Lens, Frame, Locked Door

## CameraSpot

Defines a named camera position. The camera tweens to this position when triggered.

```rust
CameraSpot {
    name: String,    // Unique ID (e.g., "desk_closeup")
    look_at: Vec3,   // World position the camera looks toward
}
```

The entity's `Transform` determines where the camera moves to. The `look_at` field determines the camera's rotation target.

**Used by**: Room overview, desk closeup, drawer detail, hallway overview

## NavigatesTo

Links a clickable object to a CameraSpot. Clicking the object tweens the camera to that spot.

```rust
NavigatesTo {
    spot_name: String, // Name of the target CameraSpot
}
```

**Used by**: Desk (navigates to "desk_closeup")

## ParentSpot

Marks the parent camera spot for back-navigation. When the player presses Escape or right-clicks, the camera returns to this spot.

```rust
ParentSpot {
    spot_name: String, // Name of the parent CameraSpot
}
```

**Used by**: desk_closeup (parent: room_overview), drawer_detail (parent: desk_closeup)

## InventoryItem

Marks an entity as collectible. When clicked, the item is added to the player's inventory and the entity is hidden.

```rust
InventoryItem {
    name: String,        // Display name in inventory panel
    description: String, // Description when examined
    item_id: String,     // Unique ID for recipes and objective requirements
}
```

**Used by**: Flashlight, Lens, Frame

## ObjectState

Per-entity state enum for interactive objects. Controls behavior in the click handler.

```rust
enum ObjectState {
    Default,    // No special behavior
    Locked,     // Requires an item to unlock (see RequiresItem)
    Unlocked,   // Was locked, now open — clicking triggers GameWon
    Open,       // Container is open (drawer slid out)
    Closed,     // Container is closed (drawer default)
    Collected,  // Item has been collected
}
```

**Behavior in click handler**:
- `Closed` -> toggles to `Open`, reveals ContainedIn items, plays tween animation
- `Open` -> toggles to `Closed`, hides items, plays reverse tween
- `Locked` -> checks RequiresItem, transitions to `Unlocked` if player has the item
- `Unlocked` -> fires GameWon message (end of game)

**Used by**: Drawer (Closed), Locked Door (Locked)

## RequiresItem

Specifies what inventory item is needed to interact with a Locked object.

```rust
RequiresItem {
    item_id: String,     // Required item's ID (e.g., "flashlight")
    use_message: String, // Feedback when player has the item
    fail_message: String, // Feedback when player doesn't have the item
}
```

**Used by**: Locked Door (requires "flashlight")

## ContainedInName

String-based container reference for glTF/Skein. Resolved to `ContainedIn(Entity)` at runtime by matching `Name` components.

```rust
ContainedInName {
    container_name: String, // Name of the container entity (e.g., "Drawer")
}
```

When the container opens (ObjectState::Closed -> Open), entities with `ContainedIn` pointing to it become visible.

**Used by**: Flashlight (contained in "Drawer")

## ContainedIn

Runtime-resolved entity reference. Not exported to glTF (Entity IDs aren't stable). Created automatically from `ContainedInName` by the `resolve_contained_in` system.

```rust
ContainedIn {
    container: Entity, // The container entity
}
```

## TweenConfig

Animation configuration for objects that open/close (drawers, panels, etc.).

```rust
TweenConfig {
    open_offset: Vec3, // Translation offset when opening
    duration_ms: u32,  // Animation duration in milliseconds
}
```

The open animation moves from current position to `position + open_offset`. The close animation reverses.

**Used by**: Drawer (offset: `[0, 0, 0.4]`, duration: 400ms)

## Portal

Marks an entity as a doorway to another room. Clicking triggers a `RoomTransition` message.

```rust
Portal {
    target_room: String, // Room name (matches rooms.ron)
    entry_spot: String,  // CameraSpot name to start at in the target room
}
```

Defined in `src/navigation.rs`.

**Used by**: Door (study -> hallway), DoorToStudy (hallway -> study)

## Skein / glTF Compatibility

All components (except `ContainedIn`) can be attached to Blender objects via bevy_skein. See [blender-workflow.md](blender-workflow.md) for setup and the import extension.

Key format notes for the `BEVY_skein` glTF extension:

- `Vec3` fields serialize as arrays: `[0.0, 1.0, 0.0]` (NOT `{"x": 0, "y": 1, "z": 0}`)
- `ObjectState` enum variants serialize as strings: `"Closed"`, `"Locked"`, etc.
- `String` fields serialize as JSON strings
- `u32` fields serialize as JSON numbers
- Type paths use the full Rust path: `"litadventure::components::Clickable"`

The Skein import extension supports round-tripping: import a `.glb` with `BEVY_skein` data, edit components in Blender, re-export. Unit-variant enums (like ObjectState) are preserved correctly.
