use std::collections::HashSet;

use ash::vk;
use parking_lot::{ReentrantMutex, ReentrantMutexGuard};

use crate::{
    errors::{PompeiiError, Result},
    setup::physical_device::PhysicalDeviceInfo,
    swapchain::SurfaceWrapper,
};

/// Represents the queue indices to use for graphics, compute and transfer.
///
/// When created, it tries to find a queue family that isn't shared with graphics
/// but will fallback on whatever is available.
///
/// So these indices can overlap.
#[derive(Debug, Clone)]
pub(crate) struct PhysicalDeviceQueueIndices {
    pub(crate) graphics: u32,
    pub(crate) present: u32,
    pub(crate) compute: u32,
    pub(crate) transfer: u32,
}

const QUEUE_PRIORITIES_ONE: [f32; 1] = [1.0];

impl PhysicalDeviceQueueIndices {
    pub(crate) fn from_device(info: &PhysicalDeviceInfo, surface: &SurfaceWrapper) -> Result<Self> {
        Ok(Self {
            graphics: Self::find_graphics_queue(info)?,
            present: Self::find_present_queue(info, surface)?,
            compute: Self::find_compute_queue(info)?,
            transfer: Self::find_transfer_queue(info)?,
        })
    }

    pub(crate) fn as_queue_create_info(&self) -> Vec<vk::DeviceQueueCreateInfo> {
        let mut unique_families = HashSet::with_capacity(4);
        unique_families.insert(self.graphics);
        unique_families.insert(self.present);
        unique_families.insert(self.compute);
        unique_families.insert(self.transfer);

        unique_families
            .into_iter()
            .map(|family| {
                vk::DeviceQueueCreateInfo::builder()
                    .queue_family_index(family)
                    // SAFETY: It's const so its ok I guess
                    .queue_priorities(&QUEUE_PRIORITIES_ONE)
                    .build()
            })
            .collect::<Vec<_>>()
    }

    fn find_graphics_queue(info: &PhysicalDeviceInfo) -> Result<u32> {
        info.queue_families
            .iter()
            .enumerate()
            .find(|(_, queue)| {
                queue
                    .queue_family_properties
                    .queue_flags
                    .contains(vk::QueueFlags::GRAPHICS)
            })
            .map(|(i, _)| i as _)
            .ok_or(PompeiiError::NoGraphicsQueue)
    }

    fn find_present_queue(info: &PhysicalDeviceInfo, surface: &SurfaceWrapper) -> Result<u32> {
        // TODO: maybe try to get a queue that is explicitly not shared
        info.queue_families
            .iter()
            .enumerate()
            .find(|(queue, _)| unsafe {
                surface
                    .ext
                    .get_physical_device_surface_support(info.handle, *queue as _, surface.handle)
                    .unwrap()
            })
            .map(|(i, _)| i as _)
            .ok_or(PompeiiError::NoPresentQueue)
    }

    fn find_compute_queue(info: &PhysicalDeviceInfo) -> Result<u32> {
        Self::try_find_queue_not_shared_with(
            info,
            vk::QueueFlags::COMPUTE,
            vk::QueueFlags::GRAPHICS,
        )
        .ok_or(PompeiiError::NoComputeQueue)
    }

    fn find_transfer_queue(info: &PhysicalDeviceInfo) -> Result<u32> {
        Self::try_find_queue_not_shared_with(
            info,
            vk::QueueFlags::TRANSFER,
            vk::QueueFlags::GRAPHICS,
        )
        .ok_or(PompeiiError::NoTransferQueue)
    }

    /// Helper
    fn try_find_queue_not_shared_with(
        info: &PhysicalDeviceInfo,
        to_find: vk::QueueFlags,
        to_avoid: vk::QueueFlags,
    ) -> Option<u32> {
        info.queue_families
            .iter()
            .enumerate()
            .filter(|(_, queue)| queue.queue_family_properties.queue_flags.contains(to_find))
            .fold(None, |acc, (i, queue)| {
                // Try to get one that isn't also used for graphics
                if let Some((prev_i, prev_queue)) = acc {
                    if !queue.queue_family_properties.queue_flags.contains(to_avoid) {
                        Some((i, queue))
                    } else {
                        Some((prev_i, prev_queue))
                    }
                } else {
                    Some((i, queue))
                }
            })
            .map(|(i, _)| i as _)
    }
}

pub(crate) struct QueueWithPool {
    pub(crate) queue: vk::Queue,
    pub(crate) pool: vk::CommandPool,
}

pub(crate) struct DeviceQueues {
    // Holds unique queues, with a maximum of 4 which only happens if we don't share any queue
    pub(crate) queues: [Option<ReentrantMutex<QueueWithPool>>; 4],
    pub(crate) graphics_index: usize,
    pub(crate) present_index: usize,
    pub(crate) compute_index: usize,
    pub(crate) transfer_index: usize,
}

impl DeviceQueues {
    pub(crate) fn new(device: &ash::Device, indices: &PhysicalDeviceQueueIndices) -> Result<Self> {
        unsafe {
            let mut queues = [None, None, None, None];

            let graphics = 0;
            queues[graphics] = Some(ReentrantMutex::new(Self::retrieve_queue_and_pool(
                device,
                indices.graphics,
                Default::default(),
            )?));

            let present = if indices.present != indices.graphics {
                queues[1] = Some(ReentrantMutex::new(Self::retrieve_queue_and_pool(
                    device,
                    indices.present,
                    Default::default(),
                )?));
                1
            } else {
                0
            };

            let compute = if indices.compute == indices.graphics {
                graphics
            } else if indices.compute == indices.present {
                present
            } else {
                queues[2] = Some(ReentrantMutex::new(Self::retrieve_queue_and_pool(
                    device,
                    indices.compute,
                    Default::default(),
                )?));
                2
            };

            let transfer = if indices.transfer == indices.graphics {
                graphics
            } else if indices.transfer == indices.present {
                present
            } else if indices.transfer == indices.compute {
                compute
            } else {
                queues[3] = Some(ReentrantMutex::new(Self::retrieve_queue_and_pool(
                    device,
                    indices.transfer,
                    Default::default(),
                )?));
                3
            };

            Ok(Self {
                queues,
                graphics_index: graphics,
                present_index: present,
                compute_index: compute,
                transfer_index: transfer,
            })
        }
    }

    unsafe fn retrieve_queue_and_pool(
        device: &ash::Device,
        family_index: u32,
        flags: vk::CommandPoolCreateFlags,
    ) -> Result<QueueWithPool> {
        let queue = device.get_device_queue(family_index, 0);
        let pool = device.create_command_pool(
            &vk::CommandPoolCreateInfo::builder()
                .queue_family_index(family_index)
                .flags(flags),
            None,
        )?;

        Ok(QueueWithPool { queue, pool })
    }

    pub(crate) fn graphics(&self) -> ReentrantMutexGuard<QueueWithPool> {
        self.queues[self.graphics_index].as_ref().unwrap().lock()
    }

    pub(crate) fn present(&self) -> ReentrantMutexGuard<QueueWithPool> {
        self.queues[self.present_index].as_ref().unwrap().lock()
    }

    pub(crate) fn compute(&self) -> ReentrantMutexGuard<QueueWithPool> {
        self.queues[self.compute_index].as_ref().unwrap().lock()
    }

    pub(crate) fn transfer(&self) -> ReentrantMutexGuard<QueueWithPool> {
        self.queues[self.transfer_index].as_ref().unwrap().lock()
    }

    pub(crate) unsafe fn destroy_pools(&self, device: &ash::Device) {
        for queue in self.queues.iter().flatten() {
            device.destroy_command_pool(queue.lock().pool, None);
        }
    }
}
