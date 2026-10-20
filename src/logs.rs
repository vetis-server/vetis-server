use crate::VetisConfig;

pub(super) fn init_logs(config: &VetisConfig) {
    let mut log = logforth::starter_log::builder();
    if let Some(server_log_config) = config.server.log() {
        log = log.dispatch(|d| server_log_config.into_builder("vetis", d));
    }

    for host_config in config
        .server
        .hosts()
        .iter()
    {
        if let Some(log_config) = host_config.log() {
            for bind_address in host_config
                .bind_addresses()
                .iter()
            {
                log = log.dispatch(|d| {
                    log_config
                        .into_builder(&format!("{}:{}", host_config.hostname(), bind_address.1), d)
                })
            }
        }
    }

    log.apply();
}
