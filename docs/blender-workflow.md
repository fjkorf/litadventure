# Blender / Skein Workflow

bevy_skein connects Blender and Bevy. You define game components on Blender objects, export to glTF, and they appear automatically in Bevy at runtime.

## Setup

### 1. Install the Blender Extension

- Open Blender 5.0+
- Drag `https://bevyskein.dev/releases/index.json` onto the Blender window
- Accept the repository, enable auto-updates
- Go to **Edit > Preferences > Get Extensions**, search "skein", click **Install**

### 2. Fetch the Type Registry

The Blender addon needs to know your Rust component types.

1. Run the Bevy app: `cargo run`
2. In Blender: **Edit > Fetch a Remote Type Registry** (or F3 > search "Fetch")
3. The addon connects to `http://127.0.0.1:15702` (Bevy's BRP endpoint)
4. All registered component types are now available in Blender's autocomplete

The registry is cached in the `.blend` file as `skein-registry.json`. Re-fetch only when component definitions change.

### 3. Addon Preferences

In Blender's addon preferences for Bevy Skein:
- **Debug**: Verbose console output
- **Presets**: Fetch Default implementations
- **Host**: BRP host (default: `http://127.0.0.1`)
- **Port**: BRP port (default: `15702`)

## Adding Components to Objects

1. Select an object in Blender
2. Open **Properties panel > Object tab**, scroll to the **Skein Bevy Panel**
3. In the `type:` field, start typing the component name (autocompletes)
4. Click **Insert Component Data**
5. Edit field values in the panel that appears

Available components (full type paths):
- `litadventure::components::Clickable`
- `litadventure::components::CameraSpot`
- `litadventure::components::NavigatesTo`
- `litadventure::components::ParentSpot`
- `litadventure::components::InventoryItem`
- `litadventure::components::ObjectState`
- `litadventure::components::RequiresItem`
- `litadventure::components::ContainedInName`
- `litadventure::components::TweenConfig`
- `litadventure::navigation::Portal`

## Exporting

**File > Export > glTF 2.0** (`.glb` or `.gltf`)

Ensure the Skein export options are enabled in the export dialog (they should be by default). Skein hooks into Blender's glTF exporter and embeds component data.

Export to: `assets/scenes/your_scene.glb`

## Loading in Bevy

```rust
commands.spawn(SceneRoot(
    asset_server.load(GltfAssetLabel::Scene(0).from_asset("scenes/study.glb")),
));
```

Components are deserialized and attached automatically by `SkeinPlugin`.

## Gotchas

### Vec3 serialization format

When writing `BEVY_skein` extension data manually (e.g., via `tools/generate_study.py`), Bevy expects Vec3 as an **array** `[0.0, 1.0, 0.0]`, not a map `{"x": 0, "y": 1, "z": 0}`. The Blender addon handles this correctly; it only matters for hand-written glTF.

### Empty nodes work with the extension path

The `BEVY_skein` extension (modern path) processes all nodes during glTF loading, including empty nodes without meshes. Camera spots can be Blender Empties with no geometry — their components load correctly.

The legacy `extras` path does NOT process empty nodes (Bevy doesn't attach `GltfExtras` to them). Always use the extension path for Bevy 0.18+.

### Entity references

`Entity` IDs aren't stable across glTF loads. Use string-based references:
- `ContainedInName { container_name: "Drawer" }` instead of `ContainedIn { container: Entity }`
- The `resolve_contained_in` system resolves names to entities after scene spawn

### Component registration

All components must be registered with `app.register_type::<YourComponent>()` for Skein to deserialize them. Components also need:
- `#[derive(Component, Reflect, Default)]`
- `#[reflect(Component, Default)]`

### Programmatic scene generation

For development without Blender, use `tools/generate_study.py`:

```sh
python3 tools/generate_study.py
```

This generates `assets/scenes/study.gltf` + `study.bin` + `study.glb` with all components embedded as Skein extras. Verify with:

```sh
cargo run --example load_gltf_test -- --glb
```
