use std::time::{Duration, Instant};

use bevy_app::prelude::*;
use bevy_ecs::prelude::*;
use log::info;

use pompeii::PompeiiRenderer;
use utils::FrameCounter;

use crate::swapchain_recreation as swapchain;
use crate::swapchain_recreation::RecreateSwapchainEvent;

pub mod gltf_loader;
pub(crate) mod setup;
pub(crate) mod swapchain_recreation;
pub(crate) mod utils;

#[derive(Clone, Hash, Debug, Eq, PartialEq, StageLabel)]
pub enum RenderStage {
    PreRender,
    Render,
}

#[derive(Default)]
pub struct PompeiiPlugin;

impl Plugin for PompeiiPlugin {
    fn build(&self, app: &mut App) {
        // Frame counter
        app.init_resource::<FrameCounter>();

        app.add_event::<RecreateSwapchainEvent>();

        // Renderer will be created in this setup system
        app.add_startup_system(setup::setup_renderer_with_window);

        // Render systems
        app.add_stage(
            RenderStage::PreRender,
            SystemStage::single_threaded()
                .with_system(swapchain::trigger_recreate_swapchain_system)
                .with_system(
                    swapchain::recreate_swapchain_system
                        .after(swapchain::trigger_recreate_swapchain_system),
                ),
        );
        app.add_stage_after(
            RenderStage::PreRender,
            RenderStage::Render,
            SystemStage::single_threaded().with_system_set(
                SystemSet::new()
                    .with_system(render_system)
                    .with_system(utils::frame_counter),
            ),
        );
    }
}

fn render_system(
    renderer: Res<PompeiiRenderer>,
    mut recreate_swapchain_events: EventWriter<RecreateSwapchainEvent>,
) {
    let recreate_swapchain = renderer.render_and_present().unwrap();
    if recreate_swapchain {
        recreate_swapchain_events.send(RecreateSwapchainEvent::default())
    }
}
