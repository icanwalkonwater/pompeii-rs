use std::{mem::ManuallyDrop, sync::Arc};

mod alloc;
mod debug_utils;
pub mod setup;
mod swapchain;

use debug_utils::DebugUtils;
use setup::*;

pub mod errors {
    use thiserror::Error;

    pub type Result<T> = std::result::Result<T, PompeiiError>;

    #[derive(Error, Debug)]
    pub enum PompeiiError {
        #[error("{0}")]
        LoadingError(#[from] ash::LoadingError),
        //#[error("{0}")]
        //InstanceError(#[from] ash::InstanceError),
        #[error("{0}")]
        VkError(#[from] ash::vk::Result),
        //#[error("{0}")]
        //VmaError(#[from] vk_mem::Error),
        #[error("No compute queue found")]
        NoComputeQueue,
        #[error("No transfer queue found")]
        NoTransferQueue,
        #[error("No physical device picked")]
        NoPhysicalDevicePicked,
    }
}

pub struct PompeiiRenderer {
    pub(crate) _entry: ash::Entry,
    pub(crate) instance: ash::Instance,
    pub(crate) debug_utils: ManuallyDrop<DebugUtils>,
    pub(crate) device: ash::Device,
    pub(crate) vma: Arc<vk_mem::Allocator>,
    pub(crate) queues: DeviceQueues,
}

impl Drop for PompeiiRenderer {
    fn drop(&mut self) {
        unsafe {
            // TODO: add destroys here

            Arc::get_mut(&mut self.vma)
                .expect("There still are buffers around referencing VMA !")
                .destroy();

            self.device.destroy_device(None);
            ManuallyDrop::drop(&mut self.debug_utils);
            self.instance.destroy_instance(None);
        }
    }
}
