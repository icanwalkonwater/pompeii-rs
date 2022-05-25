use std::sync::Arc;

pub use ash;
use ash::vk;
use log::debug;
use parking_lot::{Mutex, RwLock};
pub use vk_mem;

use debug_utils::DebugUtils;
use setup::*;

use crate::{
    alloc::VmaPools,
    swapchain::{SurfaceWrapper, SwapchainWrapper},
};

pub mod acceleration_structure;
pub mod alloc;
mod commands;
mod debug_utils;
mod images;
pub mod mesh;
mod render;
pub mod setup;
mod swapchain;
mod sync;
pub(crate) mod utils;

pub mod errors {
    use thiserror::Error;

    pub type Result<T> = std::result::Result<T, PompeiiError>;

    #[derive(Error, Debug)]
    pub enum PompeiiError {
        #[error("{0}")]
        Generic(String),
        #[error("{0}")]
        LoadingError(#[from] ash::LoadingError),
        //#[error("{0}")]
        //InstanceError(#[from] ash::InstanceError),
        #[error("{0}")]
        VkError(#[from] ash::vk::Result),
        //#[error("{0}")]
        //VmaError(#[from] vk_mem::Error),
        #[error("No graphics queue found (wtf)")]
        NoGraphicsQueue,
        #[error("No present queue found (wtf)")]
        NoPresentQueue,
        #[error("No compute queue found")]
        NoComputeQueue,
        #[error("No transfer queue found")]
        NoTransferQueue,
        #[error("No physical device picked")]
        NoPhysicalDevicePicked,
        #[error("No compatible color format found")]
        NoCompatibleColorFormatFound,
        #[error("Missing vertex position component")]
        NoVertexPosition,
        #[error("Missing vertex normal component")]
        NoVertexNormal,
        #[error("Missing vertex UV component")]
        NoVertexUv,
        #[error("Not an indexed model")]
        NoModelIndices,
    }
}

pub struct PompeiiRenderer {
    pub(crate) _entry: ash::Entry,
    pub(crate) instance: ash::Instance,
    pub(crate) debug_utils: DebugUtils,
    pub(crate) physical_device: vk::PhysicalDevice,
    pub(crate) device: ash::Device,
    pub(crate) vma: Arc<vk_mem::Allocator>,
    pub(crate) vma_pools: VmaPools,
    pub(crate) queues: DeviceQueues,
    pub(crate) surface: SurfaceWrapper,
    pub(crate) swapchain: Arc<RwLock<SwapchainWrapper>>,
    pub(crate) ext_acceleration_structure: ash::extensions::khr::AccelerationStructure,
    pub(crate) ext_ray_tracing_pipeline: ash::extensions::khr::RayTracingPipeline,

    // Deletion queue for main objects that are freed when the renderer is dropped
    pub(crate) main_deletion_queue:
        Mutex<Vec<Box<dyn FnOnce(&PompeiiRenderer) -> errors::Result<()> + Send + Sync>>>,
    // Deletion queue for the allocations that need freeing
    pub(crate) alloc_deletion_queue: Mutex<
        Vec<
            Box<
                dyn FnOnce(
                        (&ash::Device, &ash::extensions::khr::AccelerationStructure),
                        &Arc<vk_mem::Allocator>,
                    ) -> errors::Result<()>
                    + Send
                    + Sync,
            >,
        >,
    >,

    pub(crate) image_available_semaphore: vk::Semaphore,
    pub(crate) render_finished_semaphore: vk::Semaphore,
    pub(crate) in_flight_fence: vk::Fence,
}

impl Drop for PompeiiRenderer {
    fn drop(&mut self) {
        unsafe {
            // Wait for frame to finish
            self.device
                .wait_for_fences(&[self.in_flight_fence], true, u64::MAX)
                .unwrap();

            // Free everything
            let mut alloc_deletion_queue = self.alloc_deletion_queue.lock();
            for free in alloc_deletion_queue.drain(..).rev() {
                free((&self.device, &self.ext_acceleration_structure), &self.vma).unwrap();
            }

            debug!("Freed everything");

            // VMA
            let vma = Arc::get_mut(&mut self.vma).unwrap();
            vma.destroy_pool(self.vma_pools.acceleration_structures);
            vma.destroy();

            debug!("Destroyed VMA");

            // Trigger main deletion queue
            let mut main_deletion_queue = self.main_deletion_queue.lock();
            for action in main_deletion_queue.drain(..).rev() {
                action(self).unwrap();
            }

            debug!("Cleanup done !");
        }
    }
}
