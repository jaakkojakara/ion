use std::{net::SocketAddr, sync::Arc};

use ion_common::log_info;
use ion_common::net::{udp_network_socket::UdpNetworkSocket, SysMessage, UdpMessage};

use crate::config::Config;

pub struct ServiceNatPunch {
    socket: Arc<UdpNetworkSocket<UdpMessage<()>>>,
    config: Config,
}

impl ServiceNatPunch {
    pub fn new(socket: Arc<UdpNetworkSocket<UdpMessage<()>>>, config: Config) -> Self {
        log_info!("Creating service NatPunch");
        Self { socket, config }
    }

    pub fn handle_nat_punch_relay(&self, from_addr: SocketAddr, to: SocketAddr) {
        let res = UdpMessage::SysMessage(SysMessage::NatPunchStart { to: from_addr });
        self.socket
            .send(to, res, self.config.nat_punch_relay_timeout)
    }
}
