use std::time::{Duration, Instant};

use bevy_app::prelude::*;
use bevy_ecs::prelude::*;
use log::info;

use pompeii::PompeiiRenderer;

use crate::setup::setup_renderer_with_window;
use crate::swapchain_recreation::{
    recreate_swapchain_system, trigger_recreate_swapchain_system, RecreateSwapchainEvent,
};

pub(crate) mod setup;
pub(crate) mod swapchain_recreation;
pub mod gltf_loader;

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
        app.insert_resource(FrameCounter {
            last_print: Instant::now(),
            frames: 0,
        });

        app.add_event::<RecreateSwapchainEvent>();

        // Renderer will be created in this setup system
        app.add_startup_system(setup_renderer_with_window);

        // Render systems
        app.add_stage(
            RenderStage::PreRender,
            SystemStage::single_threaded()
                .with_system(trigger_recreate_swapchain_system)
                .with_system(recreate_swapchain_system.after(trigger_recreate_swapchain_system)),
        );
        app.add_stage_after(
            RenderStage::PreRender,
            RenderStage::Render,
            SystemStage::single_threaded().with_system_set(
                SystemSet::new()
                    .with_system(render_system)
                    .with_system(frame_counter),
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

#[derive(Debug)]
struct FrameCounter {
    last_print: Instant,
    frames: usize,
}

fn frame_counter(mut frame_counter: ResMut<FrameCounter>) {
    frame_counter.frames += 1;

    let now = Instant::now();
    let delta = now.duration_since(frame_counter.last_print);
    if delta >= Duration::from_secs(1) {
        let fps = frame_counter.frames as f32 / delta.as_secs_f32();
        info!("FPS: {}", fps);
        frame_counter.last_print = now;
        frame_counter.frames = 0;
    }
}
