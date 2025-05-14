use std::net::SocketAddr;
use std::sync::Arc;

use ion_common::log_dbg;
use ion_common::net::udp_network_socket::UdpNetworkSocket;
use ion_common::net::{SysMessage, UdpMessage};

use crate::config::Config;
use crate::services::service_nat_punch::ServiceNatPunch;
use crate::services::service_server_list::ServiceServerList;
use crate::services::service_socket_info::ServiceSocketInfo;

pub mod service_nat_punch;
pub mod service_server_list;
pub mod service_socket_info;

pub fn run_services(config: Config) -> ! {
    let udp_socket: Arc<UdpNetworkSocket<UdpMessage<()>>> = Arc::new(UdpNetworkSocket::new(
        SocketAddr::from(([0, 0, 0, 0], config.port)),
    ));

    let service_nat_punch = ServiceNatPunch::new(udp_socket.clone(), config);
    let service_server_list = ServiceServerList::new(udp_socket.clone(), config);
    let service_socket_info = ServiceSocketInfo::new(udp_socket.clone(), config);

    loop {
        let (from_addr, udp_message) = udp_socket.recv_blocking();
        log_dbg!("Received request from {:?}: {:?}", from_addr, udp_message);
        if let UdpMessage::SysMessage(message) = udp_message {
            match message {
                SysMessage::SocketInfoReq => {
                    service_socket_info.handle_socket_info_req(from_addr);
                }
                SysMessage::ServerInfoReq => {
                    service_server_list.handle_server_info_req(from_addr);
                }
                SysMessage::ServerInfoPost { server } => {
                    service_server_list.handle_server_info_post(from_addr, server);
                }
                SysMessage::ServerInfoDelete => {
                    service_server_list.handle_server_info_delete(from_addr);
                }
                SysMessage::NatPunchRelay { to } => {
                    service_nat_punch.handle_nat_punch_relay(from_addr, to);
                }
                _ => {}
            }
        }
    }
}
