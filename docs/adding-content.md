# Adding Content

## Adding a New Room

### 1. Create the scene

In Blender (or `tools/generate_study.py`):
- Model the room geometry (floor, walls, objects)
- Add `Clickable` components to interactive objects
- Add `CameraSpot` components to camera positions (can be Blender Empties, no mesh needed)
- Add `ParentSpot` to CameraSpots that have a parent
- Add at least one `Portal` to connect to another room
- Export as `assets/scenes/your_room.glb`

### 2. Add to level manifest

Edit `assets/data/demo.level.ron`:
```ron
(
    name: "Demo",
    starting_room: "study",
    rooms: {
        "study": (scene: "scenes/study.glb"),
        "hallway": (scene: "scenes/hallway.glb"),
        "cellar": (scene: "scenes/cellar.glb"),  // Add this line
    },
)
```

### 3. Add room metadata

Edit `assets/data/rooms.ron`:
```ron
(
    rooms: [
        // ... existing rooms ...
        (
            name: "cellar",
            display_name: "The Cellar",
            description: "A damp stone room. Water drips from the ceiling.",
            starting_spot: Some("cellar_overview"),
        ),
    ],
    starting_room: "study",
)
```

### 4. Connect rooms with portals

In the source room's scene, add a `Portal` component to a door object:
- `target_room`: `"cellar"` (matches the `name` in rooms.ron)
- `entry_spot`: `"cellar_overview"` (matches a CameraSpot in the cellar scene)

In the cellar scene, add a portal back:
- `target_room`: `"study"`
- `entry_spot`: `"room_overview"`

## Adding a New Item

### 1. Create the object

In Blender, add a mesh for the item. Attach components:
- `Clickable` — label and description
- `InventoryItem` — name, description, item_id

If the item is hidden inside a container (like the flashlight in the drawer):
- `ContainedInName` — set `container_name` to the container's `Name`
- The item's `Visibility` should start as `Hidden`

### 2. Use in objectives (optional)

Edit `assets/data/objectives.ron` to add an objective that requires the item:
```ron
(
    id: "find_key",
    text: "Find the key.",
    visible: false,
    conflicts_with: [],
    unlocks: ["open_cellar"],
    requires_item: Some("key"),
    requires_navigation: None,
),
```

### 3. Use in recipes (optional)

Edit `assets/data/recipes.ron`:
```ron
(
    item_a: "key",
    item_b: "oil",
    result_id: "oiled_key",
    result_name: "Oiled Key",
    result_description: "A key that turns smoothly now.",
),
```

## Adding a New Objective

Edit `assets/data/objectives.ron`. An objective needs:
- A unique `id`
- Display `text`
- `visible: false` (unless it's the starting objective)
- At least one objective in the chain with `unlocks: ["your_id"]` to reveal it

### Completion triggers

Objectives complete automatically when:
- **`requires_item`**: The named item is in the player's inventory
- **`requires_navigation`**: The player navigates to the named CameraSpot

For objectives that complete on other conditions (like using an item on a locked door), the interaction system fires `ObjectiveCompleted` manually. This currently requires a Rust code change.

## Adding New Hints

Edit `assets/data/hints.ron`. Add an entry keyed by the objective's `id`:

```ron
"find_key": (
    "Something shiny might be hidden.",     // Tier 1 (30s idle)
    "Check under the furniture.",           // Tier 2 (60s idle)
    "Look under the rug near the fireplace.", // Tier 3 (90s idle)
),
```

Hint thresholds can be tuned globally via `tier_thresholds`.

## Adding a Locked Door

Attach these components to the door object:
- `Clickable` — with description text
- `ObjectState` — set to `Locked`
- `RequiresItem` — specify the item_id, use_message, and fail_message

When the player clicks it with the required item, it transitions to `Unlocked`. Clicking an `Unlocked` door fires `GameWon` (this behavior is hardcoded for the demo; future versions will make this data-driven).

**Game over detection**: If the player clicks a `RequiresItem` door without the item AND the current room has no `Portal` entity (no way back), the game triggers `GameOver`. Always ensure rooms with locked doors have a portal back to another room where the player can find the required item.

## Adding a Container (Drawer, Chest, etc.)

Attach these components to the container:
- `Clickable` — description text
- `ObjectState` — set to `Closed`
- `TweenConfig` — set `open_offset` (how far it slides) and `duration_ms`

Items inside the container need:
- `ContainedInName` — set `container_name` to the container's Name
- Start with `Visibility::Hidden`

When the container opens, contained items become visible and clickable.

## Adding a New UI Page

1. Create `content/your_page.md` with YAML frontmatter:
```markdown
---
page:
  name: YourPage
  label: Your Page
  panel: window
  open: show_your_page
---

Your litui content here.
```

2. Add it to the `define_litui_app!` macro in `src/main.rs`
3. Add rendering logic in `render_ui`
4. Add a sync system for the `show_your_page` field if needed
