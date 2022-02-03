use bevy::prelude::*;
use bevy_pompeii::{DefaultPompeiiPlugins};
use simplelog::{ColorChoice, Config, LevelFilter, TermLogger, TerminalMode};

fn main() -> anyhow::Result<()> {
    TermLogger::init(
        LevelFilter::Debug,
        Config::default(),
        TerminalMode::Mixed,
        ColorChoice::Auto,
    )?;

    App::new()
        .add_plugins(MinimalPlugins)
        .add_plugins(DefaultPompeiiPlugins)
        .run();

    Ok(())
}
