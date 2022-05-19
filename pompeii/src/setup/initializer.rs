use std::{
    ffi::{CStr, CString},
    os::raw::c_char,
};

use ash::vk;
use raw_window_handle::HasRawWindowHandle;

use crate::{
    debug_utils::DebugUtils,
    errors::Result,
    setup::{
        builder::PompeiiBuilder,
        extensions::{REQUIRED_DEVICE_EXTENSIONS, REQUIRED_INSTANCE_EXTENSIONS},
    },
    swapchain::SurfaceWrapper,
};

pub(crate) const VULKAN_VERSION: u32 = vk::API_VERSION_1_3;

pub struct PompeiiInitializer {
    name: Option<CString>,
    ext_instance: Vec<*const c_char>,
    ext_device: Vec<*const c_char>,
}

impl Default for PompeiiInitializer {
    fn default() -> Self {
        Self {
            name: None,
            ext_instance: REQUIRED_INSTANCE_EXTENSIONS
                .iter()
                .map(|ext| ext.as_ptr())
                .collect(),
            ext_device: REQUIRED_DEVICE_EXTENSIONS
                .iter()
                .map(|ext| ext.as_ptr())
                .collect(),
        }
    }
}

impl PompeiiInitializer {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_name(mut self, name: &str) -> Self {
        self.name = Some(CString::new(name).unwrap());
        self
    }

    pub fn with_instance_extension(mut self, name: &'static CStr) -> Self {
        self.ext_instance.push(name.as_ptr());
        self
    }

    pub fn with_device_extension(mut self, name: &'static CStr) -> Self {
        self.ext_device.push(name.as_ptr());
        self
    }

    pub fn build(mut self, window: &impl HasRawWindowHandle) -> Result<PompeiiBuilder> {
        // Add window extension here
        self.ext_instance
            .extend(ash_window::enumerate_required_extensions(window)?);

        let entry = unsafe { ash::Entry::load()? };

        let instance = {
            let app_name = CString::new(
                self.name
                    .unwrap_or_else(|| CString::new("App Name").unwrap()),
            )
            .unwrap();
            let engine_name = CString::new(env!("CARGO_PKG_NAME")).unwrap();

            let vk_app_info = vk::ApplicationInfo::builder()
                .application_name(app_name.as_c_str())
                .application_version(vk::make_api_version(0, 1, 0, 0))
                .engine_name(engine_name.as_c_str())
                .engine_version({
                    let major = env!("CARGO_PKG_VERSION_MAJOR").parse().unwrap();
                    let minor = env!("CARGO_PKG_VERSION_MINOR").parse().unwrap();
                    let patch = env!("CARGO_PKG_VERSION_PATCH").parse().unwrap();
                    vk::make_api_version(0, major, minor, patch)
                })
                .api_version(VULKAN_VERSION);

            unsafe {
                entry.create_instance(
                    &vk::InstanceCreateInfo::builder()
                        .enabled_extension_names(&self.ext_instance)
                        .application_info(&vk_app_info),
                    None,
                )?
            }
        };

        let debug_utils = DebugUtils::new(&entry, &instance)?;

        let surface = unsafe {
            let ext = ash::extensions::khr::Surface::new(&entry, &instance);
            let handle = ash_window::create_surface(&entry, &instance, window, None)?;
            SurfaceWrapper { ext, handle }
        };

        Ok(PompeiiBuilder::new(
            entry,
            instance,
            debug_utils,
            self.ext_device,
            surface,
        ))
    }
}
