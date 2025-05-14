use std::net::{IpAddr, Ipv4Addr};
use std::sync::mpsc::Sender;
use std::{
    net::SocketAddr,
    sync::{Mutex, MutexGuard, atomic::Ordering},
    time::Duration,
};

use ion_common::net::udp_network_socket::UdpNetworkSocket;
use ion_common::net::{SysMessage, UdpMessage};
use ion_common::{Instant, log_info};
use ion_common::{Map, log_warn};

use crate::core::DEFAULT_UPS;
use crate::core::universe::UniverseDataType;
use crate::net::{NetworkPlayerInfo, NetworkServerInfo};
use crate::util::concurrency::AtomicInstant;
use crate::{
    core::{
        FrameId,
        universe::Universe,
        world::{WorldId, WorldType},
    },
    net::mp_common::{MpActionBuffer, MpMessage, NetworkEvent},
};

use super::mp_common::ActionSyncResult;

const GLOBAL_PUBLISH_INTERVAL: Duration = Duration::from_secs(20);

const PLAYER_TIMEOUT: Duration = Duration::from_secs(15);
const PLAYER_JOIN_TIMEOUT: Duration = Duration::from_secs(60);

// ---------------------------------------------------------- //
// ------------------- Multiplayer Server ------------------- //
// ---------------------------------------------------------- //

pub struct MpServer<W: WorldType> {
    udp_socket: UdpNetworkSocket<UdpMessage<MpMessage<W::ActionType>>>,
    host_addr: SocketAddr,
    server_player: Option<NetworkPlayerInfo>,
    server_info: Mutex<NetworkServerInfo>,
    client_players: Mutex<Map<SocketAddr, (NetworkPlayerInfo, Instant)>>,
    client_players_joining: Mutex<Map<SocketAddr, (NetworkPlayerInfo, Instant)>>,

    global_publish_last: AtomicInstant,

    latencies: Mutex<Map<SocketAddr, Duration>>,
    actions: Mutex<MpActionBuffer<W::ActionType>>,

    network_event_sender: Sender<NetworkEvent>,
}

impl<W: WorldType> MpServer<W> {
    pub(crate) fn new(
        network_bind_addr: SocketAddr,
        network_host_addr: SocketAddr,
        mut server_info: NetworkServerInfo,
        mut server_player: Option<NetworkPlayerInfo>,
        network_event_sender: Sender<NetworkEvent>,
    ) -> Self {
        log_info!("Starting MpServer: {:?}", &server_info);
        let udp_socket = UdpNetworkSocket::new(network_bind_addr);
        if !udp_socket.is_loopback() {
            if server_info.is_global {
                server_info.addr.set_ip(IpAddr::V4(Ipv4Addr::UNSPECIFIED));
                udp_socket.send(
                    network_host_addr,
                    UdpMessage::SysMessage(SysMessage::SocketInfoReq),
                    Duration::from_secs(30),
                );
            } else {
                server_info.addr.set_ip(
                    udp_socket
                        .local_ip_addr()
                        .expect("Local servers must get a valid local ip"),
                );
            }
        } else {
            server_info.addr = network_bind_addr;
        }

        server_info.cur_player_count = 0;

        if let Some(server_player) = &mut server_player {
            server_player.addr.set_ip(server_info.addr.ip());
        }

        Self {
            host_addr: network_host_addr,
            udp_socket,
            server_info: Mutex::new(server_info),
            server_player,
            client_players: Mutex::new(Map::default()),
            client_players_joining: Mutex::new(Map::default()),
            network_event_sender,

            global_publish_last: AtomicInstant::new(Instant::now() - Duration::from_secs(60)),

            latencies: Mutex::new(Map::default()),
            actions: Mutex::new(MpActionBuffer::new()),
        }
    }

    pub(crate) fn sync_actions(
        &self,
        own_global_actions: Map<WorldId, Vec<W::ActionType>>,
        universe: &Universe<W>,
        worlds_lock: &mut MutexGuard<Map<WorldId, W>>,
    ) -> Option<ActionSyncResult<W>> {
        self.process_network_events(universe, worlds_lock);

        let mut client_players = self.client_players.lock().unwrap();
        let mut client_players_joining = self.client_players_joining.lock().unwrap();

        let mut actions = self.actions.lock().unwrap();
        let mut latencies = self.latencies.lock().unwrap();

        let active_frame = universe.active_frame();
        let next_frame = universe.active_frame() + 1;

        self.check_and_report_dropping_players(&mut client_players, &mut client_players_joining, &mut latencies);

        self.check_and_publish_server_info(&client_players, &client_players_joining);

        self.check_and_report_latencies(active_frame, &mut latencies);

        {
            // Add own actions to action holder
            if let Some(own_player) = &self.server_player {
                for (world_id, own_actions) in &own_global_actions {
                    actions.import_actions(next_frame, *world_id, own_player.id, own_actions);
                }
            } else {
                for world_id in own_global_actions.keys() {
                    actions.import_missing_actions_as_empty(next_frame, *world_id);
                }
            }

            // Fill own actions for the first frame
            for world_id in own_global_actions.keys() {
                actions.import_missing_actions_as_empty(active_frame, *world_id);
            }

            // Check if we are missing any actions from any players
            for (_addr, player) in actions.missing_players_for_frame(next_frame, &client_players) {
                for world_id in own_global_actions.keys() {
                    actions.import_actions(next_frame, *world_id, player.id, &[]);
                }
            }

            // Send action data for the next frame
            for addr in client_players.keys() {
                let msg = UdpMessage::MpMessage(MpMessage::ActionsFromServer {
                    for_frame: next_frame,
                    actions: actions.export_actions(next_frame).unwrap(),
                });
                self.udp_socket.send(*addr, msg, Duration::from_secs(15));
            }

            for addr in client_players_joining.keys() {
                let msg = UdpMessage::MpMessage(MpMessage::ActionsFromServer {
                    for_frame: next_frame,
                    actions: actions.export_actions(next_frame).unwrap(),
                });
                self.udp_socket.send(*addr, msg, Duration::from_secs(15));
            }

            // Prepare ActionSyncResult for active frame
            let players_left: Vec<_> = actions.players_left_on_frame(active_frame);
            let players_joined: Vec<_> = actions
                .players_joined_on_frame(active_frame)
                .into_iter()
                .map(|player_id| {
                    if self.server_player.is_some() && self.server_player.as_ref().unwrap().id == player_id {
                        self.server_player.as_ref().unwrap().clone()
                    } else {
                        client_players
                            .iter()
                            .find(|(_, (player, _))| player.id == player_id)
                            .expect("Must have info for joining player")
                            .1
                            .0
                            .clone()
                    }
                })
                .collect();

            actions.delete_actions(active_frame - 20000.min(active_frame));

            Some(ActionSyncResult {
                players_joined,
                players_left,
                actions: actions.export_actions(active_frame).unwrap(),
                is_at_sync: true,
            })
        }
    }

    fn check_and_report_latencies(&self, active_frame: FrameId, latencies: &mut MutexGuard<Map<SocketAddr, Duration>>) {
        if active_frame % DEFAULT_UPS == 0 {
            for (addr, latency) in &mut **latencies {
                *latency = self.udp_socket.latency_of(*addr).unwrap_or(Duration::from_millis(100));
                self.udp_socket.send(
                    *addr,
                    UdpMessage::MpMessage(MpMessage::LatencyUpdate { latency: *latency }),
                    Duration::from_secs(15),
                )
            }
        }
    }

    fn check_and_publish_server_info(
        &self,
        client_players: &MutexGuard<Map<SocketAddr, (NetworkPlayerInfo, Instant)>>,
        client_players_joining: &MutexGuard<Map<SocketAddr, (NetworkPlayerInfo, Instant)>>,
    ) {
        let mut server_info = self.server_info.lock().unwrap();
        let self_player_count = if self.server_player.is_some() { 1 } else { 0 };
        let cur_player_count = client_players.len() + client_players_joining.len() + self_player_count;
        server_info.cur_player_count = cur_player_count as u32;

        if server_info.is_global
            && self.global_publish_last.load(Ordering::Relaxed) + GLOBAL_PUBLISH_INTERVAL < Instant::now()
            && server_info.addr.port() != 0
        {
            self.udp_socket.send(
                self.host_addr,
                UdpMessage::SysMessage(SysMessage::ServerInfoPost {
                    server: (*server_info).clone(),
                }),
                Duration::from_secs(5),
            );
            self.global_publish_last.store(Instant::now(), Ordering::Relaxed);
        }
    }

    fn check_and_report_dropping_players(
        &self,
        client_players: &mut MutexGuard<Map<SocketAddr, (NetworkPlayerInfo, Instant)>>,
        client_players_joining: &mut MutexGuard<Map<SocketAddr, (NetworkPlayerInfo, Instant)>>,
        latencies: &mut MutexGuard<Map<SocketAddr, Duration>>,
    ) {
        let now = Instant::now();

        let mut dropping_players: Vec<NetworkPlayerInfo> = Vec::new();
        client_players.retain(|_, (player, last_msg)| {
            let retain = *last_msg + PLAYER_TIMEOUT > now;
            if !retain {
                log_warn!("Player timed out: {:?}", player);
                latencies.remove(&player.addr);
                dropping_players.push(player.clone());
                self.network_event_sender
                    .send(NetworkEvent::PlayerLeft {
                        player_info: player.clone(),
                    })
                    .ok();
            }
            retain
        });

        let mut dropping_joining_players: Vec<NetworkPlayerInfo> = Vec::new();
        client_players_joining.retain(|_, (player, last_msg)| {
            let retain = *last_msg + PLAYER_JOIN_TIMEOUT > now;
            if !retain {
                log_warn!("Player join timed out: {:?}", player);
                dropping_joining_players.push(player.clone());
                self.network_event_sender
                    .send(NetworkEvent::PlayerJoinFailure {
                        player_info: player.clone(),
                    })
                    .ok();
            }
            retain
        });

        for (addr, (_, _)) in &**client_players {
            for player_info in &dropping_players {
                self.udp_socket.send(
                    *addr,
                    UdpMessage::MpMessage(MpMessage::PlayerLeft {
                        player_info: player_info.clone(),
                    }),
                    Duration::from_secs(10),
                );
            }

            for player_info in &dropping_joining_players {
                self.udp_socket.send(
                    *addr,
                    UdpMessage::MpMessage(MpMessage::PlayerJoinFailure {
                        player_info: player_info.clone(),
                    }),
                    Duration::from_secs(10),
                );
            }
        }

        for (addr, (_, _)) in &**client_players_joining {
            for player_info in &dropping_players {
                self.udp_socket.send(
                    *addr,
                    UdpMessage::MpMessage(MpMessage::PlayerLeft {
                        player_info: player_info.clone(),
                    }),
                    Duration::from_secs(10),
                );
            }

            for player_info in &dropping_joining_players {
                self.udp_socket.send(
                    *addr,
                    UdpMessage::MpMessage(MpMessage::PlayerJoinFailure {
                        player_info: player_info.clone(),
                    }),
                    Duration::from_secs(10),
                );
            }
        }
    }

    fn process_network_events(&self, universe: &Universe<W>, worlds_lock: &mut MutexGuard<Map<WorldId, W>>) {
        for (from_addr, msg) in self.udp_socket.try_recv_all() {
            match msg {
                UdpMessage::SysMessage(msg) => match msg {
                    SysMessage::SocketInfoRes { addr } => {
                        log_info!("Received SocketInfoRes from {:?}", from_addr);
                        if from_addr == self.host_addr {
                            self.server_info.lock().unwrap().addr = addr;
                        }
                    }
                    SysMessage::SocketInfoReq {} => {
                        log_info!("Received SocketInfoReq from {:?}", from_addr);
                        self.udp_socket.send(
                            from_addr,
                            UdpMessage::SysMessage(SysMessage::SocketInfoRes { addr: from_addr }),
                            Duration::from_secs(10),
                        );
                    }
                    SysMessage::NatPunchStart { to } => {
                        log_info!("Received NatPunchStart from {:?}", from_addr);
                        if from_addr == self.host_addr {
                            self.udp_socket.send(
                                to,
                                UdpMessage::SysMessage(SysMessage::NatPunchPing),
                                Duration::from_secs(15),
                            );
                        }
                    }
                    SysMessage::ServerInfoReq {} => {
                        log_info!("Received ServerInfoReq from {:?}", from_addr);
                        let server_info = self.server_info.lock().unwrap();
                        self.udp_socket.send(
                            from_addr,
                            UdpMessage::SysMessage(SysMessage::ServerInfoResLocal {
                                server: server_info.clone(),
                            }),
                            Duration::from_secs(10),
                        );
                    }
                    _ => {}
                },
                UdpMessage::MpMessage(msg) => match msg {
                    MpMessage::ActionsFromClient { for_frame, actions } => {
                        if let Some((player_info, last_msg)) = self.client_players.lock().unwrap().get_mut(&from_addr) {
                            if for_frame > universe.active_frame() {
                                let mut action_map = self.actions.lock().unwrap();
                                for (world_id, actions) in actions {
                                    action_map.import_actions(for_frame, world_id, player_info.id, &actions);
                                }
                                *last_msg = Instant::now();
                            }
                        }
                    }

                    MpMessage::JoinReq { player_info } => {
                        log_info!("Received JoinReq for {:?} from {:?}", player_info, from_addr);
                        let allowed = player_info.addr == from_addr;
                        if allowed {
                            let client_players = self.client_players.lock().unwrap();
                            let mut client_players_joining = self.client_players_joining.lock().unwrap();

                            self.network_event_sender
                                .send(NetworkEvent::PlayerJoinStart {
                                    player_info: player_info.clone(),
                                })
                                .unwrap();

                            self.udp_socket.send(
                                from_addr,
                                UdpMessage::MpMessage(MpMessage::JoinRes {
                                    accepted: true,
                                    reason: None,
                                    server_player: self.server_player.clone(),
                                    client_players: client_players
                                        .clone()
                                        .into_iter()
                                        .map(|(addr, player)| (addr, player.0))
                                        .collect(),
                                    client_players_joining: client_players_joining
                                        .clone()
                                        .into_iter()
                                        .map(|(addr, player)| (addr, player.0))
                                        .collect(),
                                }),
                                Duration::from_secs(15),
                            );

                            for (addr, (_, _)) in &*client_players {
                                self.udp_socket.send(
                                    *addr,
                                    UdpMessage::MpMessage(MpMessage::PlayerJoinStart {
                                        player_info: player_info.clone(),
                                    }),
                                    Duration::from_secs(10),
                                );
                            }

                            for (addr, (_, _)) in &*client_players_joining {
                                self.udp_socket.send(
                                    *addr,
                                    UdpMessage::MpMessage(MpMessage::PlayerJoinStart {
                                        player_info: player_info.clone(),
                                    }),
                                    Duration::from_secs(10),
                                );
                            }

                            client_players_joining.insert(from_addr, (player_info, Instant::now()));
                        } else {
                            let reason = if player_info.addr != from_addr {
                                "Player IP does not match msg source IP"
                            } else {
                                "Access denied"
                            };

                            self.udp_socket.send(
                                from_addr,
                                UdpMessage::MpMessage(MpMessage::JoinRes {
                                    accepted: false,
                                    reason: Some(reason.to_owned()),
                                    server_player: None,
                                    client_players: Map::default(),
                                    client_players_joining: Map::default(),
                                }),
                                Duration::from_secs(15),
                            );
                        }
                    }
                    MpMessage::JoinReqUniverseData { .. } => {
                        log_info!("Received JoinReqGameData from {:?}", from_addr);
                        if self.client_players_joining.lock().unwrap().contains_key(&from_addr) {
                            let worlds_data: Vec<_> = worlds_lock.iter().map(|world| world.1.as_bytes()).collect();
                            let universe_data = universe.lock_universe_data().as_ref().unwrap().as_bytes(worlds_lock);
                            let msg = UdpMessage::MpMessage(MpMessage::<W::ActionType>::JoinResUniverseData {
                                universe_data,
                                worlds_data,
                                active_frame: universe.active_frame(),
                            });
                            self.udp_socket.send(from_addr, msg, Duration::from_secs(60));
                        }
                    }
                    MpMessage::JoinComplete { .. } => {
                        log_info!("Received JoinComplete from {:?}", from_addr);
                        let mut client_players = self.client_players.lock().unwrap();
                        let mut client_players_joining = self.client_players_joining.lock().unwrap();
                        if let Some((player_info, _)) = client_players_joining.remove(&from_addr) {
                            for (addr, (_, _)) in &*client_players {
                                self.udp_socket.send(
                                    *addr,
                                    UdpMessage::MpMessage(MpMessage::PlayerJoinSuccess {
                                        player_info: player_info.clone(),
                                    }),
                                    Duration::from_secs(10),
                                );
                            }

                            for (addr, (_, _)) in &*client_players_joining {
                                self.udp_socket.send(
                                    *addr,
                                    UdpMessage::MpMessage(MpMessage::PlayerJoinStart {
                                        player_info: player_info.clone(),
                                    }),
                                    Duration::from_secs(10),
                                );
                            }

                            client_players.insert(from_addr, (player_info.clone(), Instant::now()));

                            self.latencies
                                .lock()
                                .unwrap()
                                .insert(from_addr, Duration::from_millis(100));

                            self.network_event_sender
                                .send(NetworkEvent::PlayerJoinSuccess { player_info })
                                .unwrap();
                        }
                    }
                    MpMessage::Leaving { .. } => {
                        log_info!("Received Leaving from {:?}", from_addr);
                        self.latencies.lock().unwrap().remove(&from_addr);
                        if let Some((player_info, _)) = self.client_players.lock().unwrap().remove(&from_addr) {
                            for addr in self.client_players.lock().unwrap().keys() {
                                let msg = UdpMessage::MpMessage(MpMessage::PlayerLeft {
                                    player_info: player_info.clone(),
                                });
                                self.udp_socket.send(*addr, msg, Duration::from_secs(15));
                            }
                            self.network_event_sender
                                .send(NetworkEvent::PlayerLeft { player_info })
                                .ok();
                        }
                    }
                    _ => {}
                },
            }
        }
    }
}
