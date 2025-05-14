use ion_common::bincode::{Decode, Encode};
use ion_engine::core::{coordinates::ChunkLocation, world::WorldId};

use crate::universe::{
    camera::Camera,
    chunk::Chunks,
    creator::WorldCreator,
    entities::{mobs::Mobs, structures::Structure},
    players::Players,
    systems::{movement::Movement, navigation::Navigation},
};

pub mod movement;
pub mod navigation;

/// Contains all the systems that make up the logic of a game world.
pub struct Systems {
    world_id: WorldId,

    pub creator: WorldCreator,
    pub movement: Movement,
    pub navigation: Navigation,
}

impl Systems {
    pub fn new(world_id: WorldId, seed: u64) -> Self {
        Self {
            world_id,
            creator: WorldCreator::new(seed),
            movement: Movement::new(),
            navigation: Navigation::new(),
        }
    }

    pub fn update_camera(&self, camera: &mut Camera, players: &Players, mobs: &Mobs) {
        if let Some(player) = players.active() {
            camera.loc = *mobs.get(player.entity_id()).unwrap().loc;
        }
    }

    pub fn register_static(&mut self, _stat: Structure) {}

    pub fn update_systems(&mut self, mobs: &mut Mobs, chunks: &mut Chunks) {
        self.movement.update_movement(chunks, mobs);
        self.navigation.update_navigation(chunks);
    }

    /// Generates a chunk at the given location and updates all the systems.
    pub fn create_chunk(&mut self, loc: ChunkLocation, chunks: &mut Chunks) {
        chunks.create(loc, &mut self.creator);
        self.navigation.mark_dirty(loc);
    }

    /// Generates chunks around all players and updates all the systems.
    pub fn create_chunks(&mut self, players: &Players, chunks: &mut Chunks, mobs: &mut Mobs) {
        for (_, player) in players.get_all_in_world(self.world_id) {
            let player_mob = mobs.get(player.entity_id()).unwrap();
            for loc in ChunkLocation::chunks_around((*player_mob.loc).into(), 5) {
                if !chunks.exists(loc) {
                    self.create_chunk(loc, chunks);
                }
            }
        }
    }
}

// ---------------------------------------------------------- //
// ---------------------- Save states ----------------------- //
// ---------------------------------------------------------- //

#[derive(Debug, Clone, Encode, Decode)]
pub struct SystemsSaveState {
    pub world_id: WorldId,
    pub navigation: Navigation,
}

impl SystemsSaveState {
    pub fn into_actual(self, seed: u64) -> Systems {
        Systems {
            world_id: self.world_id,
            creator: WorldCreator::new(seed),
            movement: Movement::new(),
            navigation: self.navigation,
        }
    }

    pub fn from_actual(systems: &Systems) -> Self {
        Self {
            world_id: systems.world_id,
            navigation: systems.navigation.clone(),
        }
    }
}
