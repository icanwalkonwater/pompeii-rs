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

impl Mesh {
    pub fn destroy_on_exit(&self, renderer: &PompeiiRenderer) {
        let vert = self.vertex_buffer.clone();
        let index = self.index_buffer.clone();
        renderer
            .alloc_deletion_queue
            .lock()
            .push(Box::new(move |_, vma| unsafe {
                vert.destroy(vma);
                index.destroy(vma);
                Ok(())
            }));
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

#[derive(Debug, Clone)]
pub struct SubMesh {
    pub(crate) vert_start: usize,
    pub(crate) vert_count: usize,
    pub(crate) index_start: usize,
    pub(crate) index_count: usize,
}

impl SubMesh {
    pub(crate) fn max_vertex_index(&self) -> u32 {
        (self.index_start + self.index_count - 1) as _
    }
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
