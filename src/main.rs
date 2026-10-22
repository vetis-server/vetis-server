use clap::{Args, Parser, Subcommand};
use std::error::Error;
#[cfg(target_family = "unix")]
use tokio::signal::unix::{SignalKind, signal};
#[cfg(target_family = "windows")]
use tokio::signal::windows::{ctrl_c, ctrl_shutdown};
#[cfg(feature = "flash")]
#[allow(unused_imports)]
use vetis_flash::FlashPathConfig;
#[cfg(feature = "logging")]
#[allow(unused_imports)]
use vetis_log::StderrLogConfig;
#[cfg(feature = "proxy")]
#[allow(unused_imports)]
use vetis_proxy::ProxyPathConfig;
#[cfg(feature = "rev-proxy")]
#[allow(unused_imports)]
use vetis_rev_proxy::ReverseProxyPathConfig;
#[cfg(feature = "static")]
#[allow(unused_imports)]
use vetis_static::StaticPathConfig;

mod config;
mod feat;
#[cfg(feature = "logging")]
mod logs;
#[cfg(feature = "metrics")]
mod metrics;

#[derive(Parser)]
#[command(
    name = "vetis-server",
    about = "vetis - a very tiny server",
    long_about = r#"
vetis - a very tiny server

Usage:
    vetis-server [COMMAND] [OPTIONS]

Options:
    -h, --help       Print help information
    -V, --version    Print version information

Below you have a list of commands you can use for different purposes:

To start server from a config file
    start
    stop

For static file serving
    front

"#
)]
struct Cli {
    #[command(subcommand)]
    commands: Commands,
}

#[derive(Debug, Subcommand)]
enum Commands {
    Start(StartCommand),
    Stop,
    Front(FrontCommand),
}

#[derive(Debug, Args)]
struct StartCommand {
    #[arg(
        short,
        long,
        required = false,
        num_args = 0..=1,
        require_equals = true,
        default_missing_value = "true", 
        help = "Start vetis as deamon"
    )]
    deamon: bool,
    #[arg(
        short, 
        long, 
        required = false, 
        help = "Config file to use"
    )]
    config: Option<String>,
}
#[derive(Debug, Args)]
struct FrontCommand {
    #[arg(
        short,
        long,
        required = false,
        num_args = 0..=1,
        require_equals = true,
        default_missing_value = "true", 
        help = "Start vetis as deamon"
    )]
    deamon: bool,
    #[arg(
        short, 
        long, 
        required = false, 
        num_args = 0..=1,
        require_equals = true,
        default_value = ".",
        default_missing_value = ".", 
        help = "Root directory to serve."
    )]
    root: String,
    #[arg(
        short, 
        long, 
        required = false, 
        num_args = 0..=1,
        require_equals = true,
        default_value = "0.0.0.0",
        default_missing_value = "0.0.0.0", 
        help = "Interface to bind to."
    )]
    interface: String,
    #[arg(
        short, 
        long, 
        required = false, 
        num_args = 0..=1,
        require_equals = true,
        default_value = "4444",
        default_missing_value = "4444", 
        help = "Port to bind to."
    )]
    port: u16,
}

#[cfg(target_family = "unix")]
pub(crate) async fn run_in_background() -> Result<(), Box<dyn Error>> {
    let mut signal = signal(SignalKind::terminate())?;
    signal.recv().await;
    Ok(())
}

#[cfg(target_family = "windows")]
pub(crate) async fn run_in_background() -> Result<(), Box<dyn Error>> {
    let mut signal = ctrl_shutdown()?;
    signal.recv().await;
    Ok(())
}

#[cfg(target_family = "unix")]
pub(crate) async fn run_in_foreground() -> Result<(), Box<dyn Error>> {
    let mut signal = signal(SignalKind::interrupt())?;
    signal.recv().await;
    Ok(())
}

#[cfg(target_family = "windows")]
pub(crate) async fn run_in_foreground() -> Result<(), Box<dyn Error>> {
    let mut signal = ctrl_c()?;
    signal.recv().await;
    Ok(())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Cli::parse();
    match args.commands {
        Commands::Start(cmd) => {
            feat::server::start(cmd)?;
        }
        Commands::Stop => {
            feat::server::stop()?;
        }
        Commands::Front(cmd) => {
            feat::front::serve(cmd)?;
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use crate::config::parse_file;
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
}
