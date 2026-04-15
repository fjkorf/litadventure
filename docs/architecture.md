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
         │            ├── InputConfigPlugin (input_config.rs)
         │            ├── InputIntentPlugin (input_intent.rs)
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
         │             └── sync_overlays (on GameState change)
         │
         └── drive_overlay_focus ──> render_ui ── EguiPrimaryContextPass schedule (chained)
```

## Game State Flow

```
GameState::Title ──(Start button)──> GameState::Loading ──(data ready)──> GameState::Playing
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
                                                                          GameState::Won ──(Restart)──> GameState::Title

                                                                          GameState::Playing
                                                                               │
                                                                          Failed RequiresItem + no Portal
                                                                               │
                                                                          GameState::GameOver ──(Restart)──> GameState::Title

Title also supports: Continue button ──> Load save ──> GameState::Loading
```

## Interaction Flow

```
Player clicks 3D object
    │
    ├─ Pointer<Click> observer fires on_click()
    │
    ├─ [Input guarded by EguiWantsInput — egui panels block 3D clicks]
    │
    ├─ Entity has Portal? ──> PortalApproachRequested ──> three-stage tween (see below)
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
| `PortalApproachRequested` | interaction.rs (portal click) | navigation.rs (Stage 1 tween) |
| `RoomTransition` | navigation.rs (after Stage 1), main.rs (restart) | navigation.rs (despawn/respawn scene) |
| `GameWon` | interaction.rs (unlocked door click) | main.rs |
| `GameOver` | interaction.rs (stuck detection) | main.rs |
| `SaveRequested` | main.rs (Save button) | save.rs |
| `LoadRequested` | main.rs (Continue button) | save.rs |
| `PreviewRequested` | main.rs (combine result) | item_preview.rs |

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

Each room is a separate `.glb` file loaded via `SceneRoot` + `RoomSceneMarker`:

- `study.glb` and `hallway.glb` contain meshes + component data via `BEVY_skein` extension
- `SkeinPlugin` deserializes components from the glTF extension at load time
- Camera and lights are spawned once at startup and persist across room transitions
- `resolve_contained_in` resolves `ContainedInName(String)` → `ContainedIn(Entity)` by matching `Name` components
- `hide_contained_items` hides items inside closed containers after scene load
- Camera spots can be empty nodes (no mesh needed with the BEVY_skein extension path)

### Three-Stage Portal Tween

When a portal is clicked, the camera transition happens in stages:

1. **Stage 1 (approach)**: `PortalApproachRequested` → `handle_portal_approach` starts a position-only tween toward the door entity's world position. Old room stays visible.
2. **Scene swap**: `fire_pending_room_transition` detects Stage 1 complete → fires `RoomTransition`. `process_room_transitions` despawns old scene, loads new `.glb` from `LevelManifest`.
3. **Stage 2 (entry)**: `tween_camera_to_entry` polls for the new room's `CameraSpot` entity → starts a full position + rotation tween to the entry spot.

`detect_tween_complete` (camera.rs) clears `camera_ctrl.transitioning` between stages.

### Inventory Previews

Items render to 128x128 off-screen textures via `RenderLayers::layer(1)`:
- `register_item_meshes` captures mesh/material handles when `InventoryItem` entities spawn
- `spawn_preview_on_pickup` creates a camera + render target per item on `ItemPickedUp`
- Combined items (no scene mesh) get a fallback sphere preview via `PreviewRequested`
- Displayed in egui via `EguiUserTextures` + `SizedTexture`
- Drag-to-combine: each item is both a `dnd_drag_source` and `dnd_drop_zone`

### Game Over Detection

`check_stuck` (interaction.rs) fires `GameOver` when:
- `FeedbackText` matches a `RequiresItem.fail_message`
- The player doesn't have the required item
- No `Portal` entity exists in the current scene (no exit)

If a portal exists, the player can go back — no game over.

## Save/Load

Game state is serialized to `assets/saves/game_save.ron` via the `SaveGame` struct:
- Inventory items, objective states (visible/completed), current room, camera position/history
- Entity states (`ObjectState`) and collected entity names
- Triggered by Save button (pause menu) / Continue button (title screen)
- Uses `ron` crate for serialization, `std::fs` for file I/O

### Entity State Persistence

On load, `PendingEntityStates` holds the saved entity states. `apply_pending_entity_states` polls each frame until the room scene finishes loading, then:
- Restores `ObjectState` on named entities (e.g., Drawer → Open)
- Reveals contained items if their container is Open
- Hides collected items and removes their `Clickable` component

## Input Abstraction

Input is processed in two tiers:

### Tier 1: InputIntent (input_intent.rs)

`produce_input_intents` runs in `PreUpdate`, reads raw `ButtonInput<KeyCode>` + `ButtonInput<MouseButton>`, consults `InputConfig` for rebindable keys, guards with `EguiWantsInput`, and emits `InputIntent` messages:

| Intent | Default Triggers |
|--------|-----------------|
| `CancelOrBack` | Escape, right-click |
| `ReturnToCenter` | Space |
| `TogglePause` | P |
| `CycleNext` | Tab |
| `CyclePrev` | Shift+Tab |
| `ConfirmFocused` | Enter, dwell-click |

### Tier 2: Game Systems

Systems consume `MessageReader<InputIntent>` instead of raw `ButtonInput`:
- `handle_back_navigation` ← `CancelOrBack`
- `handle_return_to_center` ← `ReturnToCenter`
- `toggle_pause` ← `TogglePause` + `CancelOrBack` (at room root)
- `handle_cycle_intent` ← `CycleNext` / `CyclePrev` (Tab through clickable objects)
- `handle_confirm_intent` ← `ConfirmFocused` (keyboard/dwell confirm on focused object)

The `on_click` observer remains pointer-driven (Pointer<Click>), guarded by `EguiWantsInput.wants_any_pointer_input()`.

### Rebindable Keys (input_config.rs)

`InputConfig` resource loaded from `assets/settings/keybindings.ron`. String-based key names parsed to `KeyCode` at load time. Hot-reloadable on native via `file_watcher`.

### Input Routing: Playing vs Overlays

Tab and Enter are routed differently based on `GameState`:

| State | Tab/Enter routed to | Why |
|-------|-------------------|-----|
| **Playing** | 3D scene (`InputIntent` system) | Tab cycles `Clickable` entities, Enter confirms focused |
| **Title, Paused, Won, GameOver** | egui buttons (native focus) | Tab cycles overlay buttons, Enter activates focused button |

During overlay states, `drive_overlay_focus` (in `EguiPrimaryContextPass`, before `render_ui`) auto-focuses the first button via `move_focus(Next)`. Overlay windows use `.resizable(false).interactable(false)` to prevent resize handles and window body from stealing keyboard focus.

### Accessibility Features

- **Tab cycling (3D)**: `FocusedClickable` resource tracks focused entity. Tab/Shift-Tab cycles through visible `Clickable` entities with emissive highlight.
- **Tab cycling (UI)**: Overlay buttons auto-focus on appearance. Tab/Shift-Tab cycles between buttons natively via egui's focus system.
- **Click-to-select combine**: Click inventory item to select (gold border), click second to auto-combine. Parallel path to drag-drop.
- **Dwell-click**: `DwellClickSettings` resource (disabled by default). Hovering over a clickable for N seconds fires `ConfirmFocused`.
