use bevy::diagnostic::{FrameTimeDiagnosticsPlugin, LogDiagnosticsPlugin};
use bevy::prelude::*;
use bevy_pompeii::pompeii::PompeiiRenderer;
use bevy_pompeii::PompeiiPlugin;

fn main() -> anyhow::Result<()> {
    let mut app = App::new();

    app
        .add_plugins(DefaultPlugins)
        .add_plugin(PompeiiPlugin)
        .add_plugin(LogDiagnosticsPlugin::default())
        .add_plugin(FrameTimeDiagnosticsPlugin::default())
        .add_system(bevy::input::system::exit_on_esc_system);

    // bevy_mod_debugdump::print_schedule(&mut app);

    app.add_startup_system(load_gltf);

    app.run();

    Ok(())
}

fn load_gltf(mut renderer: NonSendMut<PompeiiRenderer>) {
    bevy_pompeii::gltf_loader::load_gltf_models(&mut renderer, "./assets/BetterCube.glb").unwrap();
}
