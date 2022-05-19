use std::{mem::ManuallyDrop, sync::Arc};

use ash::vk;

use debug_utils::DebugUtils;
use setup::*;

use crate::swapchain::{SurfaceWrapper, SwapchainWrapper};

pub mod alloc;
mod commands;
mod debug_utils;
mod images;
pub mod mesh;
mod render;
pub mod setup;
pub(crate) mod store;
mod swapchain;
mod sync;

use crate::store::PompeiiStore;
pub use ash;

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
    pub(crate) debug_utils: ManuallyDrop<DebugUtils>,
    pub(crate) physical_device: vk::PhysicalDevice,
    pub(crate) device: ash::Device,
    pub(crate) vma: Arc<vk_mem::Allocator>,
    pub(crate) queues: DeviceQueues,
    pub(crate) surface: SurfaceWrapper,
    pub(crate) swapchain: SwapchainWrapper,
    pub(crate) ext_sync2: ash::extensions::khr::Synchronization2,

    pub(crate) store: PompeiiStore,

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

            // TODO: add destroys here

            // VMA
            let vma = Arc::get_mut(&mut self.vma)
                .expect("There still are buffers around referencing VMA !");

            self.store.cleanup(vma);
            vma.destroy();

            // Sync
            self.device
                .destroy_semaphore(self.image_available_semaphore, None);
            self.device
                .destroy_semaphore(self.render_finished_semaphore, None);
            self.device.destroy_fence(self.in_flight_fence, None);

            // Queue pools
            self.queues.destroy_pools(&self.device);

            // Swapchain
            self.swapchain.cleanup(&self.device, true);

            // Surface
            self.surface.ext.destroy_surface(self.surface.handle, None);

            // Device & instance
            self.device.destroy_device(None);
            ManuallyDrop::drop(&mut self.debug_utils);
            self.instance.destroy_instance(None);
        }
    }
}
