use std::array::from_ref;

use ash::vk;

use crate::{errors::Result, PompeiiRenderer};

impl PompeiiRenderer {
    unsafe fn record_render_commands(
        &self,
        pool: vk::CommandPool,
        swapchain_image_index: u32,
    ) -> Result<vk::CommandBuffer> {
        let cmd = self.device.allocate_command_buffers(
            &vk::CommandBufferAllocateInfo::builder()
                .command_pool(pool)
                .level(vk::CommandBufferLevel::PRIMARY)
                .command_buffer_count(1),
        )?[0];

        self.device.begin_command_buffer(
            cmd,
            &vk::CommandBufferBeginInfo::builder()
                .flags(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT),
        )?;

        self.device.cmd_pipeline_barrier(
            cmd,
            vk::PipelineStageFlags::TOP_OF_PIPE,
            vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT,
            vk::DependencyFlags::empty(),
            &[],
            &[],
            &[vk::ImageMemoryBarrier::builder()
                .dst_access_mask(vk::AccessFlags::COLOR_ATTACHMENT_WRITE)
                .old_layout(vk::ImageLayout::PRESENT_SRC_KHR)
                .new_layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL)
                .image(self.swapchain.images[swapchain_image_index as usize])
                .subresource_range(
                    vk::ImageSubresourceRange::builder()
                        .aspect_mask(vk::ImageAspectFlags::COLOR)
                        .base_mip_level(0)
                        .level_count(1)
                        .base_array_layer(0)
                        .layer_count(1)
                        .build(),
                )
                .build()],
        );

        self.ext_dynamic_rendering.cmd_begin_rendering(
            cmd,
            &vk::RenderingInfoKHR::builder()
                .render_area(vk::Rect2D::from(self.swapchain.extent))
                .layer_count(1)
                .view_mask(0)
                .color_attachments(from_ref(
                    &vk::RenderingAttachmentInfoKHR::builder()
                        .image_view(self.swapchain.image_views[swapchain_image_index as usize])
                        .image_layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL)
                        .load_op(vk::AttachmentLoadOp::CLEAR)
                        .store_op(vk::AttachmentStoreOp::STORE)
                        .clear_value(vk::ClearValue {
                            color: vk::ClearColorValue {
                                float32: [1.0, 1.0, 0.0, 0.0],
                            },
                        })
                        .build(),
                )),
        );

        self.ext_dynamic_rendering.cmd_end_rendering(cmd);

        self.device.cmd_pipeline_barrier(
            cmd,
            vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT,
            vk::PipelineStageFlags::BOTTOM_OF_PIPE,
            vk::DependencyFlags::empty(),
            &[],
            &[],
            &[vk::ImageMemoryBarrier::builder()
                .src_access_mask(vk::AccessFlags::COLOR_ATTACHMENT_WRITE)
                .old_layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL)
                .new_layout(vk::ImageLayout::PRESENT_SRC_KHR)
                .image(self.swapchain.images[swapchain_image_index as usize])
                .subresource_range(
                    vk::ImageSubresourceRange::builder()
                        .aspect_mask(vk::ImageAspectFlags::COLOR)
                        .base_mip_level(0)
                        .level_count(1)
                        .base_array_layer(0)
                        .layer_count(1)
                        .build(),
                )
                .build()],
        );

        self.device.end_command_buffer(cmd)?;

        Ok(cmd)
    }
    pub fn render(&self) -> Result<()> {
        // Wait for previous frame
        unsafe {
            self.device
                .wait_for_fences(&[self.in_flight_fence], true, u64::MAX)?;
            self.device.reset_fences(&[self.in_flight_fence])?;
        }

        let graphics_queue = self.queues.graphics();

        let (swapchain_image_index, _) = unsafe {
            self.swapchain.ext.acquire_next_image(
                self.swapchain.handle,
                u64::MAX,
                self.image_available_semaphore,
                vk::Fence::null(),
            )?
        };

        let render_commands =
            unsafe { self.record_render_commands(graphics_queue.pool, swapchain_image_index)? };

        let commands = [render_commands];
        let wait_semaphores = [self.image_available_semaphore];
        let signal_semaphores = [self.render_finished_semaphore];

        let submit_info = vk::SubmitInfo::builder()
            .wait_semaphores(&wait_semaphores)
            .wait_dst_stage_mask(&[vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT])
            .command_buffers(&commands)
            .signal_semaphores(&signal_semaphores);

        unsafe {
            self.device.queue_submit(
                graphics_queue.queue,
                from_ref(&submit_info.build()),
                self.in_flight_fence,
            )?;
        }
        std::mem::drop(graphics_queue);

        unsafe {
            let present_queue = self.queues.present();
            self.swapchain.ext.queue_present(
                present_queue.queue,
                &vk::PresentInfoKHR::builder()
                    .wait_semaphores(&[self.render_finished_semaphore])
                    .swapchains(&[self.swapchain.handle])
                    .image_indices(&[swapchain_image_index]),
            )?;
        }

        Ok(())
    }
}
