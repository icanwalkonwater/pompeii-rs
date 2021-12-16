use crate::{
    errors::{Result, VmaErrorExt},
    PompeiiVulkanBackend,
};
use ash::vk;
use pompeii_hal::{
    alloc::{AllocInfo, MemoryLocation, MemoryUsage, PompeiiAllocator},
    PompeiiBackend,
};
use std::sync::Arc;

fn translate_buffer_usage(usage: MemoryUsage) -> vk::BufferUsageFlags {
    vk::BufferUsageFlags::from_raw(usage.bits())
}

fn translate_buffer_location(location: MemoryLocation) -> vk_mem::MemoryUsage {
    match location {
        MemoryLocation::Gpu => vk_mem::MemoryUsage::GpuOnly,
        MemoryLocation::Cpu => vk_mem::MemoryUsage::CpuOnly,
        MemoryLocation::CpuToGpu => vk_mem::MemoryUsage::CpuToGpu,
        MemoryLocation::GpuToCpu => vk_mem::MemoryUsage::GpuToCpu,
    }
}

pub struct PompeiiVulkanAllocator {
    vma: Arc<vk_mem::Allocator>,
}

impl PompeiiVulkanAllocator {
    fn create_buffer(
        &self,
        size: vk::DeviceSize,
        usage: vk::BufferUsageFlags,
        location: vk_mem::MemoryUsage,
    ) -> Result<(vk::Buffer, vk_mem::Allocation, vk_mem::AllocationInfo)> {
        self.vma.create_buffer(
            &vk::BufferCreateInfo::builder()
                .size(size)
                .usage(usage)
                .sharing_mode(vk::SharingMode::EXCLUSIVE),
            &vk_mem::AllocationCreateInfo {
                usage: location,
                ..Default::default()
            },
        ).map_err_pompeii()
    }
}

impl PompeiiAllocator for PompeiiVulkanAllocator {
    type Backend = PompeiiVulkanBackend;
    type BufferHandle = (vk::Buffer, vk_mem::Allocation, vk_mem::AllocationInfo);

    fn alloc_staging_buffer(
        &self,
        size: u64,
    ) -> pompeii_hal::errors::Result<
        <<Self as PompeiiAllocator>::Backend as PompeiiBackend>::Error,
        Self::BufferHandle,
    > {
        self.create_buffer(
            size,
            vk::BufferUsageFlags::TRANSFER_SRC,
            vk_mem::MemoryUsage::CpuOnly,
        )
    }
}
