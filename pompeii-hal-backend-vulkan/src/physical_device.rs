use crate::{
    builder::PompeiiVulkanBuilder,
    errors::{Result, VkErrorExt},
    initializer::VULKAN_VERSION,
    queues::VulkanPhysicalDeviceQueueIndices,
};
use ash::{vk, vk::QueueFamilyProperties2};
use log::warn;
use std::ffi::CStr;

#[derive(Debug)]
pub struct PhysicalDeviceInfo {
    pub(crate) handle: vk::PhysicalDevice,
    pub properties: vk::PhysicalDeviceProperties2,
    pub extensions: Vec<vk::ExtensionProperties>,
    pub features: vk::PhysicalDeviceFeatures2,
    pub queue_families: Vec<QueueFamilyProperties2>,
    pub memory_properties: vk::PhysicalDeviceMemoryProperties2,
}

impl PhysicalDeviceInfo {
    pub fn name(&self) -> &str {
        unsafe {
            CStr::from_ptr(self.properties.properties.device_name.as_ptr())
                .to_str()
                .unwrap()
        }
    }

    pub fn is_discrete(&self) -> bool {
        self.properties.properties.device_type == vk::PhysicalDeviceType::DISCRETE_GPU
    }

    pub fn vram_size(&self) -> vk::DeviceSize {
        self.memory_properties
            .memory_properties
            .memory_heaps
            .iter()
            .filter(|heap| heap.flags.contains(vk::MemoryHeapFlags::DEVICE_LOCAL))
            .map(|heap| heap.size)
            .sum()
    }
}

impl PompeiiVulkanBuilder {
    pub fn list_available_physical_devices(&mut self) -> Result<Vec<PhysicalDeviceInfo>> {
        let devices = unsafe {
            self.instance
                .enumerate_physical_devices()
                .map_err_pompeii()?
        };

        let devices = devices
            .into_iter()
            .map(|d| unsafe {
                let mut properties = vk::PhysicalDeviceProperties2::default();
                self.instance
                    .get_physical_device_properties2(d, &mut properties);

                let extensions = self
                    .instance
                    .enumerate_device_extension_properties(d)
                    .expect("Failed to enumerate device extensions");

                let mut features = vk::PhysicalDeviceFeatures2::default();
                self.instance
                    .get_physical_device_features2(d, &mut features);

                let mut queue_families = Vec::new();
                queue_families.resize_with(
                    self.instance
                        .get_physical_device_queue_family_properties2_len(d),
                    || vk::QueueFamilyProperties2::default(),
                );
                self.instance
                    .get_physical_device_queue_family_properties2(d, &mut queue_families);

                let mut memory_properties = vk::PhysicalDeviceMemoryProperties2::default();
                self.instance
                    .get_physical_device_memory_properties2(d, &mut memory_properties);

                PhysicalDeviceInfo {
                    handle: d,
                    properties,
                    extensions,
                    features,
                    queue_families,
                    memory_properties,
                }
            })
            .filter(|info| {
                if !self.is_device_suitable(info) {
                    let name =
                        unsafe { CStr::from_ptr(info.properties.properties.device_name.as_ptr()) }
                            .to_str()
                            .unwrap();
                    warn!("Found unsuitable device: {}", name);
                    false
                } else {
                    true
                }
            })
            .collect();

        Ok(devices)
    }

    fn is_device_suitable(&self, info: &PhysicalDeviceInfo) -> bool {
        // Check Vulkan version
        if info.properties.properties.api_version < VULKAN_VERSION {
            return false;
        }

        // Check required extensions
        // TODO: no extensions yet

        // Check queues
        if let Err(_) = VulkanPhysicalDeviceQueueIndices::from_device(info) {
            return false;
        }

        true
    }
}
