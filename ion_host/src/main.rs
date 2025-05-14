//! # Ion Host
//!
//! The Ion Host provides server hosting capabilities for the Ion Engine.
//! Currently, following services are provided:
//! - SocketInfo: Provides the socket address of the requesting client.
//! - ServerList: Provides a list of known multiplayer servers.
//! - NatPunch: Provides NAT punching protocol for joining multiplayer servers.

use ion_common::LogLevel;
use ion_host::config::Config;
use ion_host::run_ion_host;

/// Entry point for the host
pub fn main() {
    ion_common::set_logger_on(LogLevel::Debug);
    run_ion_host(Config::default());
}
