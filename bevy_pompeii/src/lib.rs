use bevy_app::{App, Plugin};
use bevy_ecs::prelude::*;
use pompeii::setup::PompeiiBuilder;

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

        let gpu = builder
            .list_available_physical_devices()
            .unwrap()
            .into_iter()
            .filter(|gpu| gpu.is_discrete())
            .max_by_key(|gpu| gpu.vram_size())
            .unwrap_or_else(|| {
                builder
                    .list_available_physical_devices()
                    .unwrap()
                    .into_iter()
                    .next()
                    .unwrap()
            });

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
