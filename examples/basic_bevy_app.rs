use bevy::{
    core::FixedTimestep,
    diagnostic::{FrameTimeDiagnosticsPlugin, LogDiagnosticsPlugin},
    prelude::*,
};

use bevy_pompeii::{
    mesh::{MeshAsset, MeshBundle, MeshComponent},
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
    /*app.add_system_set(
        SystemSet::new()
            .with_run_criteria(FixedTimestep::steps_per_second(1.0))
            .with_system(remove_mesh),
    );*/

    app.run();

    Ok(())
}

fn load_gltf(assets: Res<AssetServer>, mut commands: Commands) {
    let handle: Handle<MeshAsset> = assets.load("BetterCube.glb");

    commands.spawn().insert_bundle(MeshBundle::from(handle));
}

fn remove_mesh(q: Query<Entity, With<MeshComponent>>, mut commands: Commands) {
    for m in q.iter() {
        commands.entity(m).remove::<MeshComponent>();
    }
}
