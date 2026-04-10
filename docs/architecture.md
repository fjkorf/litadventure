# Architecture

## Engine / Data Separation

LitAdventure separates generic engine code from game-specific content:

| Layer | Format | What lives here | Tool |
|-------|--------|-----------------|------|
| **Spatial** | glTF (`.glb`) | Meshes, materials, transforms, per-entity components | Blender + bevy_skein |
| **Logic** | RON (`.ron`) | Objectives, hints, recipes, room metadata | bevy_common_assets |
| **Engine** | Rust (`.rs`) | Systems, state machines, UI rendering, interaction logic | cargo build |
| **UI** | Markdown (`.md`) | Panel layouts, styles, widgets | litui |

## Module Map

```
main.rs ─── Plugins ──┬── CameraPlugin (camera.rs)
         │            ├── ScenePlugin (scene.rs)
         │            ├── InteractionPlugin (interaction.rs)
         │            ├── InventoryPlugin (inventory.rs)
         │            ├── ObjectivesPlugin (objectives.rs)
         │            ├── HintsPlugin (hints.rs)
         │            ├── NavigationPlugin (navigation.rs)
         │            ├── StatesPlugin (states.rs)
         │            ├── GameDataPlugin (game_data.rs)
         │            ├── DebugPlugin (debug.rs)
         │            ├── SkeinPlugin (bevy_skein)
         │            ├── MeshPickingPlugin (bevy)
         │            ├── EguiPlugin (bevy_egui)
         │            └── TweeningPlugin (bevy_tweening)
         │
         ├── UI Sync ──┬── sync_room_info (on CurrentRoom change)
         │             ├── sync_feedback (on FeedbackText change)
         │             ├── sync_objectives (on Objectives change)
         │             ├── sync_inventory (on Inventory change)
         │             ├── sync_hints (on HintText change)
         │             └── sync_win (on GameState change)
         │
         └── render_ui ── EguiPrimaryContextPass schedule
```

## Game State Flow

```
GameState::Loading ──(assets ready)──> GameState::Playing
                                            │
                                       P key / Esc
                                            │
                                       GameState::Paused
                                            │
                                        Resume
                                            │
                                       GameState::Playing
                                            │
                                       GameWon message
                                            │
                                       GameState::Won
```

## Interaction Flow

```
Player clicks 3D object
    │
    ├─ Pointer<Click> observer fires on_click()
    │
    ├─ Entity has Portal? ──> RoomTransition message ──> camera tweens to new room
    │
    ├─ Entity has InventoryItem? ──> ItemPickedUp message ──> item added to Inventory
    │
    ├─ Entity has ObjectState::Locked + RequiresItem?
    │     ├─ Player has item? ──> Unlock, ObjectiveCompleted
    │     └─ No item? ──> fail_message in FeedbackText
    │
    ├─ Entity has ObjectState::Unlocked? ──> GameWon message
    │
    ├─ Entity has ObjectState::Closed? ──> Toggle to Open, reveal ContainedIn items, tween
    ├─ Entity has ObjectState::Open? ──> Toggle to Closed, hide items, tween
    │
    └─ Entity has NavigatesTo? ──> Camera tweens to CameraSpot, PlayerNavigated message
```

## Message Flow

Messages are Bevy's `Message` type (replacing the older `Event` system in 0.18):

| Message | Fired by | Consumed by |
|---------|----------|-------------|
| `PlayerNavigated` | interaction.rs (on camera navigation) | objectives.rs, hints.rs |
| `ItemPickedUp` | interaction.rs (on item collection) | inventory.rs, hints.rs |
| `ObjectiveCompleted` | objectives.rs (auto), interaction.rs (manual) | objectives.rs, hints.rs |
| `RoomTransition` | interaction.rs (portal click) | navigation.rs |
| `GameWon` | interaction.rs (unlocked door click) | main.rs |

## Data Loading Lifecycle

1. **Startup**: `GameDataPlugin` loads all `.ron` files via `AssetServer`
2. **Each frame**: `check_game_data_ready` polls asset readiness
3. **Once ready**: `GameDataReady(true)` set, one-shot loader systems run:
   - `load_objectives_from_data` populates `Objectives` resource
   - `load_hints_from_data` populates `HintDatabase` resource
   - `load_recipes_from_data` populates `Inventory` recipes
   - `load_rooms_from_data` populates `RoomRegistry` + `CurrentRoom`
4. **Hot-reload**: `file_watcher` detects changes to `.ron` files and reloads assets

## UI Architecture

UI is defined in litui markdown pages (`content/*.md`), compiled to egui code at build time:

- `define_litui_app!` macro generates `Page` enum + `AppState` struct
- `Page` and `AppState` are made Bevy Resources with `impl Resource`
- Sync systems copy game state into `AppState` fields
- `render_ui` renders egui panels in `EguiPrimaryContextPass` schedule
- No `CentralPanel` — 3D scene shows through uncovered viewport area
- Picking events pass through to Bevy's `MeshPickingPlugin`

## Scene Architecture

The procedural test scene (`scene.rs`) spawns all entities at startup. This will be replaced by glTF loading:

- `study.glb` contains meshes + bevy_skein component extras
- `SkeinPlugin` deserializes components from glTF extras at load time
- `resolve_contained_in` system resolves `ContainedInName(String)` to `ContainedIn(Entity)` by matching `Name` components
- Camera spots use tiny invisible meshes so Bevy spawns them with `GltfExtras`
