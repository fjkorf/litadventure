use std::collections::HashMap;

use bevy::{
    camera::{RenderTarget, visibility::RenderLayers},
    prelude::*,
    render::render_resource::{
        Extent3d, TextureDescriptor, TextureDimension, TextureFormat, TextureUsages,
    },
};
use bevy_egui::{EguiTextureHandle, EguiUserTextures};

use crate::components::InventoryItem;
use crate::inventory::ItemPickedUp;

/// Requests a preview for an item that wasn't picked up from the scene
/// (e.g., a combination result).
#[derive(Message)]
pub struct PreviewRequested {
    pub item_id: String,
    pub name: String,
}

const PREVIEW_SIZE: u32 = 128;
const PREVIEW_LAYER: usize = 1;

/// Marker for preview mesh entities (rendered off-screen).
#[derive(Component)]
pub struct PreviewMesh;

/// Marker for preview cameras.
#[derive(Component)]
pub struct PreviewCamera;

/// Pre-registered mesh/material handles for inventory items.
/// Captured when InventoryItem entities first spawn (before pickup/modification).
#[derive(Resource, Default)]
pub struct ItemMeshRegistry {
    pub entries: HashMap<String, (Handle<Mesh>, Handle<StandardMaterial>)>,
}

/// Maps item_id to the Image handle used for its preview thumbnail.
#[derive(Resource, Default)]
pub struct ItemPreviews {
    pub previews: HashMap<String, Handle<Image>>,
    next_slot: usize,
}

/// Creates a render target image for item previews.
fn create_preview_image(images: &mut Assets<Image>) -> Handle<Image> {
    let size = Extent3d {
        width: PREVIEW_SIZE,
        height: PREVIEW_SIZE,
        ..default()
    };

    let mut image = Image {
        texture_descriptor: TextureDescriptor {
            label: None,
            size,
            dimension: TextureDimension::D2,
            format: TextureFormat::Bgra8UnormSrgb,
            mip_level_count: 1,
            sample_count: 1,
            usage: TextureUsages::TEXTURE_BINDING
                | TextureUsages::COPY_DST
                | TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        },
        ..default()
    };
    image.resize(size);
    images.add(image)
}

/// Capture mesh/material handles when InventoryItem entities first appear.
/// This runs before any pickup/hiding, so the mesh data is always available.
fn register_item_meshes(
    items: Query<(Entity, &InventoryItem), Added<InventoryItem>>,
    mesh_q: Query<(&Mesh3d, &MeshMaterial3d<StandardMaterial>)>,
    children_q: Query<&Children>,
    mut registry: ResMut<ItemMeshRegistry>,
) {
    for (entity, item) in items.iter() {
        if registry.entries.contains_key(&item.item_id) {
            continue;
        }

        // Try the entity itself first (procedural spawns have mesh directly)
        if let Ok((mesh, mat)) = mesh_q.get(entity) {
            registry
                .entries
                .insert(item.item_id.clone(), (mesh.0.clone(), mat.0.clone()));
            continue;
        }

        // Walk children (glTF scenes put meshes on child entities)
        if let Ok(children) = children_q.get(entity) {
            for child in children.iter() {
                if let Ok((mesh, mat)) = mesh_q.get(child) {
                    registry
                        .entries
                        .insert(item.item_id.clone(), (mesh.0.clone(), mat.0.clone()));
                    break;
                }
            }
        }
    }
}

/// When an item is picked up, create its render-to-texture preview using the pre-registered mesh.
/// Falls back to a generic sphere for items without a scene mesh (e.g., combination results).
fn spawn_preview_on_pickup(
    mut reader: MessageReader<ItemPickedUp>,
    mut preview_reader: MessageReader<PreviewRequested>,
    registry: Res<ItemMeshRegistry>,
    mut images: ResMut<Assets<Image>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut egui_textures: ResMut<EguiUserTextures>,
    mut previews: ResMut<ItemPreviews>,
    mut commands: Commands,
) {
    let preview_layer = RenderLayers::layer(PREVIEW_LAYER);

    // Collect item_ids from both message types
    let mut to_spawn: Vec<String> = Vec::new();
    for ev in reader.read() {
        to_spawn.push(ev.item_id.clone());
    }
    for ev in preview_reader.read() {
        to_spawn.push(ev.item_id.clone());
    }

    for item_id in to_spawn {
        if previews.previews.contains_key(&item_id) {
            continue;
        }

        // Use pre-registered mesh, or create a fallback sphere for combined items
        let (mesh_h, mat_h) = if let Some((m, mat)) = registry.entries.get(&item_id) {
            (m.clone(), mat.clone())
        } else {
            let m = meshes.add(Sphere::new(0.15).mesh().ico(3).unwrap());
            let mat = materials.add(StandardMaterial {
                base_color: Color::srgb(0.6, 0.7, 0.9),
                ..default()
            });
            (m, mat)
        };

        // Create render target
        let image_handle = create_preview_image(&mut images);
        egui_textures.add_image(EguiTextureHandle::Strong(image_handle.clone()));
        previews
            .previews
            .insert(item_id.clone(), image_handle.clone());

        // Position each preview at a different X offset
        let slot = previews.next_slot;
        previews.next_slot += 1;
        let x_offset = slot as f32 * 10.0;

        commands.spawn((
            Mesh3d(mesh_h.clone()),
            MeshMaterial3d(mat_h.clone()),
            Transform::from_translation(Vec3::new(x_offset, 0.0, 0.0)),
            preview_layer.clone(),
            PreviewMesh,
        ));

        commands.spawn((
            Camera3d::default(),
            Camera {
                order: -(2 + slot as isize),
                clear_color: ClearColorConfig::Custom(Color::srgba(0.05, 0.05, 0.08, 1.0)),
                ..default()
            },
            RenderTarget::Image(image_handle.into()),
            Transform::from_translation(Vec3::new(x_offset, 0.15, 0.5))
                .looking_at(Vec3::new(x_offset, 0.0, 0.0), Vec3::Y),
            preview_layer.clone(),
            PreviewCamera,
        ));
    }
}

/// Slowly rotate all preview meshes.
fn rotate_previews(time: Res<Time>, mut query: Query<&mut Transform, With<PreviewMesh>>) {
    for mut transform in query.iter_mut() {
        transform.rotate_y(0.8 * time.delta_secs());
    }
}

/// Spawn a directional light for the preview layer (illuminates all positions equally).
fn setup_preview_light(mut commands: Commands) {
    commands.spawn((
        DirectionalLight {
            illuminance: 8_000.0,
            ..default()
        },
        Transform::from_rotation(Quat::from_euler(EulerRot::XYZ, -0.7, 0.4, 0.0)),
        RenderLayers::layer(PREVIEW_LAYER),
    ));
}

pub struct ItemPreviewPlugin;

impl Plugin for ItemPreviewPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ItemMeshRegistry>()
            .init_resource::<ItemPreviews>()
            .add_message::<PreviewRequested>()
            .add_systems(Startup, setup_preview_light)
            .add_systems(
                Update,
                (register_item_meshes, spawn_preview_on_pickup, rotate_previews),
            );
    }
}
