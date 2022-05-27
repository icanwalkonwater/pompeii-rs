use std::{ffi::CString, ptr, slice::from_ref, sync::Arc};

use ash::vk;
use log::trace;

use crate::{errors::Result, mesh::VertexPosNormUvF32, PompeiiRenderer};

#[derive(Debug)]
pub(crate) struct VmaPools {
    pub(crate) acceleration_structures: vk_mem::AllocatorPool,
}

unsafe impl Send for VmaPools {}
unsafe impl Sync for VmaPools {}

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

    pub unsafe fn free_buffer_on_exit(&self, buffer: VkBufferHandle) {
        self.alloc_deletion_queue
            .lock()
            .push(Box::new(move |_, vma| {
                let buffer = buffer;
                vma.destroy_buffer(buffer.handle, buffer.allocation);
                Ok(())
            }));
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

    pub fn create_acceleration_structure_instance_buffer(
        &mut self,
        instances: &[vk::AccelerationStructureInstanceKHR],
    ) -> Result<VkBufferHandle> {
        let size =
            (instances.len() * std::mem::size_of::<vk::AccelerationStructureInstanceKHR>()) as _;
        let staging = self.renderer.alloc_staging_buffer(size)?;
        let instances_buffer = self
            .renderer
            .alloc_acceleration_structure_instance_buffer(size)?;

        self.renderer.debug_utils.name_buffer(
            &self.renderer.device,
            instances_buffer.handle,
            &CString::new(format!("TLAS Instances buffer (size: {})", size)).unwrap(),
        )?;

        unsafe {
            self.renderer.store_to_buffer(&staging, instances)?;
        }

        self.ops_buffer_copy.push((
            staging.handle,
            instances_buffer.handle,
            vk::BufferCopy::builder()
                .size(size)
                .src_offset(0)
                .dst_offset(0)
                .build(),
        ));

        self.to_destroy.push(staging);

        Ok(instances_buffer)
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

impl VkBufferHandle {
    pub unsafe fn destroy(&self, vma: &vk_mem::Allocator) {
        vma.destroy_buffer(self.handle, self.allocation);
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
                    | vk::BufferUsageFlags::SHADER_DEVICE_ADDRESS
                    | vk::BufferUsageFlags::ACCELERATION_STRUCTURE_BUILD_INPUT_READ_ONLY_KHR,
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
                    | vk::BufferUsageFlags::SHADER_DEVICE_ADDRESS
                    | vk::BufferUsageFlags::ACCELERATION_STRUCTURE_BUILD_INPUT_READ_ONLY_KHR,
                vk_mem::MemoryUsage::GpuOnly,
            )
        }
    }

    pub(crate) fn alloc_acceleration_structure_scratch_buffer(
        &self,
        size: vk::DeviceSize,
    ) -> Result<VkBufferHandle> {
        unsafe {
            self.create_buffer_from_pool(
                size,
                vk::BufferUsageFlags::STORAGE_BUFFER | vk::BufferUsageFlags::SHADER_DEVICE_ADDRESS,
                vk_mem::MemoryUsage::GpuOnly,
                self.vma_pools.acceleration_structures,
            )
        }
    }

    pub(crate) fn alloc_acceleration_structure_buffer(
        &self,
        size: vk::DeviceSize,
    ) -> Result<VkBufferHandle> {
        unsafe {
            self.create_buffer(
                size,
                vk::BufferUsageFlags::ACCELERATION_STRUCTURE_STORAGE_KHR
                    | vk::BufferUsageFlags::SHADER_DEVICE_ADDRESS,
                vk_mem::MemoryUsage::GpuOnly,
            )
        }
    }

    pub(crate) fn alloc_acceleration_structure_instance_buffer(
        &self,
        size: vk::DeviceSize,
    ) -> Result<VkBufferHandle> {
        unsafe {
            self.create_buffer(
                size,
                vk::BufferUsageFlags::TRANSFER_DST
                    | vk::BufferUsageFlags::ACCELERATION_STRUCTURE_BUILD_INPUT_READ_ONLY_KHR
                    | vk::BufferUsageFlags::SHADER_DEVICE_ADDRESS,
                vk_mem::MemoryUsage::GpuOnly,
            )
        }
    }
}

// Low level methods
impl PompeiiRenderer {
    #[inline]
    unsafe fn create_buffer(
        &self,
        size: vk::DeviceSize,
        usage: vk::BufferUsageFlags,
        location: vk_mem::MemoryUsage,
    ) -> Result<VkBufferHandle> {
        self.create_buffer_from_pool(size, usage, location, ptr::null_mut())
    }

    #[inline]
    unsafe fn create_buffer_from_pool(
        &self,
        size: vk::DeviceSize,
        usage: vk::BufferUsageFlags,
        location: vk_mem::MemoryUsage,
        pool: vk_mem::AllocatorPool,
    ) -> Result<VkBufferHandle> {
        trace!("Creating buffer:");
        trace!("- Size: {}", size);
        trace!("- {:?}", usage);
        trace!("- {:?}", location);
        if !pool.is_null() {
            trace!("- Pool: {:?}", pool);
        }

        Ok(self
            .vma
            .create_buffer(
                &vk::BufferCreateInfo::builder()
                    .size(size)
                    .usage(usage)
                    .sharing_mode(vk::SharingMode::EXCLUSIVE),
                &vk_mem::AllocationCreateInfo::new()
                    .usage(location)
                    .pool(pool),
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
