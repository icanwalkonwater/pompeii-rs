use std::array::from_ref;

use ash::vk;

use crate::{errors::Result, PompeiiRenderer};

impl PompeiiRenderer {
    #[inline]
    pub(crate) unsafe fn record_one_time_command_buffer(
        &self,
        pool: vk::CommandPool,
        actions: impl FnOnce(vk::CommandBuffer) -> Result<()>,
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

        actions(cmd)?;

        self.device.end_command_buffer(cmd)?;
        Ok(cmd)
    }

    #[inline]
    pub(crate) unsafe fn submit_to_queue_with_fence(
        &self,
        queue: vk::Queue,
        cmds: vk::CommandBuffer,
        wait_semaphores: &[vk::Semaphore],
        wait_dst_stage: &[vk::PipelineStageFlags],
        signal_semaphore: &[vk::Semaphore],
        fence: vk::Fence,
    ) -> Result<()> {
        let cmds = [cmds];
        let submit_info = vk::SubmitInfo::builder()
            .command_buffers(&cmds)
            .wait_semaphores(wait_semaphores)
            .wait_dst_stage_mask(wait_dst_stage)
            .signal_semaphores(signal_semaphore);

        self.device
            .queue_submit(queue, from_ref(&submit_info.build()), fence)?;
        Ok(())
    }

    #[inline]
    pub(crate) unsafe fn submit_to_queue(
        &self,
        queue: vk::Queue,
        cmds: vk::CommandBuffer,
        wait_semaphores: &[vk::Semaphore],
        wait_dst_stage: &[vk::PipelineStageFlags],
        signal_semaphore: &[vk::Semaphore],
    ) -> Result<vk::Fence> {
        let fence = self
            .device
            .create_fence(&vk::FenceCreateInfo::default(), None)?;

        self.submit_to_queue_with_fence(
            queue,
            cmds,
            wait_semaphores,
            wait_dst_stage,
            signal_semaphore,
            fence,
        )?;

        Ok(fence)
    }

    #[inline]
    pub(crate) unsafe fn wait_fence(&self, fence: vk::Fence) -> Result<()> {
        self.device
            .wait_for_fences(from_ref(&fence), true, u64::MAX)?;
        self.device.destroy_fence(fence, None);
        Ok(())
    }

    #[inline]
    pub(crate) unsafe fn submit_and_wait(
        &self,
        queue: vk::Queue,
        cmds: vk::CommandBuffer,
        wait_semaphores: &[vk::Semaphore],
        wait_dst_stage: &[vk::PipelineStageFlags],
        signal_semaphore: &[vk::Semaphore],
    ) -> Result<()> {
        self.wait_fence(self.submit_to_queue(
            queue,
            cmds,
            wait_semaphores,
            wait_dst_stage,
            signal_semaphore,
        )?)
    }
}
