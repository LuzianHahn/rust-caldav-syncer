use clap::{Parser, Subcommand};
use env_logger;
use log::{error, info};
use phone_sync::config::Config;
use phone_sync::hash_store::HashStore;
use phone_sync::sync::sync_with_progress;
use std::path::Path;
use walkdir::WalkDir;

#[derive(Parser)]
#[command(name = "my_binary")]
#[command(about = "Sync and hash utility")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Sync folders to WebDAV endpoint
    Sync {
        /// Path to config YAML file
        #[arg(short, long)]
        config: String,
        /// Show progress bar for missing files
        #[arg(short = 'p', long = "progress")]
        progress: bool,
    },
    /// Generate SHA‑256 hashes for all files under a directory and write them to a YAML file.
    Hash {
        /// Path to the directory whose files will be hashed
        #[arg(short, long)]
        target_dir: String,
        /// Optional path for the output hash store file (default: hashes.yaml)
        #[arg(short, long)]
        output: Option<String>,
    },
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

    let cli = Cli::parse();

    match cli.command {
        Commands::Sync { config, progress } => {
            let cfg = Config::load(&config)?;
            info!("Loaded config from {}", config);
            if let Err(e) = sync_with_progress(&cfg, progress).await {
                error!("Sync failed: {}", e);
                std::process::exit(1);
            }
            info!("Sync completed successfully");
        }
        Commands::Hash { target_dir, output } => {
            let target_path = Path::new(&target_dir);
            if !target_path.is_dir() {
                return Err(format!("Target path '{}' is not a directory", target_dir).into());
            }
            #[cfg(test)]
            mod tests {
                use super::*;
                use clap::Parser;
            
                #[test]
                fn test_cli_hash_parsing_with_output() {
                    let args = Cli::parse_from(&[
                        "my_binary",
                        "hash",
                        "-t",
                        "/tmp/target_dir",
                        "-o",
                        "custom_hashes.yaml",
                    ]);
                    match args.command {
                        Commands::Hash { target_dir, output } => {
                            assert_eq!(target_dir, "/tmp/target_dir");
                            assert_eq!(output.unwrap(), "custom_hashes.yaml");
                        }
                        _ => panic!("Expected Hash command"),
                    }
                }
            
                #[test]
                fn test_cli_hash_parsing_without_output() {
                    let args = Cli::parse_from(&["my_binary", "hash", "-t", "/tmp/target_dir"]);
                    match args.command {
                        Commands::Hash { target_dir, output } => {
                            assert_eq!(target_dir, "/tmp/target_dir");
                            assert!(output.is_none());
                        }
                        _ => panic!("Expected Hash command"),
                    }
                }
            }

            let mut store = HashStore::default();

            for entry in WalkDir::new(target_path)
                .into_iter()
                .filter_map(|e| e.ok())
                .filter(|e| e.file_type().is_file())
            {
                let file_path = entry.path();
                let hash = HashStore::compute_hash(file_path).await?;
                let rel_path = file_path
                    .strip_prefix(target_path)?
                    .to_string_lossy()
                    .to_string();
                store.hashes.insert(rel_path, hash);
            }

            let out_path = output.unwrap_or_else(|| "hashes.yaml".to_string());
            store.save(&out_path)?;
            println!("Hash store written to {}", out_path);
        }
    }

    Ok(())
}