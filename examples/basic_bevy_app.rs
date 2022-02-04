use bevy::prelude::*;
use bevy_pompeii::PompeiiPlugin;

fn main() -> anyhow::Result<()> {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugin(PompeiiPlugin)
        .run();

    Ok(())
}
