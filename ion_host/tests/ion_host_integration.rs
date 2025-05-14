use std::net::SocketAddr;
use std::sync::{Mutex, MutexGuard, OnceLock};
use std::thread::{JoinHandle, sleep};
use std::time::Duration;

use ion_common::net::udp_network_socket::UdpNetworkSocket;
use ion_common::net::{NetworkServerInfo, SysMessage, UdpMessage};
use ion_common::{self, LogLevel};
use ion_host::config::Config;
use ion_host::run_ion_host;

static TEST_SERVICES: OnceLock<JoinHandle<()>> = OnceLock::new();
static TEST_LOCK: OnceLock<Mutex<()>> = OnceLock::new();

fn start_test_services_if_needed() -> SocketAddr {
    let test_config = Config {
        port: 3333,
        server_ping_timeout: Duration::from_secs(2),
        nat_punch_relay_timeout: Duration::from_secs(2),
        socket_info_resp_timeout: Duration::from_secs(2),
        server_list_resp_timeout: Duration::from_secs(2),
    };
    TEST_SERVICES.get_or_init(move || {
        std::thread::spawn(move || {
            ion_common::set_logger_on(LogLevel::Info);
            run_ion_host(test_config)
        })
    });
    SocketAddr::from(([127, 0, 0, 1], 3333))
}

fn acquire_test_lock<'a>() -> MutexGuard<'a, ()> {
    TEST_LOCK.get_or_init(|| Mutex::new(())).lock().unwrap()
}

#[test]
fn socket_info_service_works() {
    let service_addr = start_test_services_if_needed();
    let _test_lock = acquire_test_lock();

    let addr = SocketAddr::from(([127, 0, 0, 1], 3334));
    let socket: UdpNetworkSocket<UdpMessage<()>> = UdpNetworkSocket::new(addr);

    socket.send(
        service_addr,
        UdpMessage::SysMessage(SysMessage::SocketInfoReq),
        Duration::from_secs(5),
    );

    let resp = socket.try_recv_timeout(Duration::from_secs(1)).unwrap();
    match resp.1 {
        UdpMessage::SysMessage(msg) => match msg {
            SysMessage::SocketInfoRes { addr } => assert_eq!(3334, addr.port()),
            _ => panic!("Wrong message type"),
        },
        UdpMessage::MpMessage(_) => panic!("Wrong message type"),
    }
}

#[test]
fn server_info_service_works() {
    let service_addr = start_test_services_if_needed();
    let _test_lock = acquire_test_lock();

    let addr = SocketAddr::from(([127, 0, 0, 1], 3334));
    let addr2 = SocketAddr::from(([127, 0, 0, 1], 3335));
    let socket: UdpNetworkSocket<UdpMessage<()>> = UdpNetworkSocket::new(addr);
    let socket2: UdpNetworkSocket<UdpMessage<()>> = UdpNetworkSocket::new(addr2);

    let server_1 = NetworkServerInfo {
        id: 23,
        name: "test1".to_string(),
        addr,
        is_global: false,
        has_password: false,
        description: "".to_string(),
        cur_player_count: 0,
        max_player_count: 0,
    };

    let server_2 = NetworkServerInfo {
        id: 24,
        name: "test2".to_string(),
        addr: addr2,
        is_global: false,
        has_password: false,
        description: "".to_string(),
        cur_player_count: 0,
        max_player_count: 0,
    };

    socket.send(
        service_addr,
        UdpMessage::SysMessage(SysMessage::ServerInfoPost {
            server: server_1.clone(),
        }),
        Duration::from_secs(5),
    );

    socket2.send(
        service_addr,
        UdpMessage::SysMessage(SysMessage::ServerInfoPost {
            server: server_2.clone(),
        }),
        Duration::from_secs(5),
    );

    sleep(Duration::from_millis(10));

    socket.send(
        service_addr,
        UdpMessage::SysMessage(SysMessage::ServerInfoReq),
        Duration::from_secs(5),
    );

    let resp = socket.try_recv_timeout(Duration::from_secs(1)).unwrap();
    match resp.1 {
        UdpMessage::SysMessage(msg) => match msg {
            SysMessage::ServerInfoResGlobal { servers } => {
                assert_eq!(servers.len(), 2);
                assert!(servers.contains(&server_1));
                assert!(servers.contains(&server_2));
            }
            _ => panic!("Wrong message type"),
        },
        UdpMessage::MpMessage(_) => panic!("Wrong message type"),
    }

    socket.send(
        service_addr,
        UdpMessage::SysMessage(SysMessage::ServerInfoDelete {}),
        Duration::from_secs(5),
    );

    sleep(Duration::from_millis(10));

    socket.send(
        service_addr,
        UdpMessage::SysMessage(SysMessage::ServerInfoReq),
        Duration::from_secs(5),
    );

    let resp = socket.try_recv_timeout(Duration::from_secs(1)).unwrap();
    match resp.1 {
        UdpMessage::SysMessage(msg) => match msg {
            SysMessage::ServerInfoResGlobal { servers } => {
                assert_eq!(servers.len(), 1);
                assert!(!servers.contains(&server_1));
                assert!(servers.contains(&server_2));
            }
            _ => panic!("Wrong message type"),
        },
        UdpMessage::MpMessage(_) => panic!("Wrong message type"),
    }
}

#[test]
fn nat_punch_service_works() {
    let service_addr = start_test_services_if_needed();
    let _test_lock = acquire_test_lock();

    let addr = SocketAddr::from(([127, 0, 0, 1], 3334));
    let addr2 = SocketAddr::from(([127, 0, 0, 1], 3335));
    let socket: UdpNetworkSocket<UdpMessage<()>> = UdpNetworkSocket::new(addr);
    let socket2: UdpNetworkSocket<UdpMessage<()>> = UdpNetworkSocket::new(addr2);

    socket.send(
        service_addr,
        UdpMessage::SysMessage(SysMessage::NatPunchRelay { to: addr2 }),
        Duration::from_secs(5),
    );

    let resp = socket2.try_recv_timeout(Duration::from_secs(1)).unwrap();
    match resp.1 {
        UdpMessage::SysMessage(msg) => match msg {
            SysMessage::NatPunchStart { to } => {
                assert_eq!(to, addr)
            }
            _ => panic!("Wrong message type"),
        },
        UdpMessage::MpMessage(_) => panic!("Wrong message type"),
    }
}
