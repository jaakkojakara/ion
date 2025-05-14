use std::sync::mpsc::Sender;
use std::{
    collections::BTreeMap,
    net::SocketAddr,
    sync::{Mutex, MutexGuard, RwLock},
};

// Re-export these to allow mp-common to stay as private module.
use ion_common::net::{NetworkPlayerInfo, NetworkServerInfo};
use ion_common::{Map, PlayerId};
pub use mp_common::NetworkEvent;

use crate::core::{
    Constants,
    universe::Universe,
    world::{WorldId, WorldType},
};

use self::{
    mp_browser::MpBrowser,
    mp_client::MpClient,
    mp_common::{ActionSyncResult, MpInstance},
    mp_server::MpServer,
};

pub mod mp_browser;
mod mp_client;
mod mp_common;
mod mp_server;

/// Network capabilities of the Nawi engine.
/// Main feature is multiplayer. This works as standalone in LAN environments.
/// In global environments relies on external services system which provides for example
/// IP discovery, NAT punching and server listing.
pub struct Network<W: WorldType> {
    network_bind_addr: SocketAddr,
    network_host_addr: SocketAddr,
    network_event_sender: Sender<NetworkEvent>,

    mp_instance: RwLock<Option<MpInstance<W>>>,
    mp_browser_instance: Mutex<Option<MpBrowser>>,
}

impl<W: WorldType> Network<W> {
    pub(crate) fn new(constants: &Constants, network_event_sender: Sender<NetworkEvent>) -> Self {
        Self {
            network_bind_addr: constants
                .net
                .as_ref()
                .map(|c| c.bind_addr)
                .unwrap_or(SocketAddr::from(([0, 0, 0, 0], 0))),
            network_host_addr: constants
                .net
                .as_ref()
                .map(|c| c.host_addr)
                .unwrap_or(SocketAddr::from(([0, 0, 0, 0], 0))),
            mp_instance: RwLock::new(None),
            mp_browser_instance: Mutex::new(None),
            network_event_sender,
        }
    }

    // ---------------------------------------------------------- //
    // ------------------- Public functions --------------------- //
    // ---------------------------------------------------------- //

    // ------------- Multiplayer Client and Server -------------- //

    pub fn mp_start_server(&self, server_info: NetworkServerInfo, player_info: Option<NetworkPlayerInfo>) {
        self.verify_start_conditions();
        *self.mp_instance.write().unwrap() = Some(MpInstance::Server(MpServer::new(
            self.network_bind_addr,
            self.network_host_addr,
            server_info,
            player_info,
            self.network_event_sender.clone(),
        )));
    }

    pub fn mp_start_client(&self, server_info: NetworkServerInfo, player_info: NetworkPlayerInfo) {
        self.verify_start_conditions();
        *self.mp_instance.write().unwrap() = Some(MpInstance::Client(MpClient::new(
            self.network_bind_addr,
            self.network_host_addr,
            server_info,
            player_info,
            self.network_event_sender.clone(),
        )));
    }

    pub fn mp_stop_client_server(&self) {
        *self.mp_instance.write().unwrap() = None;
    }

    // -------------------- Server Browser -------------------- //

    pub fn mp_start_server_browser(&self) {
        self.verify_start_conditions();
        *self.mp_browser_instance.lock().unwrap() = Some(MpBrowser::new(self));
    }

    pub fn mp_stop_server_browser(&self) {
        *self.mp_browser_instance.lock().unwrap() = None;
    }

    pub fn mp_server_browser(&'_ self) -> MutexGuard<'_, Option<MpBrowser>> {
        self.mp_browser_instance.lock().unwrap()
    }

    // ------------------- Helper Functions ------------------- //

    pub fn is_mp_on(&self) -> bool {
        self.mp_instance.read().unwrap().is_some()
    }

    pub fn is_loopback(&self) -> bool {
        self.network_bind_addr.ip().is_loopback()
    }

    // ---------------------------------------------------------- //
    // ---------------- Private implementation ------------------ //
    // ---------------------------------------------------------- //

    pub(crate) fn verify_start_conditions(&self) {
        #[cfg(target_arch = "wasm32")]
        panic!("Multiplayer is not supported on wasm");

        assert!(
            self.mp_instance.read().unwrap().is_none(),
            "Can't start a network system while mp is active"
        );

        assert!(
            self.mp_browser_instance.lock().unwrap().is_none(),
            "Can't start a network system while server browser is active"
        );
    }

    /// Processes all network events that have been received.
    /// Mainly used to run network events in cases where universe does not yet exist,
    /// such as when joining as a client.
    ///
    /// No need to call if `mp_sync_actions` is called in the same loop.
    pub(crate) fn mp_sync_join_process(&self, universe: &Universe<W>) {
        if let Some(MpInstance::Client(instance)) = &*self.mp_instance.read().unwrap() {
            instance.sync_join_process(universe);
        }
    }

    /// Syncs all action buffers with all other players, and checks if universe is at sync.
    /// Returns none if sync fails (connection to server is lost).
    /// If not at sync, it means this client is falling behind, and should loop frames as fast as possible.
    /// Also reports any joining or leaving players on that frame.
    /// If playing offline, simply builds the action map out of own global and local actions.
    pub(crate) fn mp_sync_actions(
        &self,
        own_player: Option<&NetworkPlayerInfo>,
        own_global_actions: Map<WorldId, Vec<W::ActionType>>,
        own_local_actions: Map<WorldId, Vec<W::ActionType>>,
        universe: &Universe<W>,
        worlds_lock: &mut MutexGuard<Map<WorldId, W>>,
    ) -> Option<ActionSyncResult<W>> {
        match &*self.mp_instance.read().unwrap() {
            Some(mp_instance) => {
                let mut sync_result = match mp_instance {
                    MpInstance::Server(instance) => instance.sync_actions(own_global_actions, universe, worlds_lock),
                    MpInstance::Client(instance) => instance.sync_actions(own_global_actions, universe),
                };

                if let Some(player) = own_player {
                    if let Some(sync_result) = &mut sync_result {
                        for (world_id, mut actions) in own_local_actions {
                            sync_result.actions.entry(world_id).and_modify(|action_map| {
                                action_map.entry(player.id).and_modify(|action_vec| {
                                    action_vec.append(&mut actions);
                                });
                            });
                        }
                    }
                }

                sync_result
            }
            _ => {
                let mut all_actions: Map<WorldId, BTreeMap<PlayerId, Vec<W::ActionType>>> = Map::default();
                if let Some(player) = own_player {
                    for (world_id, global_actions) in own_global_actions {
                        all_actions
                            .entry(world_id)
                            .or_default()
                            .insert(player.id, global_actions);
                    }

                    for (world_id, mut actions) in own_local_actions {
                        all_actions.entry(world_id).and_modify(|action_map| {
                            action_map.entry(player.id).and_modify(|action_vec| {
                                action_vec.append(&mut actions);
                            });
                        });
                    }
                } else {
                    for world in worlds_lock.values() {
                        all_actions.entry(world.id()).or_default();
                    }
                }

                let players_joined = if universe.active_frame() == 0 && own_player.is_some() {
                    // World has just started, any active player joins on this frame
                    vec![own_player.unwrap().clone()]
                } else {
                    vec![]
                };

                Some(ActionSyncResult {
                    players_joined,
                    players_left: Vec::new(),
                    actions: all_actions,
                    is_at_sync: true,
                })
            }
        }
    }
}
