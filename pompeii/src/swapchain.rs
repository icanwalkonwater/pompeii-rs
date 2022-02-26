use ash::vk;

pub(crate) struct SurfaceWrapper {
    pub(crate) ext: ash::extensions::khr::Surface,
    pub(crate) handle: vk::SurfaceKHR,
}
