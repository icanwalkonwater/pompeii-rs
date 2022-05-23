use std::{ffi::CString, slice::from_ref, sync::Arc};

use ash::vk;

use crate::{errors::Result, mesh::VertexPosNormUvF32, PompeiiRenderer};

#[derive(Debug, Copy, Clone)]
pub struct BufferHandle(usize);

pub struct PompeiiTransferContext<'a> {
    renderer: &'a PompeiiRenderer,
    vma: Arc<vk_mem::Allocator>,
    ops_buffer_copy: Vec<(vk::Buffer, vk::Buffer, vk::BufferCopy)>,
    // ops_image_copy: Vec<vk::ImageCopy>,
    to_destroy: Vec<VkBufferHandle>,
}

impl PompeiiRenderer {
    pub fn start_transfer_operations(&self) -> PompeiiTransferContext {
        let vma = Arc::clone(&self.vma);
        PompeiiTransferContext {
            renderer: self,
            vma,
            ops_buffer_copy: Vec::new(),
            // ops_image_copy: Default::default(),
            to_destroy: Vec::new(),
        }
    }

    pub unsafe fn free_buffer(&self, buffer: VkBufferHandle) {
        self.vma.destroy_buffer(buffer.handle, buffer.allocation);
    }
}

impl<'a> PompeiiTransferContext<'a> {
    pub fn create_vertex_buffer(
        &mut self,
        vertices: &[VertexPosNormUvF32],
    ) -> Result<VkBufferHandle> {
        let size = (vertices.len() * std::mem::size_of::<VertexPosNormUvF32>()) as _;
        let staging = self.renderer.alloc_staging_buffer(size)?;
        let vertex_buffer = self.renderer.alloc_vertex_buffer(size)?;

        self.renderer.debug_utils.name_buffer(
            &self.renderer.device,
            vertex_buffer.handle,
            &CString::new(format!("Vertex Buffer (size: {})", size)).unwrap(),
        )?;

        unsafe { self.renderer.store_to_buffer(&staging, vertices)? };

        self.ops_buffer_copy.push((
            staging.handle,
            vertex_buffer.handle,
            vk::BufferCopy::builder()
                .size(size)
                .src_offset(0)
                .dst_offset(0)
                .build(),
        ));

        self.to_destroy.push(staging);

        Ok(vertex_buffer)
    }

    pub fn create_index_buffer(&mut self, indices: &[u16]) -> Result<VkBufferHandle> {
        let size = (indices.len() * std::mem::size_of::<u16>()) as _;
        let staging = self.renderer.alloc_staging_buffer(size)?;
        let index_buffer = self.renderer.alloc_index_buffer(size)?;

        self.renderer.debug_utils.name_buffer(
            &self.renderer.device,
            index_buffer.handle,
            &CString::new(format!("Index Buffer (size: {})", size)).unwrap(),
        )?;

        unsafe { self.renderer.store_to_buffer(&staging, indices)? };

        self.ops_buffer_copy.push((
            staging.handle,
            index_buffer.handle,
            vk::BufferCopy::builder()
                .size(size)
                .src_offset(0)
                .dst_offset(0)
                .build(),
        ));

        self.to_destroy.push(staging);

        Ok(index_buffer)
    }

    pub fn submit_and_wait(self) -> Result<()> {
        let device = &self.renderer.device;
        let queue = self.renderer.queues.transfer();

        unsafe {
            let cmd = self
                .renderer
                .record_one_time_command_buffer(queue.pool, |cmd| {
                    for (from, to, op) in &self.ops_buffer_copy {
                        device.cmd_copy_buffer(cmd, *from, *to, from_ref(op));
                    }
                    Ok(())
                })?;

            let fence = device.create_fence(&vk::FenceCreateInfo::builder(), None)?;
            let cmds = [cmd];
            let info = vk::SubmitInfo::builder().command_buffers(&cmds);
            device.queue_submit(queue.queue, from_ref(&info), fence)?;

            // Drop queue
            drop(queue);

            // Wait
            device.wait_for_fences(from_ref(&fence), true, u64::MAX)?;
            device.destroy_fence(fence, None);

            // Destroy staging
            for buff in self.to_destroy {
                self.vma.destroy_buffer(buff.handle, buff.allocation);
            }
        }

        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct VkBufferHandle {
    pub(crate) handle: vk::Buffer,
    pub(crate) allocation: vk_mem::Allocation,
    pub(crate) info: vk_mem::AllocationInfo,
}

unsafe impl Send for VkBufferHandle {}
unsafe impl Sync for VkBufferHandle {}

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
impl PompeiiRenderer {
    pub(crate) fn alloc_staging_buffer(&self, size: vk::DeviceSize) -> Result<VkBufferHandle> {
        unsafe {
            let handle = self.create_buffer(
                size,
                vk::BufferUsageFlags::TRANSFER_SRC,
                vk_mem::MemoryUsage::CpuOnly,
            )?;

            self.debug_utils.name_buffer(
                &self.device,
                handle.handle,
                &CString::new(format!("Staging Buffer (size: {})", size)).unwrap(),
            )?;

            Ok(handle)
        }
    }

    pub(crate) fn alloc_vertex_buffer(&self, size: vk::DeviceSize) -> Result<VkBufferHandle> {
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

    pub(crate) fn alloc_index_buffer(&self, size: vk::DeviceSize) -> Result<VkBufferHandle> {
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

    pub(crate) fn alloc_acceleration_structure_scratch_buffer(
        &self,
        size: vk::DeviceSize,
    ) -> Result<VkBufferHandle> {
        unsafe {
            self.create_buffer(
                size,
                vk::BufferUsageFlags::STORAGE_BUFFER | vk::BufferUsageFlags::SHADER_DEVICE_ADDRESS,
                vk_mem::MemoryUsage::GpuOnly,
            )
        }
    }

    pub(crate) fn alloc_acceleration_structure(&self, size: vk::DeviceSize) -> Result<VkBufferHandle> {
        unsafe {
            self.create_buffer(
                size,
                vk::BufferUsageFlags::ACCELERATION_STRUCTURE_STORAGE_KHR | vk::BufferUsageFlags::SHADER_DEVICE_ADDRESS,
                vk_mem::MemoryUsage::GpuOnly,
            )
        }
    }
}

// Low level methods
impl PompeiiRenderer {
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
