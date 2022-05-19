use crate::alloc::VkBufferHandle;

#[derive(Default)]
pub struct PompeiiStore {
    buffers_vertex: Vec<VkBufferHandle>,
    buffers_index: Vec<VkBufferHandle>,
}

impl PompeiiStore {
    pub fn register_vertex_buffer(&mut self, handle: VkBufferHandle) -> usize {
        self.buffers_vertex.push(handle);
        self.buffers_vertex.len() - 1
    }

    pub fn register_index_buffer(&mut self, handle: VkBufferHandle) -> usize {
        self.buffers_index.push(handle);
        self.buffers_index.len() - 1
    }
}

impl PompeiiStore {
    pub(crate) fn cleanup(&mut self, vma: &vk_mem::Allocator) {
        for buff in self.buffers_vertex.iter().chain(self.buffers_index.iter()) {
            unsafe {
                vma.destroy_buffer(buff.handle, buff.allocation);
            }
        }
    }
}
