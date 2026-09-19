use clap::Parser;
use log::error;
use mimalloc::MiMalloc;
use serde::Deserialize;
use time::{Timestamp, macros::format_description};
use std::{error::Error, fs::read_to_string, io::Write, path::Path};
use vetis::{VetisServer as _, server::ServerConfig};
use vetis_tokio::Vetis;

#[cfg(feature = "flash")]
#[allow(unused_imports)]
use vetis_flash::FlashPathConfig;

#[cfg(feature = "proxy")]
#[allow(unused_imports)]
use vetis_proxy::ProxyPathConfig;

#[cfg(feature = "rev-proxy")]
#[allow(unused_imports)]
use vetis_rev_proxy::ReverseProxyPathConfig;

#[cfg(feature = "static")]
#[allow(unused_imports)]
use vetis_static::StaticPathConfig;

#[global_allocator]
static GLOBAL: MiMalloc = MiMalloc;

#[derive(Deserialize)]
pub struct VetisConfig {
    worker_threads: usize,
    max_blocking_threads: usize,
    server: ServerConfig,
}

#[derive(Parser)]
#[command(
    name = "vetis-server",
    about = "vetis - a very tiny server",
    long_about = r#"
vetis - a very tiny server

Usage:
    vetis [OPTIONS]

Options:
    -h, --help       Print help information
    -V, --version    Print version information
    -c, --config     <CONFIG>
                     Config file to use
"#
)]
struct Args {
    #[arg(short, long, required = false, help = "Config file to use.")]
    config: Option<String>,
}

async fn run(server_config: ServerConfig) -> Result<(), Box<dyn Error>> {
    let mut server = Vetis::from_config(server_config).await?;
    if let Err(e) = server.run().await {
        error!("Failed to start server: {}", e);
    }
    Ok(())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();
    if let Some(config) = args.config {
        if Path::exists(Path::new(&config)) {
            let file = read_to_string(&config);
            if let Ok(file) = file {
                #[cfg(feature = "yaml-config")]
                let config = serde_yaml_ng::from_str::<VetisConfig>(&file);
                #[cfg(feature = "toml-config")]
                let config = toml::from_str::<VetisConfig>(&file);
                if let Ok(config) = config {
                    if config
                        .server
                        .enable_logging()
                    {
                        env_logger::Builder::from_env(
                            env_logger::Env::default().filter_or(
                                "VETIS_LOG",
                                config
                                    .server
                                    .log_level(),
                            ),
                        )
                        .format(|buf, record| {
                            let format =
                                format_description!("[month]-[day]-[year] [hour]:[minute]:[second]");
                            writeln!(
                                buf,
                                "{} [FRONT] {}: {}",
                                Timestamp::now()
                                    .format(format)
                                    .unwrap(),
                                record.level(),
                                record.args()
                            )
                        })
                        .format_timestamp_millis()
                        .format_module_path(false)
                        .target(env_logger::Target::Stdout)
                        .init();
                    }

                    let rt = tokio::runtime::Builder::new_multi_thread()
                        .enable_all()
                        .worker_threads(config.worker_threads)
                        .max_blocking_threads(config.max_blocking_threads)
                        .build()?;
                    rt.block_on(async { run(config.server).await })?;
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
