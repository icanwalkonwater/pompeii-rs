use ash::vk;
use log::debug;

use crate::{
    errors::{PompeiiError, Result},
    images::create_image_view_2d_basic,
    PompeiiRenderer,
};

#[derive(Debug, Clone)]
pub(crate) struct SurfaceCapabilities {
    pub capabilities: vk::SurfaceCapabilities2KHR,
    pub formats: Vec<vk::SurfaceFormat2KHR>,
    pub present_modes: Vec<vk::PresentModeKHR>,
}

/// Because of the `vk::SurfaceCapabilities2KHR` not being `Send`.
unsafe impl Send for SurfaceCapabilities {}
unsafe impl Sync for SurfaceCapabilities {}

pub(crate) struct SurfaceWrapper {
    pub(crate) ext: ash::extensions::khr::Surface,
    pub(crate) handle: vk::SurfaceKHR,
}

pub(crate) struct SwapchainWrapper {
    pub(crate) ext: ash::extensions::khr::Swapchain,
    pub(crate) handle: vk::SwapchainKHR,
    pub(crate) images: Vec<vk::Image>,
    pub(crate) image_views: Vec<vk::ImageView>,
    pub(crate) format: vk::Format,
    pub(crate) extent: vk::Extent2D,
}

impl SwapchainWrapper {
    pub(crate) unsafe fn cleanup(&mut self, device: &ash::Device, destroy_swapchain: bool) {
        for view in &self.image_views {
            device.destroy_image_view(*view, None);
        }

        if destroy_swapchain {
            self.ext.destroy_swapchain(self.handle, None);
        }
    }
}

impl PompeiiRenderer {
    pub(crate) fn create_swapchain(
        device: &ash::Device,
        ext: &ash::extensions::khr::Swapchain,
        info: &SurfaceCapabilities,
        surface: &SurfaceWrapper,
        window_size: (u32, u32),
        old_swapchain: Option<vk::SwapchainKHR>,
    ) -> Result<(
        vk::SwapchainKHR,
        Vec<vk::Image>,
        Vec<vk::ImageView>,
        vk::Format,
        vk::Extent2D,
    )> {
        debug!("Formats: {:?}", &info.formats);
        debug!("Present modes: {:?}", &info.present_modes);

        // Query various parameters
        let format = choose_swapchain_format(&info.formats)
            .ok_or(PompeiiError::NoCompatibleColorFormatFound)?;
        let present_mode = choose_swapchain_present_mode(&info.present_modes);
        let extent = choose_swapchain_extent(&info.capabilities, window_size);

        debug!("Swapchain format: {:?}", format.surface_format);
        debug!("Swapchain present mode: {:?}", present_mode);
        debug!("Swapchain extent: {:?}", extent);

        let capabilities = &info.capabilities.surface_capabilities;

        let image_count = if capabilities.max_image_count > 0 {
            (capabilities.min_image_count + 1).min(capabilities.max_image_count)
        } else {
            capabilities.min_image_count + 1
        };

        // Create the swapchain
        let swapchain = unsafe {
            ext.create_swapchain(
                &vk::SwapchainCreateInfoKHR::builder()
                    .surface(surface.handle)
                    .min_image_count(image_count)
                    .image_format(format.surface_format.format)
                    .image_color_space(format.surface_format.color_space)
                    .image_extent(extent)
                    .image_array_layers(1)
                    .image_usage(vk::ImageUsageFlags::COLOR_ATTACHMENT)
                    .image_sharing_mode(vk::SharingMode::EXCLUSIVE)
                    .pre_transform(capabilities.current_transform)
                    .composite_alpha(vk::CompositeAlphaFlagsKHR::OPAQUE)
                    .present_mode(present_mode)
                    .clipped(true)
                    .old_swapchain(old_swapchain.unwrap_or(vk::SwapchainKHR::null())),
                None,
            )?
        };

        // Get the images and create an image view for each of them
        let images = unsafe { ext.get_swapchain_images(swapchain) }?;
        let image_views = unsafe {
            images
                .iter()
                .copied()
                .map(|img| {
                    Ok(create_image_view_2d_basic(
                        device,
                        img,
                        format.surface_format.format,
                        vk::ImageAspectFlags::COLOR,
                    )?)
                })
                .collect::<Result<_>>()?
        };

        Ok((
            swapchain,
            images,
            image_views,
            format.surface_format.format,
            extent,
        ))
    }
    pub fn recreate_swapchain(&mut self, window_size: Option<(u32, u32)>) -> Result<()> {
        debug!("Recreating swapchain...");
        // Wait until nothing else is happening
        // TODO perf: maybe only wait for things that need the swapchain ?
        unsafe {
            self.device.device_wait_idle()?;
        }

        // Destroy resources from previous swapchain
        unsafe {
            self.swapchain.cleanup(&self.device, false);
        }

        let window_size =
            window_size.unwrap_or((self.swapchain.extent.width, self.swapchain.extent.height));

        // Query surface properties
        let (surface_capabilities, surface_formats, surface_present_modes) = unsafe {
            let ext =
                ash::extensions::khr::GetSurfaceCapabilities2::new(&self._entry, &self.instance);
            let surface_info =
                vk::PhysicalDeviceSurfaceInfo2KHR::builder().surface(self.surface.handle);
            let surface_capabilities = ext
                .get_physical_device_surface_capabilities2(self.physical_device, &surface_info)
                .unwrap();

            let mut surface_formats = vec![
                Default::default();
                ext.get_physical_device_surface_formats2_len(
                    self.physical_device,
                    &surface_info,
                )
                .unwrap()
            ];
            ext.get_physical_device_surface_formats2(
                self.physical_device,
                &surface_info,
                &mut surface_formats,
            )
            .unwrap();

            let surface_present_modes = self
                .surface
                .ext
                .get_physical_device_surface_present_modes(
                    self.physical_device,
                    self.surface.handle,
                )
                .unwrap();

            (surface_capabilities, surface_formats, surface_present_modes)
        };

        let (swapchain, images, image_views, format, extent) = Self::create_swapchain(
            &self.device,
            &self.swapchain.ext,
            &SurfaceCapabilities {
                capabilities: surface_capabilities,
                formats: surface_formats,
                present_modes: surface_present_modes,
            },
            &self.surface,
            window_size,
            Some(self.swapchain.handle),
        )?;

        self.swapchain.handle = swapchain;
        self.swapchain.images = images;
        self.swapchain.image_views = image_views;
        self.swapchain.format = format;
        self.swapchain.extent = extent;

        Ok(())
    }
}

const ACCEPTED_FORMATS: [vk::Format; 2] = [vk::Format::B8G8R8A8_SRGB, vk::Format::R8G8B8A8_SRGB];

const ACCEPTED_COLOR_SPACES: [vk::ColorSpaceKHR; 1] = [vk::ColorSpaceKHR::SRGB_NONLINEAR];

pub(crate) fn choose_swapchain_format(
    formats: &[vk::SurfaceFormat2KHR],
) -> Option<vk::SurfaceFormat2KHR> {
    for f @ vk::SurfaceFormat2KHR { surface_format, .. } in formats {
        if ACCEPTED_FORMATS.contains(&surface_format.format)
            && ACCEPTED_COLOR_SPACES.contains(&surface_format.color_space)
        {
            return Some(*f);
        }
    }

    None
}

const PREFERRED_PRESENT_MODE: vk::PresentModeKHR = vk::PresentModeKHR::MAILBOX;
const FALLBACK_PRESENT_MODE: vk::PresentModeKHR = vk::PresentModeKHR::FIFO;

/// Rule of thumb:
/// - [FIFO] Is VSYNC basically
/// - [MAILBOX] Is render as fast as you can baby
pub(crate) fn choose_swapchain_present_mode(
    present_modes: &[vk::PresentModeKHR],
) -> vk::PresentModeKHR {
    if present_modes.contains(&PREFERRED_PRESENT_MODE) {
        PREFERRED_PRESENT_MODE
    } else {
        FALLBACK_PRESENT_MODE
    }
}

pub(crate) fn choose_swapchain_extent(
    capabilities: &vk::SurfaceCapabilities2KHR,
    (width, height): (u32, u32),
) -> vk::Extent2D {
    let capabilities = capabilities.surface_capabilities;

    if capabilities.current_extent.width != u32::MAX {
        return capabilities.current_extent;
    }

    vk::Extent2D::builder()
        .width(width.clamp(
            capabilities.min_image_extent.width,
            capabilities.max_image_extent.width,
        ))
        .height(height.clamp(
            capabilities.min_image_extent.height,
            capabilities.max_image_extent.height,
        ))
        .build()
}
