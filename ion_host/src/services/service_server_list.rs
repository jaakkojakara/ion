use std::{cell::RefCell, net::SocketAddr, sync::Arc, time::Instant};

use ion_common::net::{
    udp_network_socket::UdpNetworkSocket, NetworkServerInfo, SysMessage, UdpMessage,
};
use ion_common::{log_info, Map};

use crate::config::Config;

pub struct ServiceServerList {
    socket: Arc<UdpNetworkSocket<UdpMessage<()>>>,
    config: Config,
    servers: RefCell<Map<SocketAddr, (Instant, NetworkServerInfo)>>,
}

impl ServiceServerList {
    pub fn new(socket: Arc<UdpNetworkSocket<UdpMessage<()>>>, config: Config) -> Self {
        log_info!("Creating service ServerList");
        Self {
            socket,
            config,
            servers: RefCell::new(Map::default()),
        }
    }

    pub fn handle_server_info_req(&self, from_addr: SocketAddr) {
        let now = Instant::now();
        self.servers
            .borrow_mut()
            .retain(|_, (updated, _)| *updated + self.config.server_ping_timeout > now);
        let server_list: Vec<_> = self
            .servers
            .borrow_mut()
            .iter()
            .map(|(_, server)| server.1.clone())
            .collect();

        let res_msg = UdpMessage::SysMessage(SysMessage::ServerInfoResGlobal {
            servers: server_list,
        });

        self.socket
            .send(from_addr, res_msg, self.config.socket_info_resp_timeout)
    }
    pub fn handle_server_info_post(&self, from_addr: SocketAddr, server: NetworkServerInfo) {
        if from_addr == server.addr {
            self.servers
                .borrow_mut()
                .insert(from_addr, (Instant::now(), server));
        }
    }
    pub fn handle_server_info_delete(&self, from_addr: SocketAddr) {
        self.servers.borrow_mut().remove(&from_addr);
    }
}
