use crate::config::Config;
use ion_common::log_info;

pub mod config;
mod services;

pub fn run_ion_host(config: Config) -> ! {
    log_info!("Starting up host services");
    services::run_services(config)
}
