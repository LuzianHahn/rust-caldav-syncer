use clap::Parser;
use env_logger;
use log::{error, info};
use phone_sync::config::Config;
use phone_sync::sync::sync;

#[derive(Parser)]
#[command(name = "phone_sync")]
#[command(about = "Sync folders to WebDAV endpoint")]
struct Args {
    /// Path to config YAML file
    #[arg(short, long)]
    config: String,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

    let args = Args::parse();

    let config = Config::load(&args.config)?;
    info!("Loaded config from {}", args.config);

    if let Err(e) = sync(&config).await {
        error!("Sync failed: {}", e);
        std::process::exit(1);
    }

    info!("Sync completed successfully");
    Ok(())
}