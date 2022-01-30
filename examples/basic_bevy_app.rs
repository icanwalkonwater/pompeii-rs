use bevy_app::App;
use bevy_pompeii::PompeiiPlugin;
use simplelog::{ColorChoice, Config, LevelFilter, TermLogger, TerminalMode};

fn main() -> anyhow::Result<()> {
    TermLogger::init(
        LevelFilter::Debug,
        Config::default(),
        TerminalMode::Mixed,
        ColorChoice::Auto,
    )?;

    let mut app = App::new();

    app.add_plugin(PompeiiPlugin);

    Ok(())
}
