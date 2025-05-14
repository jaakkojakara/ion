use std::{net::SocketAddr, time::Duration};

use ion_common::log_info;
use ion_common::net::udp_network_socket::UdpNetworkSocket;
use ion_common::net::{SysMessage, UdpMessage};

use crate::core::world::WorldType;
use crate::net::NetworkServerInfo;

use super::Network;

pub struct MpBrowser {
    host_addr: SocketAddr,
    own_global_addr_resp: Option<SocketAddr>,
    own_local_addr_resp: Option<SocketAddr>,
    udp_socket: UdpNetworkSocket<UdpMessage<()>>,

    global_servers: Vec<NetworkServerInfo>,
    local_servers: Vec<NetworkServerInfo>,
}

impl MpBrowser {
    pub(crate) fn new<W: WorldType>(network: &Network<W>) -> Self {
        let udp_socket = UdpNetworkSocket::new(network.network_bind_addr);
        if !udp_socket.is_loopback() {
            udp_socket.send(
                network.network_host_addr,
                UdpMessage::SysMessage(SysMessage::SocketInfoReq),
                Duration::from_secs(10),
            );
        }

        udp_socket.enable_broadcast();

        Self {
            host_addr: network.network_host_addr,
            own_global_addr_resp: None,
            own_local_addr_resp: None,
            udp_socket,
            global_servers: Vec::new(),
            local_servers: Vec::new(),
        }
    }

    pub fn request_local_server_info(&self) {
        log_info!("Requesting local server info");
        let msg = UdpMessage::SysMessage(SysMessage::ServerInfoReq);
        if self.udp_socket.local_ip_addr().unwrap().is_loopback() {
            let test_addrs = vec![SocketAddr::from(([127, 0, 0, 1], self.host_addr.port()))];
            for addr in test_addrs {
                self.udp_socket.send(addr, msg.clone(), Duration::from_secs(5));
            }
        } else {
            self.udp_socket.send_broadcast(msg.clone());
        }
    }

    pub fn request_global_server_info(&self) {
        log_info!("Requesting global server info");
        let msg = UdpMessage::SysMessage(SysMessage::ServerInfoReq);
        self.udp_socket.send(self.host_addr, msg, Duration::from_secs(15));
    }

    pub fn global_servers(&mut self) -> &[NetworkServerInfo] {
        self.handle_network_events();
        &self.global_servers
    }

    pub fn local_servers(&mut self) -> &[NetworkServerInfo] {
        self.handle_network_events();
        &self.local_servers
    }

    pub fn own_global_addr(&self) -> Option<SocketAddr> {
        self.own_global_addr_resp
    }

    pub fn own_local_addr(&self) -> Option<SocketAddr> {
        self.own_local_addr_resp
    }

    fn handle_network_events(&mut self) {
        for (from_addr, msg) in self.udp_socket.try_recv_all() {
            match msg {
                UdpMessage::SysMessage(msg) => match msg {
                    SysMessage::SocketInfoRes { addr } => {
                        log_info!("Received SocketInfoRes, own addr: {:?}", addr);
                        if from_addr == self.host_addr {
                            self.own_global_addr_resp = Some(addr);
                        } else {
                            self.own_local_addr_resp = Some(addr);
                        }
                    }
                    SysMessage::ServerInfoResGlobal { mut servers } => {
                        log_info!("Received ServerInfoResGlobal for {} servers", servers.len());
                        if from_addr == self.host_addr {
                            self.global_servers.append(&mut servers);
                        }
                    }
                    SysMessage::ServerInfoResLocal { server } => {
                        log_info!("Received ServerInfoResLocal from {:?}: {:?}", from_addr, server);
                        self.udp_socket.send(
                            server.addr,
                            UdpMessage::SysMessage(SysMessage::SocketInfoReq),
                            Duration::from_secs(5),
                        );
                        self.local_servers.push(server);
                    }
                    _ => {}
                },
                UdpMessage::MpMessage(_) => {
                    log_info!("Received MpMessage while listening in MpBrowser");
                }
            }
        }
    }
}
