use bevy::{
    diagnostic::{FrameTimeDiagnosticsPlugin, LogDiagnosticsPlugin},
    prelude::*,
};

use bevy_pompeii::{
    acceleration_structure::{BlasAsset, BlasComponent, TlasAsset, TlasComponent},
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
    let blas: Handle<BlasAsset> = assets.load("BetterCube.glb#blas");
    let tlas: Handle<TlasAsset> = assets.load("BetterCube.glb#tlas");

    commands
        .spawn()
        .insert_bundle(MeshBundle::from(handle))
        .insert(BlasComponent { handle: blas })
        .insert(TlasComponent { handle: tlas });
}

fn remove_mesh(q: Query<Entity, With<MeshComponent>>, mut commands: Commands) {
    for m in q.iter() {
        commands.entity(m).remove::<MeshComponent>();
    }
}

// fn test(q: Query<GlobalTransform>) {
//     for t in q.iter() {
//         let t: GlobalTransform = t;
//         t.compute_affine();
//     }
// }
