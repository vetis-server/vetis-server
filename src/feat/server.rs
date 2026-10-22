#[cfg(feature = "logging")]
use crate::logs;
use crate::{StartCommand, config::parse_file, run_in_background, run_in_foreground};
use log::{error, info};
use serde::Deserialize;
use std::error::Error;
use std::{fs::read_to_string, path::Path, time::Duration};
use vetis::VetisServer;
use vetis_tokio::{ServerConfig, Vetis};

#[derive(Deserialize)]
pub struct VetisConfig {
    pub(crate) worker_threads: usize,
    pub(crate) max_blocking_threads: usize,
    pub(crate) server: ServerConfig,
}

async fn run(server_config: ServerConfig, deamon: bool) -> Result<(), Box<dyn Error>> {
    let mut server = Vetis::from_config(server_config).await?;
    if let Err(e) = server.run().await {
        error!(target: "vetis", "Failed to start server: {}", e);
    }

    if deamon {
        run_in_background().await?;
    } else {
        run_in_foreground().await?
    }

    println!("");
    info!(target: "vetis", "Stopping server...");

    server
        .stop()
        .await?;

    // Give 1 ms for cleanup
    tokio::time::sleep(Duration::from_millis(1)).await;

    info!(target: "vetis", "Server stopped successfully!");

    Ok(())
}

pub(crate) fn start(cmd: StartCommand) -> Result<(), Box<dyn Error>> {
    if let Some(config) = cmd.config {
        if Path::exists(Path::new(&config)) {
            let file = read_to_string(&config);
            if let Ok(file) = file {
                let config = parse_file(&file);
                if let Ok(config) = config {
                    #[cfg(feature = "logging")]
                    logs::init_logs(&config);
                    #[cfg(feature = "metrics")]
                    metrics::init_metrics();

                    let rt = tokio::runtime::Builder::new_multi_thread()
                        .enable_all()
                        .worker_threads(config.worker_threads)
                        .max_blocking_threads(config.max_blocking_threads)
                        .build()?;
                    rt.block_on(async { run(config.server, cmd.deamon).await })?;
                } else {
                    eprintln!(
                        "Failed to start server: {}",
                        config
                            .err()
                            .unwrap()
                    );
                }
            } else {
                eprintln!("Failed to start server: {}", config);
            }
        } else {
            eprintln!("Failed to start server: Config file does not exist: {}", config);
        }
    } else {
        eprintln!("Failed to start server: No config file specified");
    }

    Ok(())
}

pub(crate) fn stop() -> Result<(), Box<dyn Error>> {
    Ok(())
}
