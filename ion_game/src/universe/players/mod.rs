use std::ops::Deref;
use std::sync::{Mutex, MutexGuard};

use bincode::{Decode, Encode};

use ion_common::net::NetworkPlayerInfo;
use ion_common::{Map, PlayerId};
use ion_engine::core::coordinates::Location;
use ion_engine::core::world::WorldId;

use crate::universe::chunk::Chunks;
use crate::universe::entities::Entity;
use crate::universe::entities::entity_creators::create_player;
use crate::universe::entities::mobs::Mobs;
use crate::universe::players::player::Player;
use crate::universe::world::World;

pub mod player;

#[derive(Debug, Clone, Encode, Decode)]
pub struct Players {
    active_player_id: Option<PlayerId>,
    all_players: Map<PlayerId, Player>,
}

impl Players {
    pub fn empty(active_player_id: Option<PlayerId>) -> Self {
        Self {
            active_player_id,
            all_players: Default::default(),
        }
    }

    pub fn active(&self) -> Option<&Player> {
        self.active_player_id.and_then(|id| self.all_players.get(&id))
    }

    pub fn get(&self, player_id: PlayerId) -> &Player {
        self.all_players.get(&player_id).unwrap()
    }

    pub fn get_mut(&mut self, player_id: PlayerId) -> &mut Player {
        self.all_players.get_mut(&player_id).unwrap()
    }

    #[allow(dead_code)]
    pub fn get_all(&self) -> impl Iterator<Item = (&PlayerId, &Player)> {
        self.all_players.iter()
    }

    #[allow(dead_code)]
    pub fn get_all_mut(&mut self) -> impl Iterator<Item = (&PlayerId, &mut Player)> {
        self.all_players.iter_mut()
    }

    #[allow(dead_code)]
    pub fn get_all_in_world(&self, world_id: WorldId) -> impl Iterator<Item = (&PlayerId, &Player)> {
        self.all_players
            .iter()
            .filter(move |(_, player)| player.world_id() == world_id)
    }

    #[allow(dead_code)]
    pub fn get_all_in_world_mut(&mut self, world_id: WorldId) -> impl Iterator<Item = (&PlayerId, &mut Player)> {
        self.all_players
            .iter_mut()
            .filter(move |(_, player)| player.world_id() == world_id)
    }

    pub fn get_by_entity(&self, entity_id: Entity) -> Option<&Player> {
        self.all_players
            .iter()
            .find(|(_, player)| player.entity_id() == entity_id)
            .map(|(_, player)| player)
    }

    #[allow(dead_code)]
    pub fn get_by_entity_mut(&mut self, entity_id: Entity) -> Option<&mut Player> {
        self.all_players
            .iter_mut()
            .find(|(_, player)| player.entity_id() == entity_id)
            .map(|(_, player)| player)
    }

    pub fn add_player(&mut self, player: Player) {
        self.all_players.insert(player.id(), player);
    }

    pub fn remove_player(&mut self, player_id: PlayerId) -> Option<Player> {
        self.all_players.remove(&player_id)
    }
}

#[derive(Debug, Clone, Encode, Decode)]
pub struct UniversePlayers {
    pub default_world_id: WorldId,
    pub online_players: Map<PlayerId, WorldId>,
    pub offline_players: Map<PlayerId, (WorldId, Player)>,
}

impl UniversePlayers {
    pub fn new(default_world_id: WorldId) -> Self {
        Self {
            default_world_id,
            online_players: Map::default(),
            offline_players: Map::default(),
        }
    }

    pub fn player_joining(
        &mut self,
        player_info: &NetworkPlayerInfo,
        players: &mut Players,
        chunks: &mut Chunks,
        mobs: &mut Mobs,
    ) -> Option<Player> {
        if let Some((old_world_id, mut old_player)) = self.offline_players.remove(&player_info.id) {
            if old_world_id == mobs.world_id() {
                let old_mob = old_player
                    .offline_data
                    .take()
                    .expect("Offline data must exist when player is rejoining");
                let entity_id = mobs.add(old_mob.as_mob(), chunks);
                mobs.remove(old_player.entity_id, chunks);
                old_player.entity_id = entity_id;
                players.get_mut(old_player.id()).entity_id = entity_id;
                self.online_players.insert(player_info.id, mobs.world_id());
                Some(old_player)
            } else {
                self.offline_players.insert(old_player.id(), (old_world_id, old_player));
                None
            }
        } else if self.default_world_id == mobs.world_id() {
            let entity_id = mobs.add(create_player(Location::default()).as_mob(), chunks);
            self.online_players.insert(player_info.id, mobs.world_id());
            Some(Player::new(
                player_info.id,
                player_info.name.clone(),
                entity_id,
                mobs.world_id(),
            ))
        } else {
            None
        }
    }

    pub fn player_leaving(&mut self, mut player: Player, mobs: &mut Mobs) {
        if let Some(world_id) = self.online_players.remove(&player.id()) {
            assert_eq!(world_id, mobs.world_id());
            let offline_data = mobs.get(player.entity_id()).unwrap().clone();
            player.offline_data = Some(offline_data);
            self.offline_players.insert(player.id(), (world_id, player));
        }
    }
}

// ---------------------------------------------------------- //
// ---------------------- Save states ----------------------- //
// ---------------------------------------------------------- //

#[derive(Debug, Clone, Encode, Decode)]
pub(crate) struct PlayersSaveState {
    all_players: Map<PlayerId, Player>,
}

impl PlayersSaveState {
    pub fn into_actual(self, active_player_id: Option<PlayerId>) -> Players {
        Players {
            active_player_id,
            all_players: self.all_players,
        }
    }

    pub fn from_actual(actual: &Players, mobs: &Mobs) -> Self {
        let mut all_players = actual.all_players.clone();

        for (_, player) in all_players.iter_mut() {
            if player.offline_data.is_none() {
                player.offline_data = mobs.get(player.entity_id()).map(|mob| mob.clone());
            }
        }

        Self { all_players }
    }
}

#[derive(Debug, Clone, Encode, Decode)]
pub(crate) struct UniversePlayersSaveState {
    default_world_id: WorldId,
    all_players: Map<PlayerId, (WorldId, Player)>,
}

impl UniversePlayersSaveState {
    pub fn into_actual(self) -> UniversePlayers {
        UniversePlayers {
            default_world_id: self.default_world_id,
            online_players: Default::default(),
            offline_players: self.all_players,
        }
    }

    pub fn from_actual(actual: &Mutex<UniversePlayers>, worlds: &MutexGuard<Map<WorldId, World>>) -> Self {
        let actual = actual.lock().unwrap();
        let mut all_players = actual.offline_players.clone();

        for (world_id, world) in worlds.deref() {
            for (player_id, player) in &world.players.all_players {
                let mut player = player.clone();
                player.offline_data = world.mobs.get(player.entity_id()).map(|mob| mob.clone());
                all_players.insert(*player_id, (*world_id, player));
            }
        }

        Self {
            default_world_id: actual.default_world_id,
            all_players,
        }
    }
}
