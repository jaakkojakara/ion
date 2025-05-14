use std::sync::mpsc::Sender;
use std::{
    net::SocketAddr,
    sync::{
        Mutex, RwLock,
        atomic::{AtomicBool, Ordering},
    },
    time::Duration,
};

use ion_common::net::udp_network_socket::UdpNetworkSocket;
use ion_common::net::{SysMessage, UdpMessage};
use ion_common::util::native_spin_sleep;
use ion_common::{Instant, log_info};
use ion_common::{Map, log_warn};

use crate::core::{
    universe::{Universe, UniverseDataType},
    world::{WorldId, WorldType},
};
use crate::net::{NetworkPlayerInfo, NetworkServerInfo, PlayerId};
use crate::util::concurrency::AtomicInstant;

use super::mp_common::{ActionSyncResult, MpActionBuffer, MpMessage, NetworkEvent};

pub const FRAME_LATENCY_SAFETY_MULTIPLIER: u32 = 5;
pub const COMMAND_TIMEOUT: Duration = Duration::from_secs(5);
pub const JOIN_TIMEOUT: Duration = Duration::from_secs(30);

// ---------------------------------------------------------- //
// ------------------- Multiplayer Client ------------------- //
// ---------------------------------------------------------- //

pub struct MpClient<W: WorldType> {
    server_addr: SocketAddr,
    server_info: RwLock<NetworkServerInfo>,
    player_info: NetworkPlayerInfo,

    udp_socket: UdpNetworkSocket<UdpMessage<MpMessage<W::ActionType>>>,

    server_player: RwLock<Option<NetworkPlayerInfo>>,
    client_players: RwLock<Map<PlayerId, NetworkPlayerInfo>>,
    client_players_joining: RwLock<Map<PlayerId, NetworkPlayerInfo>>,

    join_request_sent: AtomicBool,
    join_started_at: AtomicInstant,
    join_synced_up: AtomicBool,

    latency_duration: Mutex<Duration>,
    action_holder: Mutex<MpActionBuffer<W::ActionType>>,
    network_event_sender: Sender<NetworkEvent>,
}

impl<W: WorldType> MpClient<W> {
    pub(crate) fn new(
        network_bind_addr: SocketAddr,
        network_host_addr: SocketAddr,
        server_info: NetworkServerInfo,
        mut player_info: NetworkPlayerInfo,
        network_event_sender: Sender<NetworkEvent>,
    ) -> Self {
        log_info!("Starting MpClient: {:?}", &server_info);

        let udp_socket = UdpNetworkSocket::new(network_bind_addr);
        let join_request_sent = if server_info.is_global {
            // Start nat punch process to open route to server
            udp_socket.send(
                network_host_addr,
                UdpMessage::SysMessage(SysMessage::NatPunchRelay { to: server_info.addr }),
                Duration::from_secs(30),
            );
            udp_socket.send(
                server_info.addr,
                UdpMessage::SysMessage(SysMessage::NatPunchPing),
                Duration::from_secs(30),
            );
            AtomicBool::new(false)
        } else {
            // No need for nat punching, just request to join
            player_info.addr.set_ip(udp_socket.local_ip_addr().unwrap());
            udp_socket.send(
                server_info.addr,
                UdpMessage::MpMessage(MpMessage::JoinReq {
                    player_info: player_info.clone(),
                }),
                Duration::from_secs(30),
            );
            AtomicBool::new(true)
        };

        Self {
            server_addr: server_info.addr,
            server_info: RwLock::new(server_info),
            player_info,
            udp_socket,
            server_player: RwLock::new(None),
            client_players: RwLock::new(Map::default()),
            client_players_joining: RwLock::new(Map::default()),
            join_request_sent,
            join_started_at: AtomicInstant::new(Instant::now()),
            join_synced_up: AtomicBool::new(false),
            latency_duration: Mutex::new(Duration::from_millis(100)),
            action_holder: Mutex::new(MpActionBuffer::new()),
            network_event_sender,
        }
    }

    pub(crate) fn sync_join_process(&self, universe: &Universe<W>) {
        let mut received_actions = self.action_holder.lock().unwrap();
        self.process_network_events(universe, &mut received_actions);

        if self.join_started_at.load(Ordering::Relaxed) + JOIN_TIMEOUT < Instant::now() {
            self.network_event_sender
                .send(NetworkEvent::OwnJoinDataRecvFailure {
                    reason: "Timeout downloading map data".to_owned(),
                })
                .ok();
        }
    }

    pub(crate) fn sync_actions(
        &self,
        own_global_actions: Map<WorldId, Vec<W::ActionType>>,
        universe: &Universe<W>,
    ) -> Option<ActionSyncResult<W>> {
        let mut actions = self.action_holder.lock().unwrap();
        self.process_network_events(universe, &mut actions);

        {
            let active_frame = universe.active_frame();
            let send_for_frame = {
                let rtt = *self.latency_duration.lock().unwrap() * FRAME_LATENCY_SAFETY_MULTIPLIER;
                active_frame + (rtt.as_micros() / universe.universe_frame_time().as_micros() + 1).max(2) as u64
            };
            let action_msg = UdpMessage::MpMessage(MpMessage::ActionsFromClient {
                for_frame: send_for_frame,
                actions: own_global_actions,
            });

            self.udp_socket
                .send(self.server_addr, action_msg, Duration::from_secs(5));

            // Receive combined actions from server
            let wait_start = Instant::now();
            let mut frame_actions = actions.export_actions(active_frame);
            while frame_actions.is_none() && wait_start + COMMAND_TIMEOUT > Instant::now() {
                native_spin_sleep(Duration::from_millis(1));
                self.process_network_events(universe, &mut actions);
                frame_actions = actions.export_actions(active_frame);
            }
            let is_at_sync = !actions.contains_frame(active_frame + 1);

            // Check if just caught up with server after join
            if is_at_sync {
                if !self.join_synced_up.load(Ordering::Acquire) {
                    self.join_synced_up.store(true, Ordering::Release);
                    self.network_event_sender.send(NetworkEvent::OwnJoinSuccess).ok();
                    self.udp_socket.send(
                        self.server_addr,
                        UdpMessage::MpMessage(MpMessage::JoinComplete {
                            player_info: self.player_info.clone(),
                        }),
                        Duration::from_secs(15),
                    );
                }
            } else if !self.join_synced_up.load(Ordering::Acquire)
                && self.join_started_at.load(Ordering::Relaxed) + JOIN_TIMEOUT < Instant::now()
            {
                self.network_event_sender
                    .send(NetworkEvent::OwnJoinFailure {
                        reason: "Could not catch up with server state".to_owned(),
                    })
                    .ok();
            }

            match frame_actions {
                Some(frame_actions) => {
                    let players_left = actions.players_left_on_frame(active_frame);
                    let players_joined = actions
                        .players_joined_on_frame(active_frame)
                        .into_iter()
                        .map(|player_id| {
                            if player_id != self.player_info.id {
                                self.client_players
                                    .read()
                                    .unwrap()
                                    .iter()
                                    .find(|player| player.1.id == player_id)
                                    .expect("Must have info for joining player")
                                    .1
                                    .clone()
                            } else {
                                self.player_info.clone()
                            }
                        })
                        .collect();

                    actions.delete_actions(active_frame.saturating_sub(20000));

                    Some(ActionSyncResult {
                        players_joined,
                        players_left,
                        actions: frame_actions,
                        is_at_sync,
                    })
                }
                _ => {
                    let msg = NetworkEvent::ServerActionsNotReceived;
                    self.network_event_sender.send(msg).ok();
                    None
                }
            }
        }
    }

    fn process_network_events(&self, universe: &Universe<W>, action_holder: &mut MpActionBuffer<W::ActionType>) {
        for (from_addr, msg) in self.udp_socket.try_recv_all() {
            match msg {
                UdpMessage::SysMessage(msg) => {
                    if msg == SysMessage::NatPunchPing {
                        if from_addr == self.server_addr {
                            log_info!("Received NatPunchPing");
                            if !self.join_request_sent.load(Ordering::Acquire) {
                                self.join_request_sent.store(true, Ordering::Release);
                                log_info!("Sending JoinReq to {:?}", self.server_addr);
                                self.udp_socket.send(
                                    self.server_addr,
                                    UdpMessage::MpMessage(MpMessage::JoinReq {
                                        player_info: self.player_info.clone(),
                                    }),
                                    Duration::from_secs(30),
                                );
                            }
                        } else {
                            log_info!("Got SystemMessage from non-server addr: {:?}", from_addr);
                        }
                    }
                }
                UdpMessage::MpMessage(msg) => {
                    if from_addr == self.server_addr {
                        match msg {
                            MpMessage::ActionsFromServer { for_frame, actions } => {
                                action_holder.import_batch_actions(for_frame, &actions);
                            }
                            MpMessage::JoinRes {
                                accepted,
                                reason,
                                server_player,
                                client_players,
                                client_players_joining,
                            } => {
                                if accepted {
                                    log_info!("Received JoinRes accepted");
                                    self.udp_socket.send(
                                        from_addr,
                                        UdpMessage::MpMessage(MpMessage::JoinReqUniverseData {
                                            player_info: self.player_info.clone(),
                                        }),
                                        Duration::from_secs(5),
                                    );

                                    self.network_event_sender.send(NetworkEvent::OwnJoinAllowed).ok();
                                    *self.server_player.write().unwrap() = server_player;

                                    let mut client_players_map = self.client_players.write().unwrap();
                                    for (_, player) in client_players {
                                        client_players_map.insert(player.id, player);
                                    }
                                    let mut client_players_joining_map = self.client_players_joining.write().unwrap();
                                    for (_, player) in client_players_joining {
                                        client_players_joining_map.insert(player.id, player);
                                    }
                                } else {
                                    log_info!("Received JoinRes denied");
                                    self.network_event_sender
                                        .send(NetworkEvent::OwnJoinDenied {
                                            reason: reason.unwrap(),
                                        })
                                        .ok();
                                }
                            }
                            MpMessage::PlayerJoinStart { player_info } => {
                                log_info!("Received PlayerJoinStart: {:?}", player_info);
                                self.server_info.write().unwrap().cur_player_count += 1;
                                self.client_players_joining
                                    .write()
                                    .unwrap()
                                    .insert(player_info.id, player_info.clone());
                                self.network_event_sender
                                    .send(NetworkEvent::PlayerJoinStart { player_info })
                                    .ok();
                            }
                            MpMessage::PlayerJoinSuccess { player_info } => {
                                log_info!("Received PlayerJoinSuccess: {:?}", player_info);
                                match self.client_players_joining.write().unwrap().remove(&player_info.id) {
                                    Some(player_info) => {
                                        self.client_players
                                            .write()
                                            .unwrap()
                                            .insert(player_info.id, player_info.clone());
                                        self.network_event_sender
                                            .send(NetworkEvent::PlayerJoinSuccess { player_info })
                                            .ok();
                                    }
                                    _ => {
                                        panic!("Desynced players. Received PlayerJoinSuccess for unknown player");
                                    }
                                }
                            }
                            MpMessage::PlayerJoinFailure { player_info } => {
                                log_warn!("Received PlayerJoinFailure: {:?}", player_info);
                                self.client_players_joining.write().unwrap().remove(&player_info.id);
                                self.network_event_sender
                                    .send(NetworkEvent::PlayerJoinFailure { player_info })
                                    .ok();
                            }
                            MpMessage::LatencyUpdate { latency } => {
                                *self.latency_duration.lock().unwrap() = latency;
                            }
                            MpMessage::JoinResUniverseData {
                                universe_data,
                                worlds_data,
                                active_frame,
                            } => {
                                log_info!("Received JoinResUniverseData for frame: {:?}", active_frame);
                                universe.load_universe(
                                    UniverseDataType::from_bytes(
                                        universe_data.as_slice(),
                                        Some(self.server_info.read().unwrap().clone()),
                                        Some(self.player_info.clone()),
                                    ),
                                    worlds_data
                                        .into_iter()
                                        .map(|world_data| {
                                            WorldType::from_bytes(&world_data, Some(self.player_info.clone())).unwrap()
                                        })
                                        .collect(),
                                    Some(active_frame),
                                );

                                self.network_event_sender
                                    .send(NetworkEvent::OwnJoinDataRecvSuccess)
                                    .ok();
                            }
                            MpMessage::PlayerLeft { player_info } => {
                                log_info!("Received PlayerLeft: {:?}", player_info);
                                self.server_info.write().unwrap().cur_player_count -= 1;
                                self.client_players.write().unwrap().remove(&player_info.id);
                            }
                            _ => {}
                        }
                    } else {
                        log_info!("Got MpMessage from non-server addr: {:?}", from_addr);
                    }
                }
            }
        }
    }
}

impl<W: WorldType> Drop for MpClient<W> {
    fn drop(&mut self) {
        self.udp_socket.send(
            self.server_addr,
            UdpMessage::MpMessage(MpMessage::Leaving {
                player_info: self.player_info.clone(),
            }),
            Duration::from_secs(5),
        );
    }
}
