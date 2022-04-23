mod bot;
mod ext;

use futures::{future, FutureExt, TryFutureExt};

pub const COLOR_GREEN: u32 = 0x73d216;
pub const COLOR_RED: u32 = 0xcc0000;

async fn load_config() -> anyhow::Result<bot::Config> {
    use tokio::fs;

    let config = fs::read("config.toml").await?;
    let config = toml::from_slice(&config)?;

    Ok(config)
}

fn main() -> anyhow::Result<()> {
    use std::env::var;
    use tokio::runtime::Builder;
    use tracing::Level;
    use tracing_subscriber::FmtSubscriber;

    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .finish();
    tracing::subscriber::set_global_default(subscriber).unwrap();

    let token = var("DISCORD_TOKEN").unwrap();

    let rt = Builder::new_current_thread().enable_all().build().unwrap();
    rt.block_on(load_config().and_then(|config| bot::run(config, &token).then(|_| future::ok(()))))
}
