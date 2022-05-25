use std::{io::Write, os::raw::c_char, sync::Arc};

use ash::vk;
use log::debug;
use parking_lot::{lock_api::Mutex, RwLock};

use crate::{
    debug_utils::DebugUtils,
    errors::{PompeiiError, Result},
    setup::{
        initializer::PompeiiInitializer,
        physical_device::PhysicalDeviceInfo,
        queues_finder::{DeviceQueues, PhysicalDeviceQueueIndices},
    },
    swapchain::{SurfaceWrapper, SwapchainWrapper},
    PompeiiRenderer, VmaPools, VULKAN_VERSION,
};

type DeviceAdapter = (PhysicalDeviceInfo, PhysicalDeviceQueueIndices);

pub struct PompeiiBuilder {
    pub(crate) entry: ash::Entry,
    pub(crate) instance: ash::Instance,
    pub(crate) ext_surface_capabilities2: ash::extensions::khr::GetSurfaceCapabilities2,
    pub(crate) debug_utils: DebugUtils,
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
            debug_utils,
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
        let mut main_deletion_queue =
            Vec::<Box<dyn FnOnce(&PompeiiRenderer) -> Result<()> + Send + Sync>>::new();

        // Mark instance and debug utils for deletion
        main_deletion_queue.push(Box::new(|r| unsafe {
            r.instance.destroy_instance(None);
            Ok(())
        }));

        main_deletion_queue.push(Box::new(|r| unsafe {
            r.debug_utils.destroy();
            Ok(())
        }));

        main_deletion_queue.push(Box::new(|r| unsafe {
            r.surface.ext.destroy_surface(r.surface.handle, None);
            Ok(())
        }));

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

            // TODO check if features are available
            // TODO enabled things maybe
            let mut vk12_features = vk::PhysicalDeviceVulkan12Features::builder()
                .descriptor_indexing(true)
                .buffer_device_address(true);

            let mut vk13_features = vk::PhysicalDeviceVulkan13Features::builder()
                .dynamic_rendering(true)
                .synchronization2(true);

            let mut acceleration_structure_features =
                vk::PhysicalDeviceAccelerationStructureFeaturesKHR::builder()
                    .acceleration_structure(true);

            let mut ray_tracing_pipeline_features =
                vk::PhysicalDeviceRayTracingPipelineFeaturesKHR::builder()
                    .ray_tracing_pipeline(true);

            // TODO enabled things maybe
            let features = vk::PhysicalDeviceFeatures::builder();

            unsafe {
                self.instance.create_device(
                    physical.0.handle,
                    &vk::DeviceCreateInfo::builder()
                        .push_next(&mut vk12_features)
                        .push_next(&mut vk13_features)
                        .push_next(&mut acceleration_structure_features)
                        .push_next(&mut ray_tracing_pipeline_features)
                        .enabled_features(&features)
                        .enabled_extension_names(&self.device_extensions)
                        .queue_create_infos(&queue_create_info),
                    None,
                )?
            }
        };

        main_deletion_queue.push(Box::new(|r| unsafe {
            r.device.destroy_device(None);
            Ok(())
        }));

        let vma = Arc::new(vk_mem::Allocator::new(
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
        )?);

        let vma_pool_acceleration_structure = unsafe {
            let alignment = {
                let mut accel_props =
                    vk::PhysicalDeviceAccelerationStructurePropertiesKHR::default();
                let mut props =
                    vk::PhysicalDeviceProperties2::builder().push_next(&mut accel_props);
                self.instance
                    .get_physical_device_properties2(physical_device.0.handle, &mut props);

                accel_props.min_acceleration_structure_scratch_offset_alignment
            };

            let memory_type = vma.find_memory_type_index_for_buffer_info(
                &vk::BufferCreateInfo::builder().size(1).usage(
                    vk::BufferUsageFlags::STORAGE_BUFFER
                        | vk::BufferUsageFlags::SHADER_DEVICE_ADDRESS,
                ),
                &vk_mem::AllocationCreateInfo::new().usage(vk_mem::MemoryUsage::GpuOnly),
            )?;

            vma.create_pool(
                &vk_mem::PoolCreateInfo::new()
                    .memory_type_index(memory_type)
                    .flags(&vk_mem::AllocatorPoolCreateFlags::NONE)
                    .min_allocation_alignment(alignment as _),
            )?
        };

        // NOTE: VMA destruction is a special case and is not in the main deletion queue

        let queues = DeviceQueues::new(&device, &self.physical_device.as_ref().unwrap().1)?;

        main_deletion_queue.push(Box::new(|r| unsafe {
            r.queues.destroy_pools(&r.device);
            Ok(())
        }));

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

        main_deletion_queue.push(Box::new(|r| unsafe {
            let mut swapchain = r.swapchain.write();
            swapchain.cleanup(&r.device, true);
            Ok(())
        }));

        let ext_acceleration_structure =
            ash::extensions::khr::AccelerationStructure::new(&self.instance, &device);
        let ext_ray_tracing_pipeline =
            ash::extensions::khr::RayTracingPipeline::new(&self.instance, &device);

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

        main_deletion_queue.push(Box::new(|r| unsafe {
            r.device
                .destroy_semaphore(r.image_available_semaphore, None);
            r.device
                .destroy_semaphore(r.render_finished_semaphore, None);
            r.device.destroy_fence(r.in_flight_fence, None);
            Ok(())
        }));

        Ok(PompeiiRenderer {
            _entry: self.entry,
            instance: self.instance,
            debug_utils: self.debug_utils,
            physical_device: physical_device.0.handle,
            device,
            vma,
            vma_pools: VmaPools {
                acceleration_structures: vma_pool_acceleration_structure,
            },
            queues,
            surface: self.surface,
            swapchain: Arc::new(RwLock::new(swapchain)),
            ext_acceleration_structure,
            ext_ray_tracing_pipeline,

            main_deletion_queue: Mutex::new(main_deletion_queue),
            alloc_deletion_queue: Default::default(),

            image_available_semaphore,
            render_finished_semaphore,
            in_flight_fence,
        })
    }
}
