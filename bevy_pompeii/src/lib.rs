use std::{
    cmp::Ordering,
    time::{Duration, Instant},
};

use bevy_app::prelude::*;
use bevy_ecs::prelude::*;
use bevy_window::{WindowResized, Windows};
use log::{debug, info};

use pompeii::{setup::PompeiiBuilder, PompeiiRenderer};

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

fn setup_renderer_with_window(windows: Res<Windows>, mut commands: Commands) {
    // Get window
    let primary_window = windows
        .get_primary()
        .expect("A primary window need to exist for the renderer to finish its setup !");
    let handle = unsafe { primary_window.raw_window_handle().get_handle() };

    // Phase 1 builder, used to setup the vulkan instance
    let mut builder = PompeiiBuilder::builder()
        .with_name("Test 1")
        .build(&handle)
        .expect("Failed to create pompeii builder");

    // Pick a GPU
    let (_, gpu) = builder
        .list_suitable_physical_devices()
        .unwrap()
        .into_iter()
        .map(|gpu| (gpu.is_discrete(), gpu))
        .max_by(|a, b| match (a.0, b.0) {
            (true, true) | (false, false) => a.1.vram_size().cmp(&b.1.vram_size()),
            (true, _) => Ordering::Greater,
            _ => Ordering::Less,
        })
        .expect("No compatible GPU available !");

    // Phase 2 builder to build the real renderer
    let pompeii_app = builder
        .set_physical_device(gpu)
        .build((primary_window.width() as _, primary_window.height() as _))
        .expect("Failed to create pompeii renderer");

    commands.insert_resource(pompeii_app);
}

struct RecreateSwapchainEvent {
    window_size: Option<(u32, u32)>,
}

fn trigger_recreate_swapchain_system(
    mut resize: EventReader<WindowResized>,
    mut swapchain: EventWriter<RecreateSwapchainEvent>,
) {
    if let Some(&WindowResized { width, height, .. }) = resize.iter().last() {
        debug!("Trigger recreate swapchain");
        swapchain.send(RecreateSwapchainEvent {
            window_size: Some((width as _, height as _)),
        });
    }
}

fn recreate_swapchain_system(
    mut events: EventReader<RecreateSwapchainEvent>,
    mut renderer: ResMut<PompeiiRenderer>,
) {
    if let Some(&RecreateSwapchainEvent { window_size }) = events.iter().last() {
        renderer
            .recreate_swapchain(window_size)
            .expect("Failed to recreate swapchain");
    }
}

fn render_system(
    renderer: Res<PompeiiRenderer>,
    mut recreate_swapchain_events: EventWriter<RecreateSwapchainEvent>,
) {
    let recreate_swapchain = renderer.render_and_present().unwrap();
    if recreate_swapchain {
        recreate_swapchain_events.send(RecreateSwapchainEvent { window_size: None })
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
