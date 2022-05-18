use ash::vk;

pub trait Vertex {}

#[derive(Debug, Copy, Clone)]
#[repr(C)]
pub struct VertexPosNormUvF32 {
    pub pos: [f32; 3],
    pub norm: [f32; 3],
    pub uv: [f32; 2],
}

impl Vertex for VertexPosNormUvF32 {}

pub struct Mesh {
    vertex_buffer: vk::Buffer,
    vertex_offset: usize,
    vertex_count: usize,
    index_buffer: vk::Buffer,
    index_offset: usize,
    index_count: usize,
}
