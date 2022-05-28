use std::ffi::CStr;

use ash::{
    extensions::{ext, khr},
    vk,
};
// TODO: use the std one when it is stabilized
use once_cell::sync::Lazy;

use crate::PhysicalDeviceInfo;

pub(crate) static REQUIRED_INSTANCE_EXTENSIONS: Lazy<[&CStr; 2]> = Lazy::new(|| {
    [
        ext::DebugUtils::name(),
        khr::GetSurfaceCapabilities2::name(),
    ]
});

pub(crate) static REQUIRED_DEVICE_EXTENSIONS: Lazy<[&CStr; 4]> = Lazy::new(|| {
    [
        khr::Swapchain::name(),
        khr::DeferredHostOperations::name(),
        khr::AccelerationStructure::name(),
        khr::RayTracingPipeline::name(),
    ]
});

pub(crate) static REQUIRED_FEATURES_CHECK: [(&str, fn(&PhysicalDeviceInfo) -> bool); 6] = [
    ("Descriptor Indexing", |info| {
        info.features_vk12.descriptor_indexing != 0
    }),
    ("Buffer Device Address", |info| {
        info.features_vk12.buffer_device_address != 0
    }),
    ("Synchronization 2", |info| {
        info.features_vk13.synchronization2 != 0
    }),
    ("Dynamic Rendering", |info| {
        info.features_vk13.dynamic_rendering != 0
    }),
    ("Acceleration Structure", |info| {
        info.features_acceleration_structure.acceleration_structure != 0
    }),
    ("RayTracing Pipeline", |info| {
        info.features_ray_tracing_pipeline.ray_tracing_pipeline != 0
    }),
];

pub(crate) fn get_required_features() -> (
    impl vk::ExtendsDeviceCreateInfo,
    impl vk::ExtendsDeviceCreateInfo,
    impl vk::ExtendsDeviceCreateInfo,
    impl vk::ExtendsDeviceCreateInfo,
) {
    (
        vk::PhysicalDeviceVulkan12Features::builder()
            .descriptor_indexing(true)
            .buffer_device_address(true)
            .build(),
        vk::PhysicalDeviceVulkan13Features::builder()
            .synchronization2(true)
            .dynamic_rendering(true)
            .build(),
        vk::PhysicalDeviceAccelerationStructureFeaturesKHR::builder()
            .acceleration_structure(true)
            .build(),
        vk::PhysicalDeviceRayTracingPipelineFeaturesKHR::builder()
            .ray_tracing_pipeline(true)
            .build(),
    )
}

// pub(crate) fn get_required_features() -> [Box<dyn vk::ExtendsDeviceCreateInfo>; 4] {
//     [
//         Box::new(vk::PhysicalDeviceVulkan12Features::builder()
//             .descriptor_indexing(true)
//             .buffer_device_address(true)
//             .build()),
//         Box::new(vk::PhysicalDeviceVulkan13Features::builder()
//             .synchronization2(true)
//             .dynamic_rendering(true)
//             .build()),
//         Box::new(vk::PhysicalDeviceAccelerationStructureFeaturesKHR::builder()
//             .acceleration_structure(true)
//             .build()),
//         Box::new(vk::PhysicalDeviceRayTracingPipelineFeaturesKHR::builder()
//             .ray_tracing_pipeline(true)
//             .build()),
//     ]
// }
