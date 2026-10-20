use clap::Parser;
use log::{error, info};
use regex::{Captures, regex};
use serde::Deserialize;
use std::{collections::HashMap, error::Error, fs::read_to_string, path::Path, time::Duration};
#[cfg(target_family = "unix")]
use tokio::signal::unix::{SignalKind, signal};
#[cfg(target_family = "windows")]
use tokio::signal::windows::{ctrl_c, ctrl_shutdown};
use vetis::{VetisServer as _, server::ServerConfig};
use vetis_tokio::Vetis;

#[cfg(feature = "logging")]
#[allow(unused_imports)]
use vetis_log::StderrLogConfig;

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

#[derive(Deserialize)]
pub struct VetisConfig {
    worker_threads: usize,
    max_blocking_threads: usize,
    server: ServerConfig,
}

#[cfg(feature = "metrics")]
mod metrics;

#[derive(Parser)]
#[command(
    name = "vetis-server",
    about = "vetis - a very tiny server",
    long_about = r#"
vetis - a very tiny server

Usage:
    vetis-server [OPTIONS]

Options:
    -h, --help       Print help information
    -V, --version    Print version information
    -d, --deamon     Start vetis as deamon 
    -c, --config     <CONFIG>
                     Config file to use
"#
)]
struct Args {
    #[arg(
        short,
        long,
        required = false,
        num_args = 0..=1,
        require_equals = true,
        default_missing_value = "true", 
        help = "Start vetis as deamon"
    )]
    deamon: Option<bool>,
    #[arg(
        short,
        long,
        required = false,
        help = "Config file to use"
    )]
    config: Option<String>,
}

#[cfg(target_family = "unix")]
async fn run_in_background() -> Result<(), Box<dyn Error>> {
    let mut signal = signal(SignalKind::terminate())?;
    signal.recv().await;
    Ok(())
}

#[cfg(target_family = "windows")]
async fn run_in_background() -> Result<(), Box<dyn Error>> {
    let mut signal = ctrl_shutdown()?;
    signal.recv().await;
    Ok(())
}

#[cfg(target_family = "unix")]
async fn run_in_foreground() -> Result<(), Box<dyn Error>> {
    let mut signal = signal(SignalKind::interrupt())?;
    signal.recv().await;
    Ok(())
}

#[cfg(target_family = "windows")]
async fn run_in_foreground() -> Result<(), Box<dyn Error>> {
    let mut signal = ctrl_c()?;
    signal.recv().await;
    Ok(())
}

async fn run(server_config: ServerConfig, deamon: Option<bool>) -> Result<(), Box<dyn Error>> {
    let mut server = Vetis::from_config(server_config).await?;
    if let Err(e) = server.run().await {
        error!(target: "vetis", "Failed to start server: {}", e);
    }

    if let Some(is_deamon) = deamon
        && is_deamon
    {
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

fn parse_file(file_contents: &str) -> Result<VetisConfig, Box<dyn std::error::Error>> {

    let empty = String::default();

    let mut resolved_files = HashMap::new();
    let includes_regex = regex!(r"\$\(include_file\s*=\s*(.*)\)");
    let mut matches = includes_regex.captures_iter(file_contents);
    while let Some(matches) = matches.next(){
        let (full, [file_path]) = matches.extract();
        match read_to_string(file_path) {
            Ok(file_contents) => {
                resolved_files.insert(full, file_contents);
            }
            Err(e) => {
                panic!("Could not include file: {}", e.to_string());
            }
        }
    }
    let file_contents = includes_regex.replace_all(file_contents, |caps: &Captures| {
        resolved_files.get(&caps[0]).unwrap_or(&empty)
    });

    let mut resolved_vars = HashMap::new();
    let env_vars = regex!("\\$\\(([a-zA-Z_]*):{0,1}([\\\"\\\']{0,1}.*[\\\"\\\']{0,1})\\)");
    let mut matches = env_vars.captures_iter(&file_contents);
    while let Some(matches) = matches.next(){
        let (full, [name, default]) = matches.extract();
        let value = match std::env::var_os(name) {
            Some(var_value) => {
                format!("{:?}", var_value)
            }
            None => {
                default.to_string()
            }
        };
        resolved_vars.insert(full, value);
    }

    let file_contents = env_vars.replace_all(&file_contents, |caps: &Captures| {
        resolved_vars.get(&caps[0]).unwrap_or(&empty)
    });

    #[cfg(all(feature = "yaml-config", feature = "toml-config"))]
    let config = match serde_saphyr::from_str::<VetisConfig>(&file_contents) {
        Ok(config) => config,
        Err(yaml_error) => toml::from_str::<VetisConfig>(&file_contents).map_err(|toml_error| {
            std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!("Could not parse config as YAML ({yaml_error}) or TOML ({toml_error})"),
            )
        })?,
    };
    #[cfg(all(feature = "yaml-config", not(feature = "toml-config")))]
    let config = serde_saphyr::from_str::<VetisConfig>(&file_contents)?;
    #[cfg(all(feature = "toml-config", not(feature = "yaml-config")))]
    let config = toml::from_str::<VetisConfig>(&file_contents)?;
    Ok(config)
}

fn init_logs(config: &VetisConfig) {
    let mut log = logforth::starter_log::builder();
    if let Some(server_log_config) = config
        .server
        .log()
    {
        log = log.dispatch(|d| {
            server_log_config.into_builder("vetis", d)
        });
    }

    for host_config in config.server.hosts().iter() {
        if let Some(log_config) = host_config.log()  {
            for bind_address in host_config.bind_addresses().iter() {
                log = log.dispatch(|d| log_config.into_builder(&format!("{}:{}", host_config.hostname(), bind_address.1), d))
            }
        }
    }

    log.apply();
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();
    if let Some(config) = args.config {
        if Path::exists(Path::new(&config)) {
            let file = read_to_string(&config);
            if let Ok(file) = file {
                let config = parse_file(&file);
                if let Ok(config) = config {
                    init_logs(&config);
                    #[cfg(feature = "metrics")]
                    metrics::init_metrics();

                    let rt = tokio::runtime::Builder::new_multi_thread()
                        .enable_all()
                        .worker_threads(config.worker_threads)
                        .max_blocking_threads(config.max_blocking_threads)
                        .build()?;
                    rt.block_on(async { run(config.server, args.deamon).await })?;
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        fs,
        path::PathBuf,
        sync::atomic::{AtomicU64, Ordering},
        time::SystemTime,
    };

    static NEXT_TEMP_FILE_ID: AtomicU64 = AtomicU64::new(0);

    fn config_with_worker_threads(value: &str) -> String {
        #[cfg(feature = "yaml-config")]
        return format!("worker_threads: {value}\nmax_blocking_threads: 4\nserver: {{}}\n");

        #[cfg(all(feature = "toml-config", not(feature = "yaml-config")))]
        return format!("worker_threads = {value}\nmax_blocking_threads = 4\n[server]\n");
    }

    #[cfg(feature = "toml-config")]
    fn toml_config_with_worker_threads(value: &str) -> String {
        format!("worker_threads = {value}\nmax_blocking_threads = 4\n[server]\n")
    }

    fn temp_file(contents: &str) -> PathBuf {
        let id = NEXT_TEMP_FILE_ID.fetch_add(1, Ordering::Relaxed);
        let timestamp = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .expect("system clock must be after the Unix epoch")
            .as_nanos();
        let path = std::env::temp_dir()
            .join(format!("vetis-server-test-{}-{timestamp}-{id}.conf", std::process::id()));
        fs::write(&path, contents).expect("temporary config file should be writable");
        path
    }

    struct TempFile(PathBuf);

    impl Drop for TempFile {
        fn drop(&mut self) {
            let _ = fs::remove_file(&self.0);
        }
    }

    #[cfg(feature = "yaml-config")]
    #[test]
    fn parses_yaml_server_configuration() {
        let config = parse_file(&config_with_worker_threads("8")).unwrap();

        assert_eq!(config.worker_threads, 8);
        assert_eq!(config.max_blocking_threads, 4);
        assert_eq!(
            config
                .server
                .workers(),
            1
        );
    }

    #[cfg(feature = "toml-config")]
    #[test]
    fn parses_toml_server_configuration() {
        let config = parse_file(&toml_config_with_worker_threads("8")).unwrap();

        assert_eq!(config.worker_threads, 8);
        assert_eq!(config.max_blocking_threads, 4);
        assert_eq!(
            config
                .server
                .workers(),
            1
        );
    }

    #[cfg(any(feature = "yaml-config", feature = "toml-config"))]
    #[test]
    fn uses_default_for_missing_environment_variable() {
        let name = "VETIS_TEST_DEFAULT_WORKER_THREADS";
        assert!(std::env::var_os(name).is_none());
        let config = parse_file(&config_with_worker_threads(&format!("$({name}:8)"))).unwrap();

        assert_eq!(config.worker_threads, 8);
    }

    #[cfg(any(feature = "yaml-config", feature = "toml-config"))]
    #[test]
    fn includes_file_contents() {
        let included = TempFile(temp_file("8"));
        let config = parse_file(&config_with_worker_threads(&format!(
            "$(include_file = {})",
            included.0.display()
        )))
        .unwrap();

        assert_eq!(config.worker_threads, 8);
    }

    #[cfg(any(feature = "yaml-config", feature = "toml-config"))]
    #[test]
    fn rejects_invalid_configuration() {
        assert!(parse_file("not a server configuration").is_err());
    }

    #[test]
    fn parses_cli_options() {
        let args = Args::try_parse_from(["vetis-server", "--config", "vetis.yaml"]).unwrap();
        assert_eq!(
            args.config
                .as_deref(),
            Some("vetis.yaml")
        );
        assert_eq!(args.deamon, None);

        let args = Args::try_parse_from(["vetis-server", "--deamon"]).unwrap();
        assert_eq!(args.deamon, Some(true));

        let args = Args::try_parse_from(["vetis-server", "--deamon=false"]).unwrap();
        assert_eq!(args.deamon, Some(false));
    }
}
