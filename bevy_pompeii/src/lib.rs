use bevy_app::prelude::*;
use bevy_core::CorePlugin;
use bevy_ecs::prelude::*;
use bevy_window::{CreateWindow, WindowPlugin, Windows};
use log::{debug, error, info};
use pompeii::setup::PompeiiBuilder;
use pompeii::PompeiiRenderer;
use std::cmp::Ordering;

#[derive(Clone, Hash, Debug, Eq, PartialEq, StageLabel)]
pub enum RenderStage {
    Render,
}

#[derive(Default)]
pub struct PompeiiPlugin;

impl Plugin for PompeiiPlugin {
    fn build(&self, app: &mut App) {
        let mut builder = PompeiiBuilder::builder()
            .with_name("Test 1")
            .build()
            .expect("Failed to create pompeii builder");

        let (_, gpu) = builder
            .list_suitable_physical_devices(todo!())
            .unwrap()
            .into_iter()
            .map(|gpu| (gpu.is_discrete(), gpu))
            .max_by(|a, b| match (a.0, b.0) {
                (true, true) | (false, false) => a.1.vram_size().cmp(&b.1.vram_size()),
                (true, _) => Ordering::Greater,
                _ => Ordering::Less,
            })
            .expect("No compatible GPU available !");

        let pompeii_app = builder
            .set_physical_device(gpu)
            .build()
            .expect("Failed to create pompeii renderer");

        // Insert app into world
        app.insert_non_send_resource(pompeii_app);

        // Register systems
        app.add_startup_system(attach_to_window);
        app.schedule.add_stage(
            RenderStage::Render,
            SystemStage::parallel().with_system(render_system),
        );
    }
}

fn attach_to_window(windows: Res<Windows>, renderer: Res<PompeiiRenderer>) {
    let primary_window = windows.get_primary().expect("A primary window need to exist for the renderer to finish its setup !");
    let handle = primary_window.raw_window_handle();

    println!("Window is {:?}", primary_window);
}

#[derive(Debug, Component)]
pub struct Rendererable;

fn render_system(query: Query<&Rendererable>) {
    for renderable in query.iter() {
        //println!("{renderable:?}");
    }
}
