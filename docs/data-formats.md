# Data File Formats

Game data lives in `assets/data/` as RON files. These are loaded at startup via `bevy_common_assets` and support hot-reload while the app is running.

RON (Rusty Object Notation) is similar to Rust literal syntax. Key differences from JSON:
- Structs use `()` not `{}`
- Fixed-size arrays use `()`: `("a", "b", "c")` for `[String; 3]`
- Variable-length lists use `[]`: `["a", "b"]` for `Vec<String>`
- Maps use `{}`: `{ "key": "value" }`
- `Option` uses `Some("value")` or `None`
- Trailing commas are allowed
- Comments with `//` are supported

## objectives.ron

Defines the objective graph: what the player needs to do, in what order, and what triggers completion.

```ron
(
    objectives: [
        (
            id: "explore",                          // Unique identifier
            text: "Explore the study.",              // Display text in UI
            visible: true,                           // Shown to player at start?
            conflicts_with: [],                      // IDs of objectives removed when this completes
            unlocks: ["find_flashlight"],             // IDs of objectives revealed when this completes
            requires_item: None,                     // Item ID that auto-completes this (Option<String>)
            requires_navigation: Some("desk_closeup"), // CameraSpot name that auto-completes this
        ),
        (
            id: "find_flashlight",
            text: "Find the flashlight.",
            visible: false,                          // Hidden until unlocked by "explore"
            conflicts_with: [],
            unlocks: ["explore_hallway"],
            requires_item: Some("flashlight"),        // Completes when "flashlight" is in inventory
            requires_navigation: None,
        ),
        // ... more objectives
    ],
)
```

**Fields**:

| Field | Type | Description |
|-------|------|-------------|
| `id` | String | Unique identifier, referenced by `unlocks` and `conflicts_with` |
| `text` | String | Shown in the objectives panel |
| `visible` | bool | If `false`, hidden until another objective's `unlocks` reveals it |
| `conflicts_with` | Vec\<String\> | When this objective completes, listed objectives are removed |
| `unlocks` | Vec\<String\> | When this objective completes, listed objectives become visible |
| `requires_item` | Option\<String\> | Auto-completes when the named item is in the player's inventory |
| `requires_navigation` | Option\<String\> | Auto-completes when the player navigates to the named CameraSpot |

Objectives can also be completed manually by the interaction system (e.g., `unlock_door` is completed when the player clicks a Locked door with the right item).

## hints.ron

Defines progressive hints for each objective. Hints appear after the player has been idle.

```ron
(
    hints: {
        "explore": (                                // Key = objective ID
            "What catches your eye?",               // Tier 1: vague nudge (30s)
            "The desk might be worth a closer look.", // Tier 2: specific suggestion (60s)
            "Click the desk to examine it.",          // Tier 3: direct solution (90s)
        ),
        "find_flashlight": (
            "It must be nearby.",
            "Check inside the furniture.",
            "Open the drawer on the desk.",
        ),
    },
    tier_thresholds: (30.0, 60.0, 90.0),           // Seconds of idle time for each tier
)
```

**Fields**:

| Field | Type | Description |
|-------|------|-------------|
| `hints` | HashMap\<String, [String; 3]\> | Maps objective ID to 3 hint strings (tier 1, 2, 3) |
| `tier_thresholds` | [f32; 3] | Seconds of idle time before each tier becomes available |

Note: Fixed-size arrays (`[String; 3]`, `[f32; 3]`) use `()` in RON, not `[]`.

The hint timer resets whenever the player does something meaningful (navigates, picks up an item, or completes an objective).

## recipes.ron

Defines item combination recipes.

```ron
(
    recipes: [
        (
            item_a: "lens",                         // First ingredient item_id
            item_b: "frame",                        // Second ingredient item_id
            result_id: "magnifying_glass",           // Result item_id
            result_name: "Magnifying Glass",         // Result display name
            result_description: "A small magnifying glass.", // Result description
        ),
    ],
)
```

Combination is order-independent (`lens + frame` = `frame + lens`). Both ingredients are consumed and replaced by the result.

## rooms.ron

Defines room metadata (display names and descriptions for the UI).

```ron
(
    rooms: [
        (
            name: "study",                          // Internal name (matches Portal.target_room)
            display_name: "The Study",               // Shown in room info panel
            description: "A quiet room lined with bookshelves.", // Shown below room name
            starting_spot: Some("room_overview"),    // CameraSpot name for room entry
        ),
        (
            name: "hallway",
            display_name: "The Hallway",
            description: "A dim hallway with a worn carpet.",
            starting_spot: Some("hallway_overview"),
        ),
    ],
    starting_room: "study",                         // Which room the game begins in
)
```

## demo.level.ron

The level manifest maps room names to their glTF scene files. It tells the engine which `.glb` to load when entering each room.

```ron
(
    name: "Demo",                                    // Level display name
    starting_room: "study",                          // First room loaded on game start
    rooms: {
        "study": (scene: "scenes/study.glb"),        // Room name → scene file path
        "hallway": (scene: "scenes/hallway.glb"),
    },
)
```

**Fields**:

| Field | Type | Description |
|-------|------|-------------|
| `name` | String | Display name of the level |
| `starting_room` | String | Room loaded at game start (matches a key in `rooms`) |
| `rooms` | HashMap\<String, RoomSceneRef\> | Maps room names to scene file paths |

Room names must match:
- The `name` field in `rooms.ron`
- The `target_room` field on `Portal` components in glTF scenes

## game_save.ron (generated at runtime)

Save files are written to `assets/saves/game_save.ron` when the player clicks Save in the pause menu. The format is:

```ron
(
    current_room: "hallway",
    camera_spot: Some("hallway_overview"),
    camera_history: [],
    items: [
        (item_id: "flashlight", name: "Flashlight", qty: 1),
    ],
    objective_states: [
        (id: "explore", visible: true, completed: true),
        (id: "find_flashlight", visible: true, completed: true),
    ],
    entity_states: [
        ("Drawer", "Open"),
        ("LockedDoor", "Unlocked"),
    ],
    collected_entities: ["Flashlight"],
)
```

This file should not be hand-edited. It's managed by the save/load system (`src/save.rs`).

## Adding New Data

To add a new objective, add an entry to `objectives.ron`, optionally add hints for it in `hints.ron`, and ensure something triggers its completion (an item requirement, a navigation requirement, or a manual `ObjectiveCompleted` message from the interaction system).

See [adding-content.md](adding-content.md) for complete walkthroughs.
