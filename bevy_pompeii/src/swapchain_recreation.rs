use bevy_ecs::prelude::*;
use bevy_window::WindowResized;
use log::debug;

use pompeii::PompeiiRenderer;

#[derive(Default)]
pub(crate) struct RecreateSwapchainEvent {
    pub window_size: Option<(u32, u32)>,
}

impl From<(u32, u32)> for RecreateSwapchainEvent {
    fn from(window_size: (u32, u32)) -> Self {
        Self {
            window_size: Some(window_size),
        }
    }
}

pub(crate) fn trigger_recreate_swapchain_system(
    mut resize: EventReader<WindowResized>,
    mut swapchain: EventWriter<RecreateSwapchainEvent>,
) {
    if let Some(&WindowResized { width, height, .. }) = resize.iter().last() {
        debug!("Trigger recreate swapchain");
        swapchain.send(RecreateSwapchainEvent::from((width as _, height as _)));
    }
}

pub(crate) fn recreate_swapchain_system(
    mut events: EventReader<RecreateSwapchainEvent>,
    mut renderer: NonSendMut<PompeiiRenderer>,
) {
    if let Some(&RecreateSwapchainEvent { window_size }) = events.iter().last() {
        renderer
            .recreate_swapchain(window_size)
            .expect("Failed to recreate swapchain");
    }
}
