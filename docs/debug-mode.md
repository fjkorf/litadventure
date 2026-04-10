# Debug Mode

Press **F1** to toggle the debug overlay. It draws wireframe gizmos over interactive entities and camera positions.

## Wireframe Colors (Click Zones)

| Color | Meaning | Components Present |
|-------|---------|-------------------|
| Orange | Portal (room transition) | `Clickable` + `Portal` |
| Green | Collectible item | `Clickable` + `InventoryItem` |
| Blue | Navigable object | `Clickable` + `NavigatesTo` |
| Yellow | Stateful object (drawer, door) | `Clickable` + `ObjectState` |
| White | Examinable only | `Clickable` (no other interaction) |

Each clickable entity gets a wireframe box matching its mesh AABB, plus a cross marker above it.

## State Indicators

Small colored spheres appear above stateful objects showing their current `ObjectState`:

| Sphere Color | State |
|-------------|-------|
| Red | Closed |
| Green | Open |
| Orange | Locked |

## Camera Spots

| Gizmo | Color | Meaning |
|-------|-------|---------|
| Cross (0.25 units) | Cyan | Current camera spot |
| Cross (0.25 units) | Gray | Other camera spots |
| Line from cross | Same, faded | Points to the spot's `look_at` target |

## Play State Indicator

A small sphere at the world origin `(0, 0, 0)` shows the current `PlayState`:

| Color | State |
|-------|-------|
| Green | Exploring |
| Yellow | Examining |
| Red | Transitioning (camera moving) |

## Hidden Entities

Hidden entities (`Visibility::Hidden`) are not drawn in the debug overlay. This includes items inside closed containers (like the flashlight before the drawer opens).

## Usage Tips

- Use debug mode to verify click zones don't overlap (e.g., drawer inside desk)
- Check that camera spots have correct look_at lines pointing to their targets
- Verify portals are positioned where players would expect doors
- Confirm all interactive objects have the expected wireframe color
