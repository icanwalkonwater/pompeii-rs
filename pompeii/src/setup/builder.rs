use std::{mem::ManuallyDrop, os::raw::c_char, sync::Arc};

use ash::vk;

use crate::{
    debug_utils::DebugUtils,
    errors::{PompeiiError, Result},
    setup::{
        initializer::PompeiiInitializer,
        physical_device::PhysicalDeviceInfo,
        queues_finder::{DeviceQueues, PhysicalDeviceQueueIndices},
    },
    swapchain::{SurfaceWrapper, SwapchainWrapper},
    PompeiiRenderer, PompeiiStore, VULKAN_VERSION,
};

type DeviceAdapter = (PhysicalDeviceInfo, PhysicalDeviceQueueIndices);

pub struct PompeiiBuilder {
    pub(crate) entry: ash::Entry,
    pub(crate) instance: ash::Instance,
    pub(crate) ext_surface_capabilities2: ash::extensions::khr::GetSurfaceCapabilities2,
    pub(crate) debug_utils: ManuallyDrop<DebugUtils>,
    pub(crate) device_extensions: Vec<*const c_char>,
    pub(crate) surface: SurfaceWrapper,
    physical_device: Option<DeviceAdapter>,
}

impl PompeiiBuilder {
    pub fn builder() -> PompeiiInitializer {
        PompeiiInitializer::default()
    }

    pub(crate) fn new(
        entry: ash::Entry,
        instance: ash::Instance,
        debug_utils: DebugUtils,
        device_extensions: Vec<*const c_char>,
        surface: SurfaceWrapper,
    ) -> Self {
        let ext_surface_capabilities2 =
            ash::extensions::khr::GetSurfaceCapabilities2::new(&entry, &instance);

        Self {
            entry,
            instance,
            ext_surface_capabilities2,
            debug_utils: ManuallyDrop::new(debug_utils),
            surface,
            physical_device: None,
            device_extensions,
        }
    }

    pub fn set_physical_device(mut self, device: PhysicalDeviceInfo) -> Self {
        let queues = PhysicalDeviceQueueIndices::from_device(&device, &self.surface).unwrap();
        self.physical_device = Some((device, queues));
        self
    }

    pub fn build(self, window_size: (u32, u32)) -> Result<PompeiiRenderer> {
        let physical_device = self
            .physical_device
            .as_ref()
            .ok_or(PompeiiError::NoPhysicalDevicePicked)?;

        let device = {
            let physical = self
                .physical_device
                .as_ref()
                .ok_or(PompeiiError::NoPhysicalDevicePicked)?;
            let queue_create_info = physical.1.as_queue_create_info();

            // TODO enabled things maybe
            let mut vk12_features = vk::PhysicalDeviceVulkan12Features::builder()
                .descriptor_indexing(true)
                .buffer_device_address(true);

            let mut vk13_features = vk::PhysicalDeviceVulkan13Features::builder()
                .dynamic_rendering(true);

            // TODO enabled things maybe
            let features = vk::PhysicalDeviceFeatures::builder();

            unsafe {
                self.instance.create_device(
                    physical.0.handle,
                    &vk::DeviceCreateInfo::builder()
                        .push_next(&mut vk12_features)
                        .push_next(&mut vk13_features)
                        .enabled_features(&features)
                        .enabled_extension_names(&self.device_extensions)
                        .queue_create_infos(&queue_create_info),
                    None,
                )?
            }
        };

        let vma = vk_mem::Allocator::new(
            vk_mem::AllocatorCreateInfo::new(
                &self.instance,
                &device,
                &self.physical_device.as_ref().unwrap().0.handle,
            )
            // TODO: when fixed in master
            .flags(unsafe {
                vk_mem::AllocationCreateFlags::from_bits_unchecked(
                    (vk_mem::AllocatorCreateFlags::KHR_DEDICATED_ALLOCATION
                        | vk_mem::AllocatorCreateFlags::BUFFER_DEVICE_ADDRESS)
                        .bits(),
                )
            })
            .vulkan_api_version(VULKAN_VERSION),
        )?;

        let queues = DeviceQueues::new(&device, &self.physical_device.as_ref().unwrap().1)?;

        let swapchain = {
            let ext = ash::extensions::khr::Swapchain::new(&self.instance, &device);
            let (handle, images, image_views, format, extent) = PompeiiRenderer::create_swapchain(
                &device,
                &ext,
                physical_device.0.surface_capabilities.as_ref().unwrap(),
                &self.surface,
                window_size,
                None,
            )?;

            SwapchainWrapper {
                ext,
                handle,
                images,
                image_views,
                format,
                extent,
            }
        };

        let ext_sync2 = ash::extensions::khr::Synchronization2::new(&self.instance, &device);

        // Sync
        let image_available_semaphore =
            unsafe { device.create_semaphore(&vk::SemaphoreCreateInfo::default(), None)? };
        let render_finished_semaphore =
            unsafe { device.create_semaphore(&vk::SemaphoreCreateInfo::default(), None)? };
        let in_flight_fence = unsafe {
            device.create_fence(
                &vk::FenceCreateInfo::builder().flags(vk::FenceCreateFlags::SIGNALED),
                None,
            )?
        };

        Ok(PompeiiRenderer {
            _entry: self.entry,
            instance: self.instance,
            debug_utils: self.debug_utils,
            physical_device: physical_device.0.handle,
            device,
            vma: Arc::new(vma),
            queues,
            surface: self.surface,
            swapchain,
            ext_sync2,

            store: PompeiiStore::default(),

            image_available_semaphore,
            render_finished_semaphore,
            in_flight_fence,
        })
    }
}
