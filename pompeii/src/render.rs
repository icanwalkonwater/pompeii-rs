use std::array::from_ref;

use ash::vk;
use log::{debug, trace, warn};
use vk_sync_fork::{AccessType, ImageLayout};

use crate::{errors::Result, PompeiiRenderer};

impl PompeiiRenderer {
    unsafe fn record_render_commands(
        &self,
        pool: vk::CommandPool,
        swapchain_image_index: u32,
    ) -> Result<vk::CommandBuffer> {
        self.record_one_time_command_buffer(pool, |cmd| {
            self.cmd_sync_image_barrier(
                cmd,
                &[AccessType::Present],
                &[AccessType::ColorAttachmentWrite],
                ImageLayout::Optimal,
                ImageLayout::Optimal,
                true,
                self.swapchain.images[swapchain_image_index as usize],
                vk::ImageAspectFlags::COLOR,
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
                                    float32: [0.0, 1.0, 0.0, 0.0],
                                },
                            })
                            .build(),
                    )),
            );

            self.ext_dynamic_rendering.cmd_end_rendering(cmd);

            self.cmd_sync_image_barrier(
                cmd,
                &[AccessType::ColorAttachmentWrite],
                &[AccessType::Present],
                ImageLayout::Optimal,
                ImageLayout::Optimal,
                false,
                self.swapchain.images[swapchain_image_index as usize],
                vk::ImageAspectFlags::COLOR,
            );

            Ok(())
        })
    }

    /// Return whether or not to recreate the swapchain before next round
    pub fn render_and_present(&self) -> Result<bool> {
        // Wait for previous frame
        unsafe {
            self.device
                .wait_for_fences(&[self.in_flight_fence], true, u64::MAX)?;
            self.device.reset_fences(&[self.in_flight_fence])?;
        }

        trace!("[Render] Start commands");

        let (swapchain_image_index, is_suboptimal) = unsafe {
            self.swapchain.ext.acquire_next_image(
                self.swapchain.handle,
                u64::MAX,
                self.image_available_semaphore,
                vk::Fence::null(),
            )?
        };

        let graphics_queue = self.queues.graphics();
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

        trace!("[Render] Submitted graphics work");

        // Release the lock on the graphics queue
        std::mem::drop(graphics_queue);

        unsafe {
            let present_queue = self.queues.present();
            let swapchains = [self.swapchain.handle];
            let res = self.swapchain.ext.queue_present(
                present_queue.queue,
                &vk::PresentInfoKHR::builder()
                    .wait_semaphores(&signal_semaphores)
                    .swapchains(&swapchains)
                    .image_indices(&[swapchain_image_index]),
            );

            if let Err(vk::Result::ERROR_OUT_OF_DATE_KHR) = res {
                warn!("Swapchain out of date, skipping this presentation");
                return Ok(true);
            };

            res?;
        }

        trace!("[Render] Submitted present work");

        Ok(is_suboptimal)
    }
}
