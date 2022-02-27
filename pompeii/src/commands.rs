use ash::vk;
use crate::{PompeiiRenderer, errors::Result};

impl PompeiiRenderer {
    #[inline]
    pub(crate) unsafe fn record_one_time_command_buffer(&self, pool: vk::CommandPool, actions: impl Fn(vk::CommandBuffer) -> Result<()>) -> Result<vk::CommandBuffer> {
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
}