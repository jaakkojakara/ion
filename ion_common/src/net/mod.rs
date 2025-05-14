use std::fmt::{self, Display};
use std::net::SocketAddr;
use std::{collections::HashMap, fmt::Debug};

use bincode::{Decode, Encode};

use crate::{PlayerId, ServerId};

pub mod tcp_network_socket;
pub mod udp_network_socket;

// ---------------------------------------------------------- //
// --------------- Player and Server types ------------------ //
// ---------------------------------------------------------- //

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Encode, Decode)]
pub struct NetworkPlayerInfo {
    pub id: PlayerId,
    pub name: String,
    pub addr: SocketAddr,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Encode, Decode)]
pub struct NetworkServerInfo {
    pub id: ServerId,
    pub name: String,
    pub addr: SocketAddr,
    pub is_global: bool,
    pub has_password: bool,
    pub description: String,
    pub cur_player_count: u32,
    pub max_player_count: u32,
}

// ---------------------------------------------------------- //
// ------------------ Udp message types --------------------- //
// ---------------------------------------------------------- //

#[derive(Debug, Clone, PartialEq, Eq, Encode, Decode)]
pub enum UdpMessage<T>
where
    T: 'static + Debug + Clone + Send + Encode + Decode<()>,
{
    SysMessage(SysMessage),
    MpMessage(T),
}

#[derive(Debug, Clone, PartialEq, Eq, Encode, Decode)]
pub enum SysMessage {
    SocketInfoReq,
    SocketInfoRes { addr: SocketAddr },
    ServerInfoReq,
    ServerInfoResGlobal { servers: Vec<NetworkServerInfo> },
    ServerInfoResLocal { server: NetworkServerInfo },
    ServerInfoPost { server: NetworkServerInfo },
    ServerInfoDelete,
    NatPunchRelay { to: SocketAddr },
    NatPunchStart { to: SocketAddr },
    NatPunchPing,
}

// ---------------------------------------------------------- //
// ------------------ Tcp message types --------------------- //
// ---------------------------------------------------------- //

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HttpMethod {
    GET,
    POST,
    DELETE,
}

impl Display for HttpMethod {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

#[derive(Debug, Clone)]
pub struct HttpRequest {
    pub url: String,
    pub method: HttpMethod,
    pub headers: HashMap<String, String>,
    pub body: Vec<u8>,
}

#[derive(Debug, Clone)]
pub struct HttpResponse {
    pub status_code: u16,
    pub headers: HashMap<String, String>,
    pub body: Vec<u8>,
}

// ---------------------------------------------------------- //
// ------------------------- Tests -------------------------- //
// ---------------------------------------------------------- //

#[cfg(test)]
mod tests {
    use std::net::SocketAddr;

    use bincode::{Decode, Encode, config};

    use crate::net::{NetworkPlayerInfo, NetworkServerInfo, SysMessage, UdpMessage};

    #[derive(Debug, Clone, PartialEq, Eq, Encode, Decode)]
    struct TestStruct {
        count: u32,
        data: Vec<u16>,
    }

    #[test]
    fn can_serialize_and_deserialize_network_player_info() {
        let player_info = NetworkPlayerInfo {
            id: 123,
            name: "test-name".to_owned(),
            addr: SocketAddr::from(([1, 1, 1, 1], 1234)),
        };

        let bytes = bincode::encode_to_vec(&player_info, config::standard()).unwrap();
        let (d, _): (NetworkPlayerInfo, usize) =
            bincode::decode_from_slice(bytes.as_slice(), config::standard()).unwrap();
        assert_eq!(&d, &player_info);
    }

    #[test]
    fn can_serialize_and_deserialize_network_server_info() {
        let server_info = NetworkServerInfo {
            id: 123,
            name: "test-name".to_owned(),
            addr: SocketAddr::from(([1, 1, 1, 1], 1234)),
            is_global: false,
            has_password: false,
            description: "desc".to_owned(),
            cur_player_count: 3,
            max_player_count: 8,
        };

        let bytes = bincode::encode_to_vec(&server_info, config::standard()).unwrap();
        let (d, _): (NetworkServerInfo, usize) =
            bincode::decode_from_slice(bytes.as_slice(), config::standard()).unwrap();
        assert_eq!(&d, &server_info);
    }

    #[test]
    fn udp_message_serialization_works_with_different_game_message_types() {
        let orig_msg = UdpMessage::<u32>::SysMessage(SysMessage::SocketInfoRes {
            addr: SocketAddr::from(([0, 1, 2, 3], 5277)),
        });

        let bytes = bincode::encode_to_vec(&orig_msg, config::standard()).unwrap();
        let (decoded_msg, _): (UdpMessage<TestStruct>, usize) =
            bincode::decode_from_slice(bytes.as_slice(), config::standard()).unwrap();

        match decoded_msg {
            UdpMessage::SysMessage(system_message_decoded) => match orig_msg {
                UdpMessage::SysMessage(system_message_original) => {
                    assert_eq!(system_message_decoded, system_message_original)
                }
                UdpMessage::MpMessage(_) => panic!("Got wrong type"),
            },
            UdpMessage::MpMessage(_) => panic!("Got wrong type"),
        }
    }
}
