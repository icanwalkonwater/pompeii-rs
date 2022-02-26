use std::ffi::CStr;

use ash::{vk, vk::QueueFamilyProperties2};
use log::warn;

use crate::{
    errors::Result,
    setup::{
        builder::PompeiiBuilder, initializer::VULKAN_VERSION,
        queues_finder::PhysicalDeviceQueueIndices,
    },
    swapchain::SurfaceWrapper,
};

#[derive(Debug)]
pub struct PhysicalDeviceInfo {
    pub(crate) handle: vk::PhysicalDevice,
    pub features: vk::PhysicalDeviceFeatures2,
    pub properties: vk::PhysicalDeviceProperties2,
    pub features_descriptor_indexing: vk::PhysicalDeviceDescriptorIndexingFeatures,
    pub properties_descriptor_indexing: vk::PhysicalDeviceDescriptorIndexingProperties,
    pub extensions: Vec<vk::ExtensionProperties>,
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

impl PompeiiBuilder {
    pub fn list_suitable_physical_devices(&mut self) -> Result<Vec<PhysicalDeviceInfo>> {
        let devices = unsafe { self.instance.enumerate_physical_devices()? };

        let devices = devices
            .into_iter()
            .map(|d| unsafe {
                // Query features
                let mut features_descriptor_indexing =
                    vk::PhysicalDeviceDescriptorIndexingFeatures::builder();
                let mut features = vk::PhysicalDeviceFeatures2::builder()
                    .push_next(&mut features_descriptor_indexing);
                self.instance
                    .get_physical_device_features2(d, &mut features);

                // Query properties
                let mut properties_descriptor_indexing =
                    vk::PhysicalDeviceDescriptorIndexingProperties::builder();
                let mut properties = vk::PhysicalDeviceProperties2::builder()
                    .push_next(&mut properties_descriptor_indexing);
                self.instance
                    .get_physical_device_properties2(d, &mut properties);

                // Query extensions
                let extensions = self
                    .instance
                    .enumerate_device_extension_properties(d)
                    .expect("Failed to enumerate device extensions");

                // Query queue information
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
                    features: features.build(),
                    properties: properties.build(),
                    features_descriptor_indexing: features_descriptor_indexing.build(),
                    properties_descriptor_indexing: properties_descriptor_indexing.build(),
                    extensions,
                    queue_families,
                    memory_properties,
                }
            })
            .filter(|info| {
                if !self.is_device_suitable(info, &self.surface).unwrap() {
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

    fn is_device_suitable(
        &self,
        info: &PhysicalDeviceInfo,
        surface: &SurfaceWrapper,
    ) -> Result<bool> {
        // Check Vulkan version
        if info.properties.properties.api_version < VULKAN_VERSION {
            return Ok(false);
        }

        // TODO: check required extensions here

        // Check queues
        if let Err(_) = PhysicalDeviceQueueIndices::from_device(info, surface) {
            return Ok(false);
        }

        Ok(true)
    }
}
