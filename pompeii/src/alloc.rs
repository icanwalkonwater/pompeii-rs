use std::sync::Arc;

use ash::vk;

use crate::errors::Result;
use crate::mesh::VertexPosNormUvF32;
use crate::PompeiiRenderer;

pub struct BufferHandle(usize);

pub struct PompeiiTransferContext<'a> {
    renderer: &'a mut PompeiiRenderer,
    vma: Arc<vk_mem::Allocator>,
    ops_buffer_copy: Vec<(vk::Buffer, vk::Buffer, vk::BufferCopy)>,
    // ops_image_copy: Vec<vk::ImageCopy>,
}

impl PompeiiRenderer {
    pub fn start_transfer_operations(&mut self) -> PompeiiTransferContext {
        let vma = Arc::clone(&self.vma);
        PompeiiTransferContext {
            renderer: self,
            vma,
            ops_buffer_copy: Default::default(),
            // ops_image_copy: Default::default(),
        }
    }
}

impl<'a> PompeiiTransferContext<'a> {
    pub fn create_vertex_buffer(
        &mut self,
        vertices: &[VertexPosNormUvF32],
    ) -> Result<BufferHandle> {
        let size = (vertices.len() * std::mem::size_of::<VertexPosNormUvF32>()) as _;
        let staging = self.alloc_staging_buffer(size)?;
        let vertex_buffer = self.alloc_vertex_buffer(size)?;

        unsafe { self.store_to_buffer(&staging, vertices)? };

        self.ops_buffer_copy.push((
            staging.handle,
            vertex_buffer.handle,
            vk::BufferCopy::builder()
                .size(size)
                .src_offset(0)
                .dst_offset(0)
                .build(),
        ));

        Ok(BufferHandle(
            self.renderer.store.register_vertex_buffer(vertex_buffer),
        ))
    }

    pub fn create_index_buffer(&mut self, indices: &[u16]) -> Result<BufferHandle> {
        let size = (indices.len() * std::mem::size_of::<u16>()) as _;
        let staging = self.alloc_staging_buffer(size)?;
        let index_buffer = self.alloc_index_buffer(size)?;

        unsafe { self.store_to_buffer(&staging, indices)? };

        self.ops_buffer_copy.push((
            staging.handle,
            index_buffer.handle,
            vk::BufferCopy::builder()
                .size(size)
                .src_offset(0)
                .dst_offset(0)
                .build(),
        ));

        Ok(BufferHandle(
            self.renderer.store.register_index_buffer(index_buffer),
        ))
    }

    pub fn submit_and_wait(self) {
        let queue = self.renderer.queues.transfer();
        todo!()
    }
}

pub struct VkBufferHandle {
    handle: vk::Buffer,
    allocation: vk_mem::Allocation,
    info: vk_mem::AllocationInfo,
}

impl From<(vk::Buffer, vk_mem::Allocation, vk_mem::AllocationInfo)> for VkBufferHandle {
    fn from(
        (handle, allocation, info): (vk::Buffer, vk_mem::Allocation, vk_mem::AllocationInfo),
    ) -> Self {
        Self {
            handle,
            allocation,
            info,
        }
    }
}

// Utils methods
impl PompeiiTransferContext<'_> {
    fn alloc_staging_buffer(&self, size: vk::DeviceSize) -> Result<VkBufferHandle> {
        unsafe {
            self.create_buffer(
                size,
                vk::BufferUsageFlags::TRANSFER_SRC,
                vk_mem::MemoryUsage::CpuOnly,
            )
        }
    }

    fn alloc_vertex_buffer(&self, size: vk::DeviceSize) -> Result<VkBufferHandle> {
        unsafe {
            self.create_buffer(
                size,
                vk::BufferUsageFlags::TRANSFER_DST
                    | vk::BufferUsageFlags::VERTEX_BUFFER
                    | vk::BufferUsageFlags::SHADER_DEVICE_ADDRESS,
                vk_mem::MemoryUsage::GpuOnly,
            )
        }
    }

    fn alloc_index_buffer(&self, size: vk::DeviceSize) -> Result<VkBufferHandle> {
        unsafe {
            self.create_buffer(
                size,
                vk::BufferUsageFlags::TRANSFER_DST
                    | vk::BufferUsageFlags::INDEX_BUFFER
                    | vk::BufferUsageFlags::SHADER_DEVICE_ADDRESS,
                vk_mem::MemoryUsage::GpuOnly,
            )
        }
    }
}

// Low level methods
impl PompeiiTransferContext<'_> {
    unsafe fn create_buffer(
        &self,
        size: vk::DeviceSize,
        usage: vk::BufferUsageFlags,
        location: vk_mem::MemoryUsage,
    ) -> Result<VkBufferHandle> {
        Ok(self
            .vma
            .create_buffer(
                &vk::BufferCreateInfo::builder()
                    .size(size)
                    .usage(usage)
                    .sharing_mode(vk::SharingMode::EXCLUSIVE)
                    .build(),
                &vk_mem::AllocationCreateInfo::new().usage(location),
            )?
            .into())
    }

    unsafe fn store_to_buffer<D: Copy>(&self, buffer: &VkBufferHandle, data: &[D]) -> Result<()> {
        debug_assert_ne!(
            (buffer.info.get_memory_type() & vk::MemoryPropertyFlags::HOST_VISIBLE.as_raw()),
            0
        );

        let (need_unmap, mapped_ptr) = if buffer.info.get_mapped_data().is_null() {
            (true, self.vma.map_memory(buffer.allocation)?)
        } else {
            (false, buffer.info.get_mapped_data())
        };

        let size = (std::mem::size_of::<D>() * data.len()) as _;
        let mut mapped_data =
            ash::util::Align::new(mapped_ptr as _, std::mem::align_of::<D>() as _, size);

        mapped_data.copy_from_slice(data);

        self.vma.flush_allocation(buffer.allocation, 0, size as _)?;

        if need_unmap {
            self.vma.unmap_memory(buffer.allocation);
        }

        Ok(())
    }
}
