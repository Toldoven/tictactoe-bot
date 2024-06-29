use color_eyre::Result;
use tiktaktoe::bot::bot_main;

#[tokio::main]
async fn main() -> Result<()> {
    _ = dotenvy::dotenv();
    color_eyre::install()?;
    pretty_env_logger::init();
    bot_main().await?;
    Ok(())
}
