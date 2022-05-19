use std::{ffi::CStr, os::raw::c_char};

use ash::vk;
use log::{debug, info, warn};

use crate::{
    errors::Result,
    setup::{
        builder::PompeiiBuilder, initializer::VULKAN_VERSION,
        queues_finder::PhysicalDeviceQueueIndices,
    },
    swapchain::{SurfaceCapabilities, SurfaceWrapper},
};

#[derive(Debug)]
pub struct PhysicalDeviceInfo {
    pub(crate) handle: vk::PhysicalDevice,
    pub features: vk::PhysicalDeviceFeatures2,
    pub properties: vk::PhysicalDeviceProperties2,
    pub features_descriptor_indexing: vk::PhysicalDeviceDescriptorIndexingFeatures,
    pub properties_descriptor_indexing: vk::PhysicalDeviceDescriptorIndexingProperties,
    pub extensions: Vec<vk::ExtensionProperties>,
    pub queue_families: Vec<vk::QueueFamilyProperties2>,
    pub memory_properties: vk::PhysicalDeviceMemoryProperties2,
    pub(crate) surface_capabilities: Option<SurfaceCapabilities>,
}

impl PhysicalDeviceInfo {
    pub fn name(&self) -> &str {
        unsafe {
            CStr::from_ptr(self.properties.properties.device_name.as_ptr())
                .to_str()
                .unwrap()
        }
    }

    pub fn vulkan_version(&self) -> String {
        let version = self.properties.properties.api_version;
        // Ignore variant, it doesn't really matter that much
        format!(
            "{}.{}.{}",
            vk::api_version_major(version),
            vk::api_version_minor(version),
            vk::api_version_patch(version),
        )
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
                let mut queue_families =
                    vec![
                        Default::default();
                        self.instance
                            .get_physical_device_queue_family_properties2_len(d)
                    ];
                self.instance
                    .get_physical_device_queue_family_properties2(d, &mut queue_families);

                // Query memory properties
                let mut memory_properties = vk::PhysicalDeviceMemoryProperties2::default();
                self.instance
                    .get_physical_device_memory_properties2(d, &mut memory_properties);

                // Query surface properties
                let surface_capabilities = if self.is_surface_supported(d, &queue_families) {
                    let surface_info =
                        vk::PhysicalDeviceSurfaceInfo2KHR::builder().surface(self.surface.handle);
                    let capabilities = self
                        .ext_surface_capabilities2
                        .get_physical_device_surface_capabilities2(d, &surface_info)
                        .unwrap();

                    let mut formats = vec![
                        Default::default();
                        self.ext_surface_capabilities2
                            .get_physical_device_surface_formats2_len(d, &surface_info)
                            .unwrap()
                    ];
                    self.ext_surface_capabilities2
                        .get_physical_device_surface_formats2(d, &surface_info, &mut formats)
                        .unwrap();

                    let present_modes = self
                        .surface
                        .ext
                        .get_physical_device_surface_present_modes(d, self.surface.handle)
                        .unwrap();

                    Some(SurfaceCapabilities {
                        capabilities,
                        formats,
                        present_modes,
                    })
                } else {
                    None
                };

                PhysicalDeviceInfo {
                    handle: d,
                    features: features.build(),
                    properties: properties.build(),
                    features_descriptor_indexing: features_descriptor_indexing.build(),
                    properties_descriptor_indexing: properties_descriptor_indexing.build(),
                    extensions,
                    queue_families,
                    memory_properties,
                    surface_capabilities,
                }
            })
            .filter(|info| {
                if !self
                    .is_device_suitable(info, &self.surface, &self.device_extensions)
                    .unwrap()
                {
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

    fn is_surface_supported(
        &self,
        device: vk::PhysicalDevice,
        queues: &[vk::QueueFamilyProperties2],
    ) -> bool {
        queues.iter().enumerate().any(|(i, _)| unsafe {
            self.surface
                .ext
                .get_physical_device_surface_support(device, i as _, self.surface.handle)
                .unwrap()
        })
    }

    fn is_device_suitable(
        &self,
        info: &PhysicalDeviceInfo,
        surface: &SurfaceWrapper,
        device_extensions: &[*const c_char],
    ) -> Result<bool> {
        if info.surface_capabilities.is_none() {
            warn!("[{}] [KO] No support for the surface", info.name());
            return Ok(false);
        }

        let surface_capabilities = info.surface_capabilities.as_ref().unwrap();

        // Check Vulkan version
        if info.properties.properties.api_version < VULKAN_VERSION {
            warn!(
                "[{}] [KO] No support for the required vulkan version",
                info.name()
            );
            return Ok(false);
        }
        debug!(
            "[{}] [OK] Recent enough vulkan version found ({})",
            info.name(),
            info.vulkan_version()
        );

        // Check extensions
        let supports_all_extensions = device_extensions.iter().all(|ext| unsafe {
            let ext = CStr::from_ptr(*ext);
            let found = info
                .extensions
                .iter()
                .any(|device_ext| CStr::from_ptr(device_ext.extension_name.as_ptr()) == ext);
            if found {
                debug!("[{}] [OK] Found extension {:?}", info.name(), ext);
            } else {
                warn!("[{}] [KO] Failed to find extension {:?}", info.name(), ext);
            }
            found
        });

        if !supports_all_extensions {
            warn!("[{}] [KO] Not all extensions are supported !", info.name());
            return Ok(false);
        }

        // Check queues
        if PhysicalDeviceQueueIndices::from_device(info, surface).is_err() {
            warn!("[{}] [KO] Missing queues", info.name());
            return Ok(false);
        }

        // Check swapchain support
        if surface_capabilities.formats.is_empty() || surface_capabilities.present_modes.is_empty()
        {
            warn!(
                "[{}] [KO] Swapchain support incomplete (no formats or present modes detected)",
                info.name()
            );
            return Ok(false);
        }

        info!("[{}] [OK] Physical device is suitable !", info.name());

        Ok(true)
    }
}
