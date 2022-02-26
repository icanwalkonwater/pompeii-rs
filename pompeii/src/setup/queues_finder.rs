use ash::vk;
use ash::vk::DeviceQueueCreateInfoBuilder;
use parking_lot::{Mutex, MutexGuard};

use crate::{
    errors::{PompeiiError, Result},
    setup::physical_device::PhysicalDeviceInfo,
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
    pub(crate) compute: u32,
    pub(crate) transfer: u32,
}

const QUEUE_PRIORITIES_ONE: [f32; 1] = [1.0];

impl PhysicalDeviceQueueIndices {
    pub(crate) fn from_device(info: &PhysicalDeviceInfo) -> Result<Self> {
        Ok(Self {
            graphics: Self::find_graphics_queue(info),
            compute: Self::find_compute_queue(info)?,
            transfer: Self::find_transfer_queue(info)?,
        })
    }

    pub(crate) fn as_queue_create_info(&self) -> Vec<vk::DeviceQueueCreateInfo> {
        let mut create_info = vec![
            // Graphics
            vk::DeviceQueueCreateInfo::builder()
                .queue_family_index(self.graphics)
                .queue_priorities(&QUEUE_PRIORITIES_ONE)
                .build(),
        ];

        // Compute
        if self.compute != self.graphics {
            create_info.push(
                vk::DeviceQueueCreateInfo::builder()
                    .queue_family_index(self.compute)
                    .queue_priorities(&QUEUE_PRIORITIES_ONE)
                    .build(),
            );
        }

        // Transfer
        if self.transfer != self.compute && self.transfer != self.graphics {
            create_info.push(
                vk::DeviceQueueCreateInfo::builder()
                    .queue_family_index(self.transfer)
                    .queue_priorities(&QUEUE_PRIORITIES_ONE)
                    .build(),
            );
        }

        create_info
    }

    fn find_graphics_queue(info: &PhysicalDeviceInfo) -> u32 {
        info.queue_families
            .iter()
            .enumerate()
            .find(|(_, queue)| {
                queue
                    .queue_family_properties
                    .queue_flags
                    .contains(vk::QueueFlags::GRAPHICS)
            })
            .map(|(i, _)| i)
            .unwrap() as _
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
    pub(crate) queues: [Option<Mutex<QueueWithPool>>; 3],
    pub(crate) graphics_index: usize,
    pub(crate) compute_index: usize,
    pub(crate) transfer_index: usize,
}

impl DeviceQueues {
    pub(crate) fn new(device: &ash::Device, indices: &PhysicalDeviceQueueIndices) -> Result<Self> {
        unsafe {
            let mut queues = [None, None, None];

            let graphics = 0;
            queues[graphics] = Some(Mutex::new(Self::retrieve_queue_and_pool(
                device,
                indices.graphics,
                Default::default(),
            )?));

            let compute = if indices.compute != indices.graphics {
                queues[1] = Some(Mutex::new(Self::retrieve_queue_and_pool(
                    device,
                    indices.compute,
                    Default::default(),
                )?));
                1
            } else {
                0
            };

            let transfer = if indices.transfer == indices.graphics {
                graphics
            } else if indices.transfer == indices.compute {
                compute
            } else {
                queues[2] = Some(Mutex::new(Self::retrieve_queue_and_pool(
                    device,
                    indices.transfer,
                    Default::default(),
                )?));
                2
            };

            Ok(Self {
                queues,
                graphics_index: graphics,
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

    pub(crate) fn graphics(&self) -> MutexGuard<QueueWithPool> {
        self.queues[self.graphics_index].as_ref().unwrap().lock()
    }

    pub(crate) fn compute(&self) -> MutexGuard<QueueWithPool> {
        self.queues[self.compute_index].as_ref().unwrap().lock()
    }

    pub(crate) fn transfer(&self) -> MutexGuard<QueueWithPool> {
        self.queues[self.transfer_index].as_ref().unwrap().lock()
    }
}
