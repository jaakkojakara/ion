use std::collections::HashSet;
use std::fmt::Debug;
use std::sync::MutexGuard;
use std::{collections::BTreeMap, net::SocketAddr, time::Duration};

use bincode::{Decode, Encode};

use ion_common::net::NetworkPlayerInfo;
use ion_common::{Instant, Map, PlayerId};

use crate::core::FrameId;
use crate::core::world::{ActionType, WorldId, WorldType};

use super::{mp_client::MpClient, mp_server::MpServer};

// ---------------------------------------------------------- //
// ----------------- Common network types ------------------- //
// ---------------------------------------------------------- //

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NetworkEvent {
    OwnJoinAllowed,
    OwnJoinDenied { reason: String },

    OwnJoinDataRecvSuccess,
    OwnJoinDataRecvFailure { reason: String },

    OwnJoinSuccess,
    OwnJoinFailure { reason: String },

    ServerActionsNotReceived,

    PlayerJoinStart { player_info: NetworkPlayerInfo },
    PlayerJoinSuccess { player_info: NetworkPlayerInfo },
    PlayerJoinFailure { player_info: NetworkPlayerInfo },
    PlayerLeft { player_info: NetworkPlayerInfo },
}

#[derive(Debug, Clone)]
pub(crate) struct ActionSyncResult<W: WorldType> {
    pub players_joined: Vec<NetworkPlayerInfo>,
    pub players_left: Vec<PlayerId>,
    pub actions: Map<WorldId, BTreeMap<PlayerId, Vec<W::ActionType>>>,
    pub is_at_sync: bool,
}

pub(crate) enum MpInstance<W: WorldType> {
    Server(MpServer<W>),
    Client(MpClient<W>),
}

#[derive(Debug, Clone, Encode, Decode)]
pub(crate) enum MpMessage<C: ActionType> {
    ActionsFromClient {
        for_frame: FrameId,
        actions: Map<WorldId, Vec<C>>,
    },
    ActionsFromServer {
        for_frame: FrameId,
        actions: Map<PlayerId, BTreeMap<WorldId, Vec<C>>>,
    },

    LatencyUpdate {
        latency: Duration,
    },

    JoinReq {
        player_info: NetworkPlayerInfo,
    },
    JoinRes {
        accepted: bool,
        reason: Option<String>,
        server_player: Option<NetworkPlayerInfo>,
        client_players: Map<SocketAddr, NetworkPlayerInfo>,
        client_players_joining: Map<SocketAddr, NetworkPlayerInfo>,
    },
    JoinReqUniverseData {
        player_info: NetworkPlayerInfo,
    },
    JoinResUniverseData {
        universe_data: Vec<u8>,
        worlds_data: Vec<Vec<u8>>,
        active_frame: FrameId,
    },
    JoinComplete {
        player_info: NetworkPlayerInfo,
    },
    Leaving {
        player_info: NetworkPlayerInfo,
    },

    PlayerJoinStart {
        player_info: NetworkPlayerInfo,
    },
    PlayerJoinSuccess {
        player_info: NetworkPlayerInfo,
    },
    PlayerJoinFailure {
        player_info: NetworkPlayerInfo,
    },
    PlayerLeft {
        player_info: NetworkPlayerInfo,
    },
}

#[allow(clippy::type_complexity)]
#[derive(Clone, Encode, Decode)]
pub(super) struct MpActionBuffer<C: ActionType> {
    players: Map<FrameId, HashSet<PlayerId>>,
    actions: Map<FrameId, Map<WorldId, BTreeMap<PlayerId, Vec<C>>>>,
}

impl<C: ActionType> MpActionBuffer<C> {
    pub(super) fn new() -> Self {
        Self {
            players: Map::default(),
            actions: Map::default(),
        }
    }

    pub(super) fn contains_frame(&self, frame: FrameId) -> bool {
        self.actions.contains_key(&frame)
    }

    pub(super) fn contains_player_for_frame(&self, frame: FrameId, player_id: PlayerId) -> bool {
        self.actions
            .get(&frame)
            .map(|worlds| worlds.iter().all(|world| world.1.contains_key(&player_id)))
            .unwrap_or(false)
    }
    pub(super) fn import_actions(&mut self, frame: FrameId, world: WorldId, player_id: PlayerId, actions: &[C]) {
        self.players.entry(frame).or_default().insert(player_id);
        self.actions
            .entry(frame)
            .or_default()
            .entry(world)
            .or_default()
            .entry(player_id)
            .or_default()
            .append(&mut actions.to_vec());
    }

    pub(super) fn import_batch_actions(&mut self, frame: FrameId, actions: &Map<WorldId, BTreeMap<PlayerId, Vec<C>>>) {
        for (world_id, player_action_map) in actions {
            for (player_id, action_vec) in player_action_map {
                self.import_actions(frame, *world_id, *player_id, action_vec.as_slice());
            }
        }
    }

    pub(super) fn export_actions(&mut self, frame: FrameId) -> Option<Map<WorldId, BTreeMap<PlayerId, Vec<C>>>> {
        self.actions.get(&frame).cloned()
    }

    pub(super) fn delete_actions(&mut self, before_frame: FrameId) {
        self.actions.retain(|frame, _| frame >= &before_frame);
        self.players.retain(|frame, _| frame >= &before_frame);
    }

    pub(super) fn import_missing_actions_as_empty(&mut self, frame: FrameId, world: WorldId) {
        self.actions.entry(frame).or_default().entry(world).or_default();
        self.players.entry(frame).or_default();
    }

    pub(super) fn missing_players_for_frame(
        &self,
        frame: FrameId,
        all_players: &MutexGuard<Map<SocketAddr, (NetworkPlayerInfo, Instant)>>,
    ) -> Vec<(SocketAddr, NetworkPlayerInfo)> {
        let mut missing_players = vec![];
        for (socket, (player_info, _)) in all_players.iter() {
            if !self.contains_player_for_frame(frame, player_info.id) {
                missing_players.push((*socket, player_info.clone()));
            }
        }

        missing_players
    }

    pub(super) fn players_joined_on_frame(&self, frame: FrameId) -> Vec<PlayerId> {
        let active_frame_players = self.players.get(&frame);
        let prev_frame_players = self.players.get(&(frame.max(1) - 1));
        if let (Some(active_frame_players), Some(prev_frame_players)) = (active_frame_players, prev_frame_players) {
            active_frame_players
                .iter()
                .filter(|player_id| !prev_frame_players.contains(player_id))
                .copied()
                .collect()
        } else {
            Vec::new()
        }
    }

    pub(super) fn players_left_on_frame(&self, frame: FrameId) -> Vec<PlayerId> {
        let active_frame_players = self.players.get(&frame);
        let prev_frame_players = self.players.get(&(frame.max(1) - 1));
        if let (Some(active_frame_players), Some(prev_frame_players)) = (active_frame_players, prev_frame_players) {
            prev_frame_players
                .iter()
                .filter(|player_id| !active_frame_players.contains(player_id))
                .copied()
                .collect()
        } else {
            Vec::new()
        }
    }
}

impl<C: ActionType> Debug for MpActionBuffer<C> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AllActions").finish()
    }
}
