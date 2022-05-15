use bevy::prelude::*;
use bevy_pompeii::PompeiiPlugin;

fn main() -> anyhow::Result<()> {
    let mut app = App::new();

    app.add_plugins(DefaultPlugins).add_plugin(PompeiiPlugin);

    // bevy_mod_debugdump::print_schedule(&mut app);

    app.run();

    Ok(())
}
