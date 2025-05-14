use std::{net::SocketAddr, sync::Arc};

use ion_common::log_info;
use ion_common::net::{udp_network_socket::UdpNetworkSocket, SysMessage, UdpMessage};

use crate::config::Config;

pub struct ServiceSocketInfo {
    socket: Arc<UdpNetworkSocket<UdpMessage<()>>>,
    config: Config,
}

impl ServiceSocketInfo {
    pub fn new(socket: Arc<UdpNetworkSocket<UdpMessage<()>>>, config: Config) -> Self {
        log_info!("Creating service SocketInfo");
        Self { socket, config }
    }

    pub fn handle_socket_info_req(&self, from_addr: SocketAddr) {
        let res = UdpMessage::SysMessage(SysMessage::SocketInfoRes { addr: from_addr });
        self.socket
            .send(from_addr, res, self.config.socket_info_resp_timeout);
    }
}
