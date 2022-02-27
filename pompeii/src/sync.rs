use ash::vk;
use vk_sync_fork::{AccessType, ImageBarrier, ImageLayout};

use crate::PompeiiRenderer;

impl PompeiiRenderer {
    pub(crate) unsafe fn cmd_sync_image_barrier(
        &self,
        command_buffer: vk::CommandBuffer,
        previous_accesses: &[AccessType],
        next_accesses: &[AccessType],
        previous_layout: ImageLayout,
        next_layout: ImageLayout,
        discard_contents: bool,
        image: vk::Image,
        aspect: vk::ImageAspectFlags,
    ) {
        let (src_stages, dst_stages, barrier) =
            vk_sync_fork::get_image_memory_barrier(&ImageBarrier {
                previous_accesses,
                next_accesses,
                previous_layout,
                next_layout,
                discard_contents,
                src_queue_family_index: vk::QUEUE_FAMILY_IGNORED,
                dst_queue_family_index: vk::QUEUE_FAMILY_IGNORED,
                image,
                range: vk::ImageSubresourceRange::builder()
                    .aspect_mask(aspect)
                    .base_mip_level(0)
                    .level_count(1)
                    .base_array_layer(0)
                    .layer_count(1)
                    .build(),
            });

        self.device.cmd_pipeline_barrier(
            command_buffer,
            src_stages,
            dst_stages,
            vk::DependencyFlags::empty(),
            &[],
            &[],
            &[barrier],
        );
    }
}
