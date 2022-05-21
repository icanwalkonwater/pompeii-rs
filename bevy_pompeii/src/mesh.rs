use bevy_app::AppExit;
use bevy_asset::{Assets, Handle};
use bevy_ecs::prelude::*;
use bevy_reflect::TypeUuid;
use bevy_transform::TransformBundle;

use log::{debug, trace};
use pompeii::{alloc::VkBufferHandle, PompeiiRenderer};
use std::sync::Arc;

#[derive(Debug, Bundle)]
pub struct MeshBundle {
    pub mesh: MeshComponent,
    #[bundle]
    pub transform: TransformBundle,
}

impl From<Handle<MeshAsset>> for MeshBundle {
    fn from(handle: Handle<MeshAsset>) -> Self {
        Self {
            mesh: MeshComponent { handle },
            transform: TransformBundle::identity(),
        }
    }
}

#[derive(Debug, Component)]
pub struct MeshComponent {
    pub handle: Handle<MeshAsset>,
}

#[derive(Debug, Clone, TypeUuid)]
#[uuid = "c4ff691e-eaee-4369-84da-429838ea6e71"]
pub struct MeshAsset {
    pub(crate) vertices_handle: VkBufferHandle,
    pub(crate) indices_handle: VkBufferHandle,
    pub(crate) sub_meshes: Vec<SubMesh>,
}

#[derive(Debug, Clone, TypeUuid)]
#[uuid = "5df2278d-38bb-4c84-9492-22109d82c6b5"]
pub struct SubMesh {
    pub(crate) vert_start: usize,
    pub(crate) vert_count: usize,
    pub(crate) index_start: usize,
    pub(crate) index_count: usize,
}

pub(crate) fn free_mesh_on_exit(
    mut events: EventReader<AppExit>,
    renderer: ResMut<Arc<PompeiiRenderer>>,
    assets: ResMut<Assets<MeshAsset>>,
    q_mesh: Query<&MeshComponent>,
) {
    if events.is_empty() {
        return;
    }

    trace!("Freeing meshes before exit");
    for mesh in q_mesh.iter() {
        if let Some(mesh) = assets.get(&mesh.handle) {
            unsafe {
                renderer.free_buffer(mesh.vertices_handle.clone());
                renderer.free_buffer(mesh.indices_handle.clone());
            }
        }
    }
}
