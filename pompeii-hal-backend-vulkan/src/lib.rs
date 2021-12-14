use crate::{
    builder::PompeiiVulkanBuilder, debug_utils::DebugUtils, errors::VulkanError,
    initializer::PompeiiVulkanInitializer, queues::DeviceQueues,
};
use pompeii_hal::PompeiiBackend;
use std::{mem::ManuallyDrop, sync::Arc};

pub mod builder;
pub mod debug_utils;
pub mod initializer;
pub mod physical_device;
pub mod queues;

pub mod errors {

    use pompeii_hal::errors::BackendError;
    use thiserror::Error;

    pub type Result<T> = pompeii_hal::errors::Result<VulkanError, T>;

    #[derive(Error, Debug)]
    pub enum VulkanError {
        #[error("{0}")]
        LoadingError(#[from] ash::LoadingError),
        #[error("{0}")]
        InstanceError(#[from] ash::InstanceError),
        #[error("{0}")]
        VkError(#[from] ash::vk::Result),
        #[error("{0}")]
        VmaError(#[from] vk_mem::Error),
        #[error("No compute queue found")]
        NoComputeQueue,
        #[error("No transfer queue found")]
        NoTransferQueue,
        #[error("No physical device picked")]
        NoPhysicalDevicePicked,
    }

    // Intellij-Rust user might see an error here, its not
    impl BackendError for VulkanError {}

    // `thiserror` can't convert from two levels of error so we do it ourselves
    // <editor-fold>
    pub(crate) trait VkErrorExt<T> {
        fn map_err_pompeii(self) -> Result<T>;
    }

    impl<T> VkErrorExt<T> for ash::prelude::VkResult<T> {
        fn map_err_pompeii(self) -> Result<T> {
            Ok(self.map_err(|e| VulkanError::VkError(e))?)
        }
    }

    pub(crate) trait VmaErrorExt<T> {
        fn map_err_pompeii(self) -> Result<T>;
    }

    impl<T> VmaErrorExt<T> for vk_mem::Result<T> {
        fn map_err_pompeii(self) -> Result<T> {
            Ok(self.map_err(|e| VulkanError::VmaError(e))?)
        }
    }
    // </editor-fold>
}

pub struct PompeiiVulkanBackend {
    pub(crate) _entry: ash::Entry,
    pub(crate) instance: ash::Instance,
    pub(crate) debug_utils: ManuallyDrop<DebugUtils>,
    pub(crate) device: ash::Device,
    pub(crate) vma: Arc<vk_mem::Allocator>,
    pub(crate) queues: DeviceQueues,
}

impl Drop for PompeiiVulkanBackend {
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

impl PompeiiBackend for PompeiiVulkanBackend {
    type Error = VulkanError;
    type Initializer = PompeiiVulkanInitializer;
    type Builder = PompeiiVulkanBuilder;
}
