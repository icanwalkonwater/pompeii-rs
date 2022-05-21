use bevy::{
    diagnostic::{FrameTimeDiagnosticsPlugin, LogDiagnosticsPlugin},
    prelude::*,
};

use bevy_pompeii::{
    mesh::{MeshAsset, MeshBundle},
    PompeiiPlugin,
};

fn main() -> anyhow::Result<()> {
    let mut app = App::new();

    app.add_plugins(DefaultPlugins)
        .add_plugin(PompeiiPlugin)
        .add_plugin(LogDiagnosticsPlugin::default())
        .add_plugin(FrameTimeDiagnosticsPlugin::default())
        .add_system(bevy::input::system::exit_on_esc_system);

    // bevy_mod_debugdump::print_schedule(&mut app);

    app.add_startup_system(load_gltf);

    app.run();

    Ok(())
}

fn load_gltf(assets: Res<AssetServer>, mut commands: Commands) {
    let handle: Handle<MeshAsset> = assets.load("BetterCube.glb");

    commands.spawn().insert_bundle(MeshBundle::from(handle));
}
