use bevy::{
    diagnostic::{FrameTimeDiagnosticsPlugin, LogDiagnosticsPlugin},
    prelude::*,
};
use bevy_pompeii::PompeiiPlugin;

fn main() -> anyhow::Result<()> {
    let mut app = App::new();

    app.add_plugins(DefaultPlugins)
        .add_plugin(PompeiiPlugin)
        .add_plugin(LogDiagnosticsPlugin::default())
        .add_plugin(FrameTimeDiagnosticsPlugin::default())
        .add_system(bevy::input::system::exit_on_esc_system);

    // bevy_mod_debugdump::print_schedule(&mut app);

    app.add_startup_system(load_gltf.exclusive_system().at_end());

    app.run();

    Ok(())
}

fn load_gltf(world: &mut World) {
    bevy_pompeii::gltf_loader::load_gltf_models(world, "./assets/BetterCube.glb").unwrap();
}
