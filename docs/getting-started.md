# Getting Started

## Prerequisites

- **Rust** 1.92+ (edition 2024)
- **Bevy 0.18.1** (pulled automatically via Cargo)
- **litui** — must be cloned adjacent to this project at `../egui-md-macro`
- **Blender 5.0+** with the bevy_skein extension (for scene authoring, optional for running)
- **macOS / Linux / Windows** with GPU support (Metal, Vulkan, or DX12)

## Build

```sh
cargo build
```

First build takes several minutes (Bevy compilation). Subsequent builds are fast (~2-5s).

## Run

```sh
cargo run
```

Opens a window with the 3D study scene. UI panels overlay the viewport:
- **Top**: Room name and description
- **Right**: Inventory
- **Bottom**: Feedback text, current objective, hints
- **Tab / Shift+Tab**: Focus next/previous clickable object
- **Enter**: Interact with focused object
- **P key**: Pause (opens help overlay)
- **F1 key**: Toggle debug overlay (see [debug-mode.md](debug-mode.md))
- **Escape / Right-click**: Navigate back to previous camera position
- All keybindings are rebindable via `assets/settings/keybindings.ron`

## Run Tests

```sh
# Unit tests (25 tests across all modules)
cargo test

# Screenshot regression test
cargo run --example screenshot_test -- --save   # generate reference image
cargo run --example screenshot_test             # compare against reference

# glTF loading verification
cargo run --example load_gltf_test -- --glb     # test .glb format
cargo run --example load_gltf_test -- --gltf    # test .gltf format
```

## Project Structure

```
litadventure/
├── src/                    # Rust engine + game code
│   ├── main.rs             # App setup, UI rendering, state sync
│   ├── lib.rs              # Module exports
│   ├── components.rs       # All game components (Clickable, CameraSpot, etc.)
│   ├── interaction.rs      # Click handling, hover, back-nav, PlayState
│   ├── camera.rs           # Camera tweening, CameraController, spot lookup
│   ├── inventory.rs        # Inventory resource, items, combination recipes
│   ├── objectives.rs       # Objective tracking, completion, unlock chains
│   ├── hints.rs            # Timed 3-tier hint system
│   ├── navigation.rs       # Room transitions, Portal handling, room load/unload
│   ├── scene.rs            # glTF scene loading, camera/lights setup, ContainedIn resolver
│   ├── states.rs           # GameState (Title/Loading/Playing/Paused/Won/GameOver)
│   ├── game_data.rs        # RON asset types, level manifest, loading lifecycle
│   ├── input_intent.rs     # InputIntent enum, Tab cycling, dwell-click
│   ├── input_config.rs     # Rebindable key bindings from RON
│   ├── save.rs             # Save/load game state to/from RON files
│   └── debug.rs            # F1 debug overlay with wireframe click zones
├── content/                # litui markdown UI pages
│   ├── _app.md             # Global styles
│   ├── title.md            # Title/start screen with Start + Continue
│   ├── room_info.md        # Top panel
│   ├── inventory.md        # Right panel with Combine button
│   ├── objectives.md       # Bottom panel with hints
│   ├── help.md             # Pause overlay with Save button
│   ├── victory.md          # Win screen with Restart button
│   └── ...
├── assets/
│   ├── data/               # RON game data (hot-reloadable)
│   │   ├── objectives.ron
│   │   ├── hints.ron
│   │   ├── recipes.ron
│   │   ├── rooms.ron
│   │   └── demo.level.ron  # Level manifest (room → scene mappings)
│   ├── scenes/             # glTF scene files (one per room)
│   │   ├── study.glb (+ study.gltf + study.bin)
│   │   └── hallway.glb (+ hallway.gltf + hallway.bin)
│   ├── saves/              # Save game files (generated at runtime)
│   └── settings/           # Configuration files
│       └── keybindings.ron  # Rebindable key bindings
├── tools/
│   └── generate_study.py   # Python script to generate glTF scenes
├── examples/
│   ├── screenshot_test.rs  # Visual regression test
│   └── load_gltf_test.rs   # glTF component verification
├── tests/screenshots/      # Screenshot reference images
├── docs/                   # Documentation (you are here)
└── Cargo.toml
```

## Hot-Reload

Game data files in `assets/data/*.ron` support hot-reload. While the app is running, edit any `.ron` file and changes take effect immediately without recompilation. This is enabled by the `file_watcher` feature on the `bevy` dependency.

## Dependencies

| Crate | Version | Purpose |
|-------|---------|---------|
| bevy | 0.18.1 | Game engine |
| litui | 0.33.3 (local) | Compile-time Markdown-to-egui UI |
| bevy_egui | 0.39 | egui integration for Bevy |
| bevy_tweening | 0.15 | Camera/object animation with easing |
| bevy_skein | 0.5 | Blender-to-Bevy component pipeline |
| bevy_common_assets | 0.15 | RON asset loading |
| serde | 1 | Serialization for RON data |
| ron | 0.9 | RON format for save/load |
| image | 0.25 | Screenshot comparison |
