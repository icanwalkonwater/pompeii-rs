use ash::vk;

use crate::{
    errors::{PompeiiError, Result},
    images::create_image_view_2d_basic,
    PhysicalDeviceInfo, PompeiiRenderer,
};

#[derive(Debug)]
pub(crate) struct SurfaceCapabilities {
    pub capabilities: vk::SurfaceCapabilities2KHR,
    pub formats: Vec<vk::SurfaceFormat2KHR>,
    pub present_modes: Vec<vk::PresentModeKHR>,
}

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
        // Query various parameters
        let format = choose_swapchain_format(&info.formats)
            .ok_or(PompeiiError::NoCompatibleColorFormatFound)?;
        let present_mode = choose_swapchain_present_mode(&info.present_modes);
        let extent = choose_swapchain_extent(&info.capabilities, window_size);

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
