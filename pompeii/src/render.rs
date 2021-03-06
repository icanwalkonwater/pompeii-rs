use std::array::from_ref;

use ash::vk;
use log::{trace, warn};
use vk_sync_fork::{AccessType, ImageLayout};

use crate::{errors::Result, PompeiiRenderer};

impl PompeiiRenderer {
    unsafe fn record_render_commands(
        &self,
        pool: vk::CommandPool,
        swapchain_image_index: u32,
    ) -> Result<vk::CommandBuffer> {
        let swapchain = self.swapchain.read();

        self.record_one_time_command_buffer(pool, |cmd| {
            self.cmd_sync_image_barrier(
                cmd,
                &[AccessType::Present],
                &[AccessType::ColorAttachmentWrite],
                ImageLayout::Optimal,
                ImageLayout::Optimal,
                true,
                swapchain.images[swapchain_image_index as usize],
                vk::ImageAspectFlags::COLOR,
            );

            self.device.cmd_begin_rendering(
                cmd,
                &vk::RenderingInfoKHR::builder()
                    .render_area(vk::Rect2D::from(swapchain.extent))
                    .layer_count(1)
                    .view_mask(0)
                    .color_attachments(from_ref(
                        &vk::RenderingAttachmentInfoKHR::builder()
                            .image_view(swapchain.image_views[swapchain_image_index as usize])
                            .image_layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL)
                            .load_op(vk::AttachmentLoadOp::CLEAR)
                            .store_op(vk::AttachmentStoreOp::STORE)
                            .clear_value(vk::ClearValue {
                                color: vk::ClearColorValue {
                                    float32: [0.2, 0.2, 0.2, 0.0],
                                },
                            })
                            .build(),
                    )),
            );

            self.device.cmd_end_rendering(cmd);

            self.cmd_sync_image_barrier(
                cmd,
                &[AccessType::ColorAttachmentWrite],
                &[AccessType::Present],
                ImageLayout::Optimal,
                ImageLayout::Optimal,
                false,
                swapchain.images[swapchain_image_index as usize],
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

        let swapchain = self.swapchain.read();

        let (swapchain_image_index, is_suboptimal) = unsafe {
            swapchain.ext.acquire_next_image(
                swapchain.handle,
                u64::MAX,
                self.image_available_semaphore,
                vk::Fence::null(),
            )?
        };

        let graphics_queue = self.queues.graphics();
        let render_commands =
            unsafe { self.record_render_commands(graphics_queue.pool, swapchain_image_index)? };

        let wait_semaphores = [self.image_available_semaphore];
        let signal_semaphores = [self.render_finished_semaphore];

        unsafe {
            self.submit_to_queue_with_fence(
                graphics_queue.queue,
                render_commands,
                &[self.image_available_semaphore],
                &[vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT],
                &[self.render_finished_semaphore],
                self.in_flight_fence,
            )?;
        }

        trace!("[Render] Submitted graphics work");

        // Release the lock on the graphics queue
        drop(graphics_queue);

        unsafe {
            let present_queue = self.queues.present();
            let swapchains = [swapchain.handle];
            let res = swapchain.ext.queue_present(
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
