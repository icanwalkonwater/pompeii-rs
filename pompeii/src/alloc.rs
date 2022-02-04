use crate::errors::Result;
use ash::vk;
use std::sync::Arc;

pub struct PompeiiAllocator {
    vma: Arc<vk_mem::Allocator>,
}

impl PompeiiAllocator {
    unsafe fn create_buffer(
        &self,
        size: vk::DeviceSize,
        usage: vk::BufferUsageFlags,
        location: vk_mem::MemoryUsage,
    ) -> Result<(vk::Buffer, vk_mem::Allocation, vk_mem::AllocationInfo)> {
        Ok(self.vma.create_buffer(
            &vk::BufferCreateInfo::builder()
                .size(size)
                .usage(usage)
                .sharing_mode(vk::SharingMode::EXCLUSIVE)
                .build(),
            &vk_mem::AllocationCreateInfo {
                usage: location,
                ..Default::default()
            },
        )?)
    }
}

pub type BufferHandle = (vk::Buffer, vk_mem::Allocation, vk_mem::AllocationInfo);

impl PompeiiAllocator {
    fn alloc_staging_buffer(&self, size: u64) -> Result<BufferHandle> {
        self.create_buffer(
            size,
            vk::BufferUsageFlags::TRANSFER_SRC,
            vk_mem::MemoryUsage::CpuOnly,
        )
    }
}
