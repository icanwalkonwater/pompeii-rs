use crate::{errors::Result, PompeiiBackend};
use bitflags::bitflags;

pub trait PompeiiAllocator {
    type Backend: PompeiiBackend;
    type BufferHandle: Sized;

    fn alloc_staging_buffer(
        &self,
        size: u64,
    ) -> Result<<<Self as PompeiiAllocator>::Backend as PompeiiBackend>::Error, Self::BufferHandle>;
}

pub enum MemoryLocation {
    Gpu,
    Cpu,
    CpuToGpu,
    GpuToCpu,
}

bitflags! {
    // To correspond to the vulkan equivalent and save some unnecessary translations.
    pub struct MemoryUsage: u32 {
        const STAGING = 0b1;
        const UNIFORM_BUFFER = 0b1_0000;
        const STORAGE_BUFFER = 0b10_0000;
        const INDEX_BUFFER = 0b100_0000;
        const VERTEX_BUFFER = 0b1000_0000;
    }
}

pub struct AllocInfo {
    pub location: MemoryLocation,
    pub usage: MemoryUsage,
}

pub trait BufferHandle<T: Sized> {
    fn size(&self) -> u64;
}
