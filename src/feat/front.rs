use std::{
    error::Error,
    net::{IpAddr, Ipv4Addr},
    time::Duration,
};

use log::{error, info};
use vetis::{VetisServer as _, log::LogConfig};
use vetis_log::StdoutLogConfig;
use vetis_static::{StaticPathConfig, tokio::StaticPath};
use vetis_tokio::{HostConfig, Vetis, host::Host};

use crate::{FrontCommand, run_in_foreground};

pub async fn init_server(root: &str, port: u16) -> Result<Vetis, Box<dyn Error>> {
    let log_config = StdoutLogConfig::builder()
        .log_level(log::Level::Info)
        .build();
    logforth::starter_log::builder()
        .dispatch(|d| log_config.into_builder("vetis", d))
        .apply();

    let host_config = HostConfig::builder()
        .hostname("localhost")
        .bind_addresses(&[(IpAddr::V4(Ipv4Addr::UNSPECIFIED), port)])
        .root_directory(root)
        .build()?;

    let mut virtual_host = Host::new(host_config).await?;
    virtual_host.add_path(StaticPath::new(
        StaticPathConfig::builder()
            .uri("/")
            .directory(root)
            .extensions("\\.(html|js|css|svg|png|ico|jsx|ts|tsx|json|mjs)$")
            .index_files(vec!["index.html".into()])
            .build()?,
    ));

    let server = Vetis::builder()
        .add_host(virtual_host)
        .await?
        .build();
    Ok(server)
}

pub async fn run(cmd: FrontCommand) -> Result<(), Box<dyn Error>> {
    let mut server = init_server(&cmd.root, cmd.port).await?;
    if let Err(e) = server.run().await {
        error!(target: "vetis", "Failed to start server: {}", e);
    }

    run_in_foreground().await?;

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

pub fn serve(cmd: FrontCommand) -> Result<(), Box<dyn Error>> {
    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()?;
    rt.block_on(async { run(cmd).await })?;

    Ok(())
}
