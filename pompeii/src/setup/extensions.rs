use std::ffi::CStr;

use ash::extensions::{ext, khr};
// TODO: use the std one when it is stabilized
use once_cell::sync::Lazy;

pub(crate) static REQUIRED_INSTANCE_EXTENSIONS: Lazy<[&CStr; 2]> = Lazy::new(|| {
    [
        ext::DebugUtils::name(),
        khr::GetSurfaceCapabilities2::name(),
    ]
});

pub(crate) static REQUIRED_DEVICE_EXTENSIONS: Lazy<[&CStr; 2]> =
    Lazy::new(|| [khr::Swapchain::name(), khr::DynamicRendering::name()]);
