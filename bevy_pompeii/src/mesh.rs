use std::sync::Weak;

use bevy_asset::Handle;
use bevy_ecs::prelude::*;
use bevy_reflect::TypeUuid;
use bevy_transform::TransformBundle;
use log::{trace, warn};

use pompeii::{alloc::VkBufferHandle, PompeiiRenderer};

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

#[derive(Clone, TypeUuid)]
#[uuid = "c4ff691e-eaee-4369-84da-429838ea6e71"]
pub struct MeshAsset {
    pub(crate) renderer: Weak<PompeiiRenderer>,
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

impl Drop for MeshAsset {
    fn drop(&mut self) {
        unsafe {
            trace!("Freeing mesh...");
            if let Some(renderer) = self.renderer.upgrade() {
                renderer.free_buffer(self.vertices_handle.clone());
                renderer.free_buffer(self.indices_handle.clone())
            } else {
                warn!("Trying to free Mesh but the renderer is nowhere to be found");
            }
        }
    }
}
