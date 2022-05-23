//! Useful shortcut for simple actions
use ash::vk;

use crate::PompeiiRenderer;

impl PompeiiRenderer {
    pub(crate) unsafe fn get_buffer_address(&self, buffer: vk::Buffer) -> vk::DeviceAddress {
        self.device
            .get_buffer_device_address(&vk::BufferDeviceAddressInfo::builder().buffer(buffer))
    }
}
