use bevy_app::prelude::*;
use bevy_core::CorePlugin;
use bevy_ecs::prelude::*;
use bevy_window::WindowPlugin;
use bevy_winit::WinitPlugin;
use bevy_input::InputPlugin;
use pompeii::setup::PompeiiBuilder;
use std::cmp::Ordering;

#[derive(Clone, Hash, Debug, Eq, PartialEq, StageLabel)]
pub enum RenderStage {
    Render,
}

/// Defaults pompeii plugins, including [PompeiiPlugin], [WindowPlugin] and [WinitPlugin]
#[derive(Default)]
pub struct DefaultPompeiiPlugins;

impl PluginGroup for DefaultPompeiiPlugins {
    fn build(&mut self, group: &mut bevy_app::PluginGroupBuilder) {
        group
            .add(CorePlugin)
            .add(InputPlugin)
            .add(WindowPlugin::default())
            .add(WinitPlugin)
            .add(PompeiiPlugin);
    }
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

        /*let pompeii_app = builder
            .set_physical_device(gpu)
            .build()
            .expect("Failed to create pompeii renderer");*/

        // Insert app into world
        //app.insert_resource(pompeii_app);

        // Register systems
        app.add_startup_system(create_window_on_startup);
        app.schedule.add_stage(
            RenderStage::Render,
            SystemStage::parallel().with_system(render_system),
        );
    }
}

#[derive(Debug, Component)]
pub struct Rendererable;

fn create_window_on_startup() {
}

fn render_system(query: Query<&Rendererable>) {
    for renderable in query.iter() {
        println!("{renderable:?}");
    }
}
