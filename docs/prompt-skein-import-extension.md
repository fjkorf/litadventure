# Prompt: Develop a bevy_skein glTF Import Extension for Blender

## Status: Implemented

Implemented in `gltf_import_extension.py` in the bevy_skein Blender addon
(`~/Library/Application Support/Blender/5.1/extensions/bevyskein_dev/bevy_skein/`).

Verified with `assets/scenes/study.glb`: 27 components across 14 objects import
correctly, including Vec3 arrays (CameraSpot.look_at), bare enum strings
(ObjectState), and nested structs. All requirement tiers (Must/Should/Nice)
are covered — see the checklist below.

## Goal

Write a `glTF2ImportUserExtension` for the bevy_skein Blender addon that reads `BEVY_skein` extension data (and optionally legacy `skein` extras) from imported glTF/glb files and populates the addon's internal `skein_two` CollectionProperty on each Blender object. This enables round-trip editing: export a scene with components from Bevy/Python, import into Blender, see and edit the components in the Skein panel, then re-export.

## Background

### What bevy_skein is

bevy_skein is a two-part toolchain connecting Blender and the Bevy game engine (Rust):
- **Blender addon** (Python): Lets users add Bevy component data to Blender objects via a UI panel. On glTF export, the addon injects this data as a `BEVY_skein` glTF extension on nodes.
- **Rust crate** (`bevy_skein`): Reads the extension data when Bevy loads the glTF and instantiates real ECS components on entities.

Currently the addon is **export-only**. When importing a glTF that contains `BEVY_skein` extension data, Blender discards the unknown extension and the Skein panel shows no components.

### The data formats

**Modern format** (BEVY_skein extension, preferred for Bevy 0.18+):
```json
{
  "name": "Desk",
  "mesh": 2,
  "extensions": {
    "BEVY_skein": {
      "components": [
        {
          "my_game::components::Clickable": {
            "label": "Desk",
            "description": "A sturdy wooden desk."
          }
        },
        {
          "my_game::components::NavigatesTo": {
            "spot_name": "desk_closeup"
          }
        }
      ]
    }
  }
}
```

**Legacy format** (extras, pre-0.18):
```json
{
  "name": "Desk",
  "extras": {
    "skein": [
      { "my_game::components::Clickable": { "label": "Desk", "description": "..." } }
    ]
  }
}
```

Both formats use the same component array structure: `[ { "type_path": { field: value, ... } }, ... ]`.

### How the export side works

The export hook (`gltf_export_extension.py`) implements `glTF2ExportUserExtension`:
1. `gather_node_hook` is called for each node during export
2. It reads from `blender_object.skein_two` — a `CollectionProperty` on the Blender object
3. For each component in `skein_two`, it calls `get_data_from_active_editor()` (in `form_to_object.py`) to serialize the PropertyGroup fields to JSON
4. It places the result in `gltf2_object.extensions["BEVY_skein"]` and optionally in `gltf2_object.extras["skein"]`

### How components are stored on Blender objects

Each Blender object (Object, Mesh, Material, Scene, Light, Bone) has a `skein_two` CollectionProperty. Each item in `skein_two` is a `ComponentData` PropertyGroup with:
- `name`: Short display name (e.g., "Clickable")
- `selected_type_path`: Full Rust type path (e.g., "my_game::components::Clickable")
- A dynamically generated PointerProperty keyed by `hash_over_64(selected_type_path)` that holds the component's fields as a Blender PropertyGroup

The PropertyGroups are dynamically created at registry load time by `make_property()` in `property_groups.py`. They are stored in `bpy.context.window_manager.skein_property_groups` keyed by type path.

### How insert_component_data works (the existing write path)

In `op_insert_component.py`, `insert_component_data()`:
1. Gets the selected type path from `context.window_manager.selected_component`
2. Looks up the registry data for that type
3. Calls `obj.skein_two.add()` to create a new collection item
4. Sets `new_component.name` and `new_component.selected_type_path`
5. Calls `touch_all_fields()` to initialize PointerProperty fields
6. Optionally calls `object_to_form()` to fill in preset/default values

### How object_to_form works (the existing data population path)

In `object_to_form.py`, `object_to_form(context, context_key, data)`:
- Takes a component PropertyGroup, a context key (hashed type path), and JSON data
- Handles enums via `skein_enum_index`
- Handles `Option<T>` via `is_core_option`
- Handles glam types (Vec2, Vec3, Vec4, Quat, Mat2-4, Affine) via `type_override` — these use array serialization, e.g., Vec3 is `[x, y, z]`
- For regular structs, iterates annotations and sets fields directly

**This function is the key reusable piece** — it already knows how to take JSON data and populate a Skein PropertyGroup.

### The type registry

The type registry is stored as a JSON text block in the .blend file named `skein-registry.json`. It is loaded by `ReloadSkeinRegistryJson` (in `op_registry_loading.py`) and processed by `process_registry()` which:
1. Parses the JSON
2. Creates dynamic PropertyGroup classes for each component type
3. Registers them as PointerProperties on `ComponentData`
4. Stores references in `bpy.context.window_manager.skein_property_groups`

**The registry MUST be loaded before import can work.** Without it, the addon doesn't know the PropertyGroup structure for each component type.

### Blender's glTF import extension system

Blender's glTF importer (io_scene_gltf2) supports import extension hooks via a `glTF2ImportUserExtension` class. The KhronosGroup provides a reference implementation:
https://github.com/KhronosGroup/glTF-Blender-IO/tree/main/example-addons/example_gltf_importer_extension

Key hooks:
- `gather_import_node_before_hook(self, vnode, gltf_node, gltf)` — called before node import
- `gather_import_node_after_hook(self, vnode, gltf_node, blender_object, gltf)` — called after, with the Blender object available
- `gather_import_scene_before_hook(self, gltf_scene, blender_scene, gltf)` / `after`
- `gather_import_mesh_after_hook(self, gltf_mesh, blender_mesh, gltf)`
- `gather_import_material_after_hook(self, gltf_material, vert_color, blender_mat, gltf)`
- `gather_import_light_after_hook(self, gltf_node, blender_node, blender_light, gltf)`
- `gather_import_joint_after_hook(self, gltf_node, bone, gltf)`

The `gltf_node` object has an `extensions` dict where `gltf_node.extensions.get("BEVY_skein")` returns the extension data if present. It also has an `extras` dict.

## Requirements

### Must Have

1. [x] **Read BEVY_skein extension data on import**: Implement `gather_import_node_after_hook` that:
   - Checks `gltf_node.extensions` for `"BEVY_skein"` key
   - Falls back to `gltf_node.extras` for `"skein"` key (legacy support)
   - For each component in the array, populates `blender_object.skein_two` using the same pattern as `insert_component_data`
   - Uses `object_to_form()` to fill in the field values from the JSON data

2. [x] **Require loaded registry**: If the type registry isn't loaded, log a warning and skip. The user must fetch/reload the registry before importing.

3. [x] **Handle all Blender object types**: Implement hooks for nodes, meshes, materials, scenes, lights, and joints (matching the export hooks).

4. [x] **Graceful handling of unknown components**: If a component type path in the glTF data doesn't match any registered type, log a warning and skip that component (don't fail the entire import).

### Should Have

5. [x] **Import settings UI**: Add a checkbox in the glTF import dialog (matching the export dialog pattern) to enable/disable Skein import. Registered via `importer_extension_layout_draw` in `__init__.py`.

6. [x] **Conflict handling**: If an object already has a component of the same type in `skein_two`, the duplicate is skipped (tracked via `existing_types` set in `_apply_components`).

### Nice to Have

7. [x] **Auto-registry-load on import**: Implemented in `_ensure_registry()` with a two-tier fallback: first tries `bpy.ops.wm.fetch_type_registry()` to fetch from a running Bevy app, then falls back to the embedded `skein-registry.json` text block. The fetch runs once on the first node with components; subsequent nodes find the registry already cached.

## Implementation Notes

### What was built

**New file: `gltf_import_extension.py`**

| Symbol | Role |
|--------|------|
| `SkeinImportExtensionProperties` | PropertyGroup with `enabled` checkbox for the import dialog |
| `draw_import()` | Draws the Skein panel in the glTF import dialog |
| `glTF2ImportUserExtension` | Import extension with hooks for all object types |
| `_extract_components()` | Reads `BEVY_skein` extension (modern) or `extras.skein` (legacy) |
| `_ensure_registry()` | Two-tier auto-load: fetch from running Bevy app, fall back to embedded text block |
| `_apply_components()` | Creates `skein_two` entries and populates via `object_to_form()` |

Key design decisions vs. the original sketch:
- **Imports `touch_all_fields` from `op_insert_component`** rather than duplicating it
- **Separate `SkeinImportExtensionProperties`** (not sharing the export PropertyGroup) so import/export enable states are independent
- **`_ensure_registry()` auto-fetches from a running Bevy app** — the original spec only mentioned loading from `skein-registry.json`, but in practice the user's workflow has a Bevy app running. The fetch fires once on the first node; the cached registry serves all subsequent nodes.
- **`force_default` types** (List, Map, Set) are detected via `pg.force_default` and skip `object_to_form()`, matching the export-side `gather_skein_two()` pattern.

**Changes to `__init__.py`**

- Import line: `from .gltf_import_extension import SkeinImportExtensionProperties, draw_import, glTF2ImportUserExtension`
- `register()`: registers `SkeinImportExtensionProperties`, creates `Scene.skein_import_extension_properties`, registers `draw_import` via `importer_extension_layout_draw`
- `unregister()`: mirrors the above

### How Blender discovers the extension

Blender's glTF importer (`io_scene_gltf2/__init__.py:2027-2034`) iterates `preferences.addons.keys()`, looks up each module in `sys.modules`, and checks for `glTF2ImportUserExtension`. Since `__init__.py` imports the class, it becomes an attribute of the addon module and is discovered automatically.

The hook dispatcher (`io_scene_gltf2/io/imp/user_extensions.py`) calls each hook inside a `try/except` that logs errors to `gltf.log.error` — not to stdout or the Blender Info editor. This means exceptions in hooks are silently swallowed. Warnings from the import extension use `print()` and are only visible in the system console (launch Blender from terminal).

### Verified test results

Tested with `assets/scenes/study.glb` (18 nodes, 14 with components, 27 total components):

- **External glTF test**: 14 objects populated correctly — Clickable (strings), CameraSpot (Vec3 via `type_override`), ObjectState (bare enum string), TweenConfig (Vec3 + int), Portal (strings), RequiresItem (strings), InventoryItem (strings), ContainedInName (string), NavigatesTo (string), ParentSpot (string)
- **Mixed content test**: 4 nodes without extensions (Floor, BackWall, HallwayFloor, HallwayEndWall) correctly get 0 `skein_two` entries
- **Error handling**: unknown types log a warning and are skipped; import continues for other components
- **Auto-registry-load**: `_ensure_registry()` fetches from running Bevy app on first node, cached for all subsequent nodes

### Key source files referenced

All paths relative to the Skein Blender addon root:

| File | Purpose | What was reused |
|------|---------|-----------------|
| `gltf_export_extension.py` | Export hook (pattern mirrored) | `SkeinExtensionProperties` pattern, `gather_skein_two()` flow |
| `op_insert_component.py` | Inserting components onto objects | `touch_all_fields()` imported directly |
| `object_to_form.py` | Populating PropertyGroups from JSON | `object_to_form()` called for each component |
| `property_groups.py` | `hash_over_64()`, `ComponentData` class | `hash_over_64()` for type path key mapping |
| `op_registry_loading.py` | Registry loading and processing | `process_registry()` used by `_ensure_registry()` fallback |
| `__init__.py` | Addon registration | Registration of import extension properties and UI |

### Repository

- bevy_skein source: https://github.com/rust-adventure/skein
- Khronos glTF import extension example: https://github.com/KhronosGroup/glTF-Blender-IO/tree/main/example-addons/example_gltf_importer_extension
- Blender glTF addon source: https://projects.blender.org/blender/blender/src/branch/main/scripts/addons_core/io_scene_gltf2
