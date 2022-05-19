use std::cmp::Ordering;

use bevy_ecs::prelude::*;
use bevy_window::Windows;

use pompeii::setup::PompeiiBuilder;

pub(crate) fn setup_renderer_with_window(world: &mut World) {
    let windows = world.get_resource::<Windows>().unwrap();

    // Get window
    let primary_window = windows
        .get_primary()
        .expect("A primary window need to exist for the renderer to finish its setup !");
    let handle = unsafe { primary_window.raw_window_handle().get_handle() };

    // Phase 1 builder, used to setup the vulkan instance
    let mut builder = PompeiiBuilder::builder()
        .with_name("Pompeii renderer")
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

    world.insert_non_send_resource(pompeii_app);
}
