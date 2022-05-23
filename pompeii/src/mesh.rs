use ash::vk;

use crate::{alloc::VkBufferHandle, PompeiiRenderer};

pub trait MeshVertex {
    fn format() -> vk::Format;
    fn stride() -> vk::DeviceSize;
}

#[derive(Debug, Copy, Clone)]
#[repr(C)]
pub struct VertexPosNormUvF32 {
    pub pos: [f32; 3],
    pub norm: [f32; 3],
    pub uv: [f32; 2],
}

impl MeshVertex for VertexPosNormUvF32 {
    fn format() -> vk::Format {
        vk::Format::R32G32B32_SFLOAT
    }

    fn stride() -> vk::DeviceSize {
        std::mem::size_of::<VertexPosNormUvF32>() as _
    }
}

pub trait MeshIndex {
    fn index_type() -> vk::IndexType;
}

impl MeshIndex for u16 {
    fn index_type() -> vk::IndexType {
        vk::IndexType::UINT16
    }
}

#[derive(Debug, Clone)]
pub struct Mesh {
    pub(crate) vertex_buffer: VkBufferHandle,
    pub(crate) index_buffer: VkBufferHandle,
    pub(crate) sub_meshes: Box<[SubMesh]>,
}

#[derive(Debug, Clone)]
pub struct SubMesh {
    pub(crate) vert_start: usize,
    pub(crate) vert_count: usize,
    pub(crate) index_start: usize,
    pub(crate) index_count: usize,
}

impl PompeiiRenderer {
    pub fn create_mesh(
        &self,
        vertices: VkBufferHandle,
        indices: VkBufferHandle,
        sub_meshes: impl Iterator<Item = impl Into<SubMesh>>,
    ) -> Mesh {
        Mesh {
            vertex_buffer: vertices,
            index_buffer: indices,
            sub_meshes: sub_meshes.map(|s| s.into()).collect(),
        }
    }
}

impl Mesh {
    /// # Safety
    /// The mesh should not me used after this.
    pub unsafe fn destroy(&self, renderer: &PompeiiRenderer) {
        renderer.free_buffer(self.vertex_buffer.clone());
        renderer.free_buffer(self.index_buffer.clone());
    }
}

impl Into<SubMesh> for (usize, usize, usize, usize) {
    fn into(self) -> SubMesh {
        SubMesh {
            vert_start: self.0,
            vert_count: self.1,
            index_start: self.2,
            index_count: self.3,
        }
    }
}

// impl From<(usize, usize, usize, usize)> for SubMesh {
//     fn from(
//         (vert_start, vert_count, index_start, index_count): (usize, usize, usize, usize),
//     ) -> Self {
//         Self {
//             vert_start,
//             vert_count,
//             index_start,
//             index_count,
//         }
//     }
// }
