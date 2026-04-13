#!/usr/bin/env python3
"""Generate study.gltf and study.glb for the litadventure demo scene.

Creates simple box/plane geometry with bevy_skein component data via BEVY_skein extension.
Run: python3 tools/generate_study.py
"""

import json
import struct
import math
import os

# -- Geometry helpers --

def box_mesh(sx, sy, sz):
    """Create a box mesh with positions, normals, and indices."""
    hx, hy, hz = sx / 2, sy / 2, sz / 2
    # 24 vertices (4 per face), 36 indices
    positions = [
        # +Z face
        [-hx, -hy, hz], [hx, -hy, hz], [hx, hy, hz], [-hx, hy, hz],
        # -Z face
        [hx, -hy, -hz], [-hx, -hy, -hz], [-hx, hy, -hz], [hx, hy, -hz],
        # +X face
        [hx, -hy, hz], [hx, -hy, -hz], [hx, hy, -hz], [hx, hy, hz],
        # -X face
        [-hx, -hy, -hz], [-hx, -hy, hz], [-hx, hy, hz], [-hx, hy, -hz],
        # +Y face
        [-hx, hy, hz], [hx, hy, hz], [hx, hy, -hz], [-hx, hy, -hz],
        # -Y face
        [-hx, -hy, -hz], [hx, -hy, -hz], [hx, -hy, hz], [-hx, -hy, hz],
    ]
    normals = [
        [0, 0, 1], [0, 0, 1], [0, 0, 1], [0, 0, 1],
        [0, 0, -1], [0, 0, -1], [0, 0, -1], [0, 0, -1],
        [1, 0, 0], [1, 0, 0], [1, 0, 0], [1, 0, 0],
        [-1, 0, 0], [-1, 0, 0], [-1, 0, 0], [-1, 0, 0],
        [0, 1, 0], [0, 1, 0], [0, 1, 0], [0, 1, 0],
        [0, -1, 0], [0, -1, 0], [0, -1, 0], [0, -1, 0],
    ]
    indices = []
    for face in range(6):
        base = face * 4
        indices.extend([base, base + 1, base + 2, base, base + 2, base + 3])
    return positions, normals, indices


def plane_mesh(sx, sz):
    """Create a horizontal plane."""
    hx, hz = sx / 2, sz / 2
    positions = [[-hx, 0, -hz], [hx, 0, -hz], [hx, 0, hz], [-hx, 0, hz]]
    normals = [[0, 1, 0]] * 4
    indices = [0, 2, 1, 0, 3, 2]
    return positions, normals, indices


def cylinder_mesh(radius, height, segments=12):
    """Create a cylinder with side walls and top/bottom caps."""
    positions = []
    normals = []
    indices = []

    # Side walls
    for i in range(segments):
        a = 2 * math.pi * i / segments
        nx, nz = math.cos(a), math.sin(a)
        x, z = radius * nx, radius * nz
        positions.append([x, -height / 2, z])
        positions.append([x, height / 2, z])
        normals.append([nx, 0, nz])
        normals.append([nx, 0, nz])
    for i in range(segments):
        j = (i + 1) % segments
        b = i * 2
        n = j * 2
        indices.extend([b, n + 1, n, b, b + 1, n + 1])

    # Top cap (y = +height/2, normal up)
    top_center = len(positions)
    positions.append([0, height / 2, 0])
    normals.append([0, 1, 0])
    for i in range(segments):
        a = 2 * math.pi * i / segments
        positions.append([radius * math.cos(a), height / 2, radius * math.sin(a)])
        normals.append([0, 1, 0])
    for i in range(segments):
        j = (i + 1) % segments
        indices.extend([top_center, top_center + 1 + i, top_center + 1 + j])

    # Bottom cap (y = -height/2, normal down)
    bot_center = len(positions)
    positions.append([0, -height / 2, 0])
    normals.append([0, -1, 0])
    for i in range(segments):
        a = 2 * math.pi * i / segments
        positions.append([radius * math.cos(a), -height / 2, radius * math.sin(a)])
        normals.append([0, -1, 0])
    for i in range(segments):
        j = (i + 1) % segments
        indices.extend([bot_center, bot_center + 1 + j, bot_center + 1 + i])

    return positions, normals, indices


# -- glTF builder --

class GltfBuilder:
    def __init__(self):
        self.nodes = []
        self.meshes = []
        self.materials = []
        self.accessors = []
        self.buffer_views = []
        self.buffer_data = bytearray()
        self.scene_nodes = []

    def add_material(self, r, g, b, name="Material"):
        idx = len(self.materials)
        self.materials.append({
            "name": name,
            "pbrMetallicRoughness": {
                "baseColorFactor": [r, g, b, 1.0],
                "metallicFactor": 0.1,
                "roughnessFactor": 0.8,
            },
        })
        return idx

    def _pack_data(self, positions, normals, indices):
        """Pack geometry into buffer, create buffer views and accessors."""
        # Indices
        idx_offset = len(self.buffer_data)
        for i in indices:
            self.buffer_data.extend(struct.pack("<H", i))
        idx_byte_len = len(self.buffer_data) - idx_offset
        # Pad to 4-byte alignment
        while len(self.buffer_data) % 4 != 0:
            self.buffer_data.append(0)

        idx_bv = len(self.buffer_views)
        self.buffer_views.append({
            "buffer": 0,
            "byteOffset": idx_offset,
            "byteLength": idx_byte_len,
            "target": 34963,  # ELEMENT_ARRAY_BUFFER
        })
        idx_acc = len(self.accessors)
        self.accessors.append({
            "bufferView": idx_bv,
            "componentType": 5123,  # UNSIGNED_SHORT
            "count": len(indices),
            "type": "SCALAR",
            "max": [max(indices)],
            "min": [min(indices)],
        })

        # Positions
        pos_offset = len(self.buffer_data)
        mins = [min(p[i] for p in positions) for i in range(3)]
        maxs = [max(p[i] for p in positions) for i in range(3)]
        for p in positions:
            self.buffer_data.extend(struct.pack("<fff", *p))
        pos_byte_len = len(self.buffer_data) - pos_offset

        pos_bv = len(self.buffer_views)
        self.buffer_views.append({
            "buffer": 0,
            "byteOffset": pos_offset,
            "byteLength": pos_byte_len,
            "target": 34962,  # ARRAY_BUFFER
        })
        pos_acc = len(self.accessors)
        self.accessors.append({
            "bufferView": pos_bv,
            "componentType": 5126,  # FLOAT
            "count": len(positions),
            "type": "VEC3",
            "max": maxs,
            "min": mins,
        })

        # Normals
        norm_offset = len(self.buffer_data)
        for n in normals:
            self.buffer_data.extend(struct.pack("<fff", *n))
        norm_byte_len = len(self.buffer_data) - norm_offset

        norm_bv = len(self.buffer_views)
        self.buffer_views.append({
            "buffer": 0,
            "byteOffset": norm_offset,
            "byteLength": norm_byte_len,
            "target": 34962,
        })
        norm_acc = len(self.accessors)
        self.accessors.append({
            "bufferView": norm_bv,
            "componentType": 5126,
            "count": len(normals),
            "type": "VEC3",
        })

        return idx_acc, pos_acc, norm_acc

    def add_mesh(self, positions, normals, indices, material_idx, name="Mesh"):
        idx_acc, pos_acc, norm_acc = self._pack_data(positions, normals, indices)
        mesh_idx = len(self.meshes)
        self.meshes.append({
            "name": name,
            "primitives": [{
                "attributes": {"POSITION": pos_acc, "NORMAL": norm_acc},
                "indices": idx_acc,
                "material": material_idx,
            }],
        })
        return mesh_idx

    def add_node(self, name, mesh_idx=None, translation=None, components=None):
        node = {"name": name}
        if mesh_idx is not None:
            node["mesh"] = mesh_idx
        if translation:
            node["translation"] = translation
        if components:
            node["extensions"] = {
                "BEVY_skein": {"components": components}
            }
        idx = len(self.nodes)
        self.nodes.append(node)
        self.scene_nodes.append(idx)
        return idx

    def to_gltf_json(self, bin_uri="study.bin"):
        return {
            "asset": {"version": "2.0", "generator": "litadventure generate_study.py"},
            "extensionsUsed": ["BEVY_skein"],
            "scene": 0,
            "scenes": [{"name": "Study", "nodes": self.scene_nodes}],
            "nodes": self.nodes,
            "meshes": self.meshes,
            "materials": self.materials,
            "accessors": self.accessors,
            "bufferViews": self.buffer_views,
            "buffers": [{"uri": bin_uri, "byteLength": len(self.buffer_data)}],
        }

    def write_gltf(self, path):
        bin_name = os.path.splitext(os.path.basename(path))[0] + ".bin"
        bin_path = os.path.join(os.path.dirname(path), bin_name)
        gltf = self.to_gltf_json(bin_uri=bin_name)
        with open(path, "w") as f:
            json.dump(gltf, f, indent=2)
        with open(bin_path, "wb") as f:
            f.write(self.buffer_data)
        print(f"Wrote {path} ({len(self.nodes)} nodes) + {bin_path} ({len(self.buffer_data)} bytes)")

    def write_glb(self, path):
        gltf = self.to_gltf_json(bin_uri=None)
        # GLB: no URI on buffer
        gltf["buffers"] = [{"byteLength": len(self.buffer_data)}]
        json_bytes = json.dumps(gltf).encode("utf-8")
        # Pad JSON to 4-byte alignment
        while len(json_bytes) % 4 != 0:
            json_bytes += b" "
        # Pad bin to 4-byte alignment
        bin_data = bytes(self.buffer_data)
        while len(bin_data) % 4 != 0:
            bin_data += b"\x00"

        total = 12 + 8 + len(json_bytes) + 8 + len(bin_data)
        with open(path, "wb") as f:
            # Header
            f.write(struct.pack("<III", 0x46546C67, 2, total))  # magic, version, length
            # JSON chunk
            f.write(struct.pack("<II", len(json_bytes), 0x4E4F534A))  # length, type=JSON
            f.write(json_bytes)
            # BIN chunk
            f.write(struct.pack("<II", len(bin_data), 0x004E4942))  # length, type=BIN
            f.write(bin_data)
        print(f"Wrote {path} ({total} bytes)")


# -- Skein component helpers --

def skein(*components):
    """Build BEVY_skein component list from component tuples."""
    return [{k: v} for k, v in components]

def clickable(label, description):
    return ("litadventure::components::Clickable", {"label": label, "description": description})

def camera_spot(name, look_at):
    return ("litadventure::components::CameraSpot", {"name": name, "look_at": [float(look_at[0]), float(look_at[1]), float(look_at[2])]})

def navigates_to(spot_name):
    return ("litadventure::components::NavigatesTo", {"spot_name": spot_name})

def parent_spot(spot_name):
    return ("litadventure::components::ParentSpot", {"spot_name": spot_name})

def inventory_item(name, description, item_id):
    return ("litadventure::components::InventoryItem", {"name": name, "description": description, "item_id": item_id})

def object_state(state):
    return ("litadventure::components::ObjectState", state)

def contained_in_name(container_name):
    return ("litadventure::components::ContainedInName", {"container_name": container_name})

def requires_item(item_id, use_msg, fail_msg):
    return ("litadventure::components::RequiresItem", {"item_id": item_id, "use_message": use_msg, "fail_message": fail_msg})

def portal(target_room, entry_spot):
    return ("litadventure::navigation::Portal", {"target_room": target_room, "entry_spot": entry_spot})

def tween_config(open_offset, duration_ms):
    return ("litadventure::components::TweenConfig", {
        "open_offset": [float(open_offset[0]), float(open_offset[1]), float(open_offset[2])],
        "duration_ms": int(duration_ms),
    })


# -- Build the scenes --

def build_study():
    """Build the study room scene."""
    g = GltfBuilder()

    floor_mat = g.add_material(0.3, 0.25, 0.2, "FloorMat")
    wall_mat = g.add_material(0.6, 0.55, 0.5, "WallMat")
    desk_mat = g.add_material(0.45, 0.3, 0.15, "DeskMat")
    drawer_mat = g.add_material(0.5, 0.35, 0.18, "DrawerMat")
    flashlight_mat = g.add_material(0.7, 0.7, 0.2, "FlashlightMat")
    bookshelf_mat = g.add_material(0.35, 0.22, 0.1, "BookshelfMat")
    door_mat = g.add_material(0.4, 0.28, 0.12, "DoorMat")

    pos, nrm, idx = plane_mesh(10.0, 10.0)
    mesh = g.add_mesh(pos, nrm, idx, floor_mat, "FloorMesh")
    g.add_node("Floor", mesh)

    pos, nrm, idx = box_mesh(10.0, 5.0, 0.1)
    mesh = g.add_mesh(pos, nrm, idx, wall_mat, "BackWallMesh")
    g.add_node("BackWall", mesh, translation=[0, 2.5, -5])

    pos, nrm, idx = box_mesh(2.0, 0.8, 1.0)
    mesh = g.add_mesh(pos, nrm, idx, desk_mat, "DeskMesh")
    g.add_node("Desk", mesh, translation=[0, 0.8, -2],
               components=skein(
                   clickable("Desk", "A sturdy wooden desk. Its surface is worn smooth."),
                   navigates_to("desk_closeup"),
               ))

    pos, nrm, idx = box_mesh(0.6, 0.2, 0.4)
    mesh = g.add_mesh(pos, nrm, idx, drawer_mat, "DrawerMesh")
    g.add_node("Drawer", mesh, translation=[0, 0.55, -1.3],
               components=skein(
                   clickable("Drawer", "A small desk drawer with a brass handle."),
                   object_state("Closed"),
                   tween_config([0, 0, 0.4], 400),
               ))

    pos, nrm, idx = cylinder_mesh(0.05, 0.3)
    mesh = g.add_mesh(pos, nrm, idx, flashlight_mat, "FlashlightMesh")
    g.add_node("Flashlight", mesh, translation=[0, 0.7, -1.4],
               components=skein(
                   clickable("Flashlight", "A small flashlight. It still works."),
                   inventory_item("Flashlight", "A small flashlight. It still works.", "flashlight"),
                   contained_in_name("Drawer"),
               ))

    pos, nrm, idx = box_mesh(1.5, 3.0, 0.4)
    mesh = g.add_mesh(pos, nrm, idx, bookshelf_mat, "BookshelfMesh")
    g.add_node("Bookshelf", mesh, translation=[-3, 1.5, -4.5],
               components=skein(
                   clickable("Bookshelf", "Rows of old books. Most are too faded to read."),
               ))

    g.add_node("CameraSpot_RoomOverview", translation=[0, 3, 6],
               components=skein(camera_spot("room_overview", [0, 1, 0])))

    g.add_node("CameraSpot_DeskCloseup", translation=[0, 1.8, 0.5],
               components=skein(
                   camera_spot("desk_closeup", [0, 0.8, -2]),
                   parent_spot("room_overview"),
               ))

    g.add_node("CameraSpot_DrawerDetail", translation=[0, 1.2, -0.5],
               components=skein(
                   camera_spot("drawer_detail", [0, 0.55, -1.8]),
                   parent_spot("desk_closeup"),
               ))

    pos, nrm, idx = box_mesh(0.8, 2.0, 0.1)
    mesh = g.add_mesh(pos, nrm, idx, door_mat, "DoorMesh")
    g.add_node("Door", mesh, translation=[3, 1, -4.9],
               components=skein(
                   clickable("Door", "A wooden door leading to the hallway."),
                   portal("hallway", "hallway_overview"),
               ))

    return g


def build_hallway():
    """Build the hallway room scene."""
    g = GltfBuilder()

    floor_mat = g.add_material(0.25, 0.2, 0.18, "HallwayFloorMat")
    wall_mat = g.add_material(0.5, 0.48, 0.44, "HallwayWallMat")
    painting_mat = g.add_material(0.2, 0.3, 0.5, "PaintingMat")
    lens_mat = g.add_material(0.6, 0.8, 0.9, "LensMat")
    frame_mat = g.add_material(0.5, 0.4, 0.2, "FrameMat")
    locked_door_mat = g.add_material(0.3, 0.2, 0.1, "LockedDoorMat")
    door_mat = g.add_material(0.4, 0.28, 0.12, "DoorMat")

    pos, nrm, idx = plane_mesh(6.0, 16.0)
    mesh = g.add_mesh(pos, nrm, idx, floor_mat, "HallwayFloorMesh")
    g.add_node("HallwayFloor", mesh, translation=[0, 0, -14])

    pos, nrm, idx = box_mesh(6.0, 4.0, 0.1)
    mesh = g.add_mesh(pos, nrm, idx, wall_mat, "HallwayEndWallMesh")
    g.add_node("HallwayEndWall", mesh, translation=[0, 2, -22])

    pos, nrm, idx = box_mesh(0.8, 0.6, 0.05)
    mesh = g.add_mesh(pos, nrm, idx, painting_mat, "PaintingMesh")
    g.add_node("Painting", mesh, translation=[-2.5, 1.8, -14],
               components=skein(
                   clickable("Painting", "A faded landscape painting. Something is written on the back: '42'."),
               ))

    pos, nrm, idx = cylinder_mesh(0.08, 0.02)
    mesh = g.add_mesh(pos, nrm, idx, lens_mat, "LensMesh")
    g.add_node("Lens", mesh, translation=[-2, 0.8, -13.5],
               components=skein(
                   clickable("Lens", "A small glass lens, slightly dusty."),
                   inventory_item("Lens", "A small glass lens.", "lens"),
               ))

    pos, nrm, idx = box_mesh(0.12, 0.02, 0.06)
    mesh = g.add_mesh(pos, nrm, idx, frame_mat, "FrameMesh")
    g.add_node("Frame", mesh, translation=[1, 0.05, -12],
               components=skein(
                   clickable("Frame", "A small brass frame. Looks like it once held a lens."),
                   inventory_item("Frame", "A small brass frame.", "frame"),
               ))

    pos, nrm, idx = box_mesh(0.8, 2.0, 0.1)
    mesh = g.add_mesh(pos, nrm, idx, locked_door_mat, "LockedDoorMesh")
    g.add_node("LockedDoor", mesh, translation=[2.5, 1, -18],
               components=skein(
                   clickable("Locked Door", "A heavy door. It's too dark to see the lock clearly."),
                   object_state("Locked"),
                   requires_item("flashlight",
                                 "You shine the flashlight on the lock. It clicks open.",
                                 "It's too dark to see the lock clearly."),
               ))

    g.add_node("CameraSpot_HallwayOverview", translation=[0, 2.5, -8],
               components=skein(camera_spot("hallway_overview", [0, 1, -14])))

    pos, nrm, idx = box_mesh(0.8, 2.0, 0.1)
    mesh = g.add_mesh(pos, nrm, idx, door_mat, "DoorToStudyMesh")
    g.add_node("DoorToStudy", mesh, translation=[-2.5, 1, -8.5],
               components=skein(
                   clickable("Door", "The door back to the study."),
                   portal("study", "room_overview"),
               ))

    return g


if __name__ == "__main__":
    out_dir = os.path.join(os.path.dirname(__file__), "..", "assets", "scenes")
    os.makedirs(out_dir, exist_ok=True)

    study = build_study()
    study.write_gltf(os.path.join(out_dir, "study.gltf"))
    study.write_glb(os.path.join(out_dir, "study.glb"))

    hallway = build_hallway()
    hallway.write_gltf(os.path.join(out_dir, "hallway.gltf"))
    hallway.write_glb(os.path.join(out_dir, "hallway.glb"))
