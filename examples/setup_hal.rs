use pompeii_hal::setup::{builder::PompeiiBuilder, initializer::PompeiiInitializer};
use pompeii_hal_backend_vulkan::setup::builder::PompeiiVulkanBuilder;
use simplelog::{ColorChoice, Config, LevelFilter, TermLogger, TerminalMode};

fn main() -> anyhow::Result<()> {
    TermLogger::init(
        LevelFilter::Debug,
        Config::default(),
        TerminalMode::Mixed,
        ColorChoice::Auto,
    )?;

    let mut builder = PompeiiVulkanBuilder::builder()
        .with_name("Test 1")
        .build()?;

    let gpus = builder.list_available_physical_devices()?;
    let gpu = gpus
        .into_iter()
        .filter(|gpu| gpu.is_discrete())
        .max_by_key(|gpu| gpu.vram_size())
        .unwrap();

    let _app = builder.set_physical_device(gpu).build()?;

    Ok(())
}
