use bevy_ecs::prelude::*;
use bevy_transform::TransformBundle;

use pompeii::alloc::BufferHandle;

#[derive(Bundle)]
pub struct MeshBundle {
    pub mesh: Mesh,
    #[bundle]
    pub transform: TransformBundle,
}

impl From<TransformBundle> for MeshBundle {
    fn from(transform: TransformBundle) -> Self {
        Self {
            mesh: Mesh,
            transform,
        }
    }
}

#[derive(Debug, Component)]
pub struct Mesh;

#[derive(Debug, Component)]
pub struct SubMesh {
    pub(crate) vert_handle: BufferHandle,
    pub(crate) vert_start: usize,
    pub(crate) vert_count: usize,
    pub(crate) index_handle: BufferHandle,
    pub(crate) index_start: usize,
    pub(crate) index_count: usize,
}
