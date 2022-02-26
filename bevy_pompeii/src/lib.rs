use std::cmp::Ordering;

use bevy_app::prelude::*;
use bevy_ecs::prelude::*;
use bevy_window::Windows;

use pompeii::setup::PompeiiBuilder;

#[derive(Clone, Hash, Debug, Eq, PartialEq, StageLabel)]
pub enum RenderStage {
    Render,
}

#[derive(Default)]
pub struct PompeiiPlugin;

impl Plugin for PompeiiPlugin {
    fn build(&self, app: &mut App) {
        // Renderer will be created in this setup system
        app.add_startup_system(setup_renderer_with_window);

        // Render systems
        app.schedule.add_stage(
            RenderStage::Render,
            SystemStage::parallel().with_system(render_system),
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
        .build()
        .expect("Failed to create pompeii renderer");

    commands.insert_resource(pompeii_app);
}

#[derive(Debug, Component)]
pub struct Rendererable;

fn render_system(query: Query<&Rendererable>) {
    for renderable in query.iter() {
        //println!("{renderable:?}");
    }
}
