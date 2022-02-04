use crate::{
    debug_utils::DebugUtils,
    errors::{PompeiiError, Result},
    setup::{
        initializer::PompeiiInitializer,
        physical_device::PhysicalDeviceInfo,
        queues_finder::{DeviceQueues, PhysicalDeviceQueueIndices},
    },
    PompeiiRenderer, VULKAN_VERSION,
};
use ash::vk;
use std::{ffi::CStr, mem::ManuallyDrop, os::raw::c_char, sync::Arc};

type DeviceAdapter = (vk::PhysicalDevice, PhysicalDeviceQueueIndices);

pub struct PompeiiBuilder {
    pub(crate) entry: ash::Entry,
    pub(crate) instance: ash::Instance,
    pub(crate) debug_utils: ManuallyDrop<DebugUtils>,
    physical_device: Option<DeviceAdapter>,
    device_extensions: Vec<*const c_char>,
}

impl PompeiiBuilder {
    pub fn builder() -> PompeiiInitializer {
        PompeiiInitializer::default()
    }

    pub(crate) fn new(entry: ash::Entry, instance: ash::Instance, debug_utils: DebugUtils) -> Self {
        Self {
            entry,
            instance,
            debug_utils: ManuallyDrop::new(debug_utils),
            physical_device: None,
            device_extensions: vec![],
        }
    }

    pub fn set_physical_device(mut self, device: PhysicalDeviceInfo) -> Self {
        let queues = PhysicalDeviceQueueIndices::from_device(&device).unwrap();
        self.physical_device = Some((device.handle, queues));
        self
    }

    pub fn with_device_extension(mut self, name: &'static CStr) -> Self {
        self.device_extensions.push(name.as_ptr());
        self
    }

    pub fn build(self) -> Result<Arc<PompeiiRenderer>> {
        let device = {
            let physical = self
                .physical_device
                .as_ref()
                .ok_or(PompeiiError::NoPhysicalDevicePicked)?;
            let queue_create_info = physical.1.as_queue_create_info();

            // TODO enabled things maybe
            let mut descriptor_indexing_features =
                vk::PhysicalDeviceDescriptorIndexingFeatures::builder();

            // TODO enabled things maybe
            let features = vk::PhysicalDeviceFeatures::builder();

            unsafe {
                self.instance.create_device(
                    physical.0,
                    &vk::DeviceCreateInfo::builder()
                        .push_next(&mut descriptor_indexing_features)
                        .enabled_features(&features)
                        .enabled_extension_names(&self.device_extensions)
                        .queue_create_infos(&queue_create_info),
                    None,
                )?
            }
        };

        let vma = unsafe {
            vk_mem::Allocator::new(&vk_mem::AllocatorCreateInfo {
                instance: self.instance.clone(),
                device: device.clone(),
                physical_device: self.physical_device.as_ref().unwrap().0,
                flags: vk_mem::AllocatorCreateFlags::KHR_DEDICATED_ALLOCATION,
                preferred_large_heap_block_size: 0,
                frame_in_use_count: 0,
                heap_size_limits: None,
                allocation_callbacks: None,
                vulkan_api_version: VULKAN_VERSION,
            })
        }?;

        let queues = DeviceQueues::new(&device, &self.physical_device.as_ref().unwrap().1)?;

        Ok(Arc::new(PompeiiRenderer {
            _entry: self.entry,
            instance: self.instance,
            debug_utils: self.debug_utils,
            device,
            vma: Arc::new(vma),
            queues,
        }))
    }
}
