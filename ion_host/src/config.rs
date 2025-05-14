use std::time::Duration;

const SERVER_PING_TIMEOUT: Duration = Duration::from_secs(60);
const NAT_PUNCH_RELAY_TIMEOUT: Duration = Duration::from_secs(20);
const SOCKET_INFO_RESP_TIMEOUT: Duration = Duration::from_secs(20);
const SERVER_LIST_RESP_TIMOUT: Duration = Duration::from_secs(20);

#[derive(Debug, Clone, Copy)]
pub struct Config {
    pub port: u16,
    pub server_ping_timeout: Duration,
    pub nat_punch_relay_timeout: Duration,
    pub socket_info_resp_timeout: Duration,
    pub server_list_resp_timeout: Duration,
}

impl Default for Config {
    fn default() -> Self {
        let port = std::env::args()
            .nth(1)
            .map(|arg| arg.parse())
            .expect("UDP port must be given as an argument")
            .expect("Given argument must be a valid port");

        Self {
            port,
            server_ping_timeout: SERVER_PING_TIMEOUT,
            nat_punch_relay_timeout: NAT_PUNCH_RELAY_TIMEOUT,
            socket_info_resp_timeout: SOCKET_INFO_RESP_TIMEOUT,
            server_list_resp_timeout: SERVER_LIST_RESP_TIMOUT,
        }
    }
}
