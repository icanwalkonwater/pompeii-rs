use bevy_app::{App, Plugin};
use bevy_ecs::prelude::*;
use pompeii::setup::PompeiiBuilder;
use std::cmp::Ordering;

#[derive(Clone, Hash, Debug, Eq, PartialEq, StageLabel)]
pub enum RenderStage {
    Render,
}

pub struct PompeiiPlugin;

impl Plugin for PompeiiPlugin {
    fn build(&self, app: &mut App) {
        let mut builder = PompeiiBuilder::builder()
            .with_name("Test 1")
            .build()
            .expect("Failed to create pompeii builder");

        let (_, gpu) = builder
            .list_available_physical_devices()
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

        // Register systems
        app.schedule.add_stage(
            RenderStage::Render,
            SystemStage::parallel().with_system(render_system),
        );
    }
}

#[derive(Debug, Component)]
pub struct Rendererable;

pub fn render_system(query: Query<&Rendererable>) {
    println!("Hey");
    for renderable in query.iter() {
        println!("{renderable:?}");
    }
}
