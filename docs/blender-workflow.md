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

When writing glTF extras manually (e.g., via `tools/generate_study.py`), Bevy expects Vec3 as an **array** `[0.0, 1.0, 0.0]`, not a map `{"x": 0, "y": 1, "z": 0}`. The Blender addon handles this correctly; it only matters for hand-written glTF.

### Empty nodes don't get GltfExtras

Bevy doesn't always attach `GltfExtras` to nodes without meshes. Camera spots (which are spatial markers, not visible objects) need a tiny invisible mesh so Bevy processes their extras. In `generate_study.py`, camera spots get a 0.01-unit box.

When authoring in Blender, use a tiny cube (scale 0.001) for camera spot empties.

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
