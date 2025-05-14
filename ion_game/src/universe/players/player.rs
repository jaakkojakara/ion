use bincode::{Decode, Encode};
use ion_common::PlayerId;

use ion_engine::KeyCode;
use ion_engine::core::coordinates::Direction;
use ion_engine::core::world::WorldId;
use ion_engine::input::input_state::InputState;

use crate::config::bindings::Command;
use crate::universe::actions::Action;
use crate::universe::entities::Entity;
use crate::universe::entities::mobs::MobClone;

#[derive(Debug, Clone, Encode, Decode)]
pub struct Player {
    id: PlayerId,
    name: String,

    pub(super) world_id: WorldId,
    pub(super) entity_id: Entity,
    pub(super) offline_data: Option<MobClone>,

    pub move_speed: f32,
}

impl Player {
    pub fn new(id: PlayerId, name: String, entity_id: Entity, world_id: WorldId) -> Self {
        Self {
            id,
            name,

            entity_id,
            world_id,
            offline_data: None,

            move_speed: 0.07,
        }
    }

    pub fn id(&self) -> PlayerId {
        self.id
    }

    #[allow(dead_code)]
    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn entity_id(&self) -> Entity {
        self.entity_id
    }

    pub fn world_id(&self) -> WorldId {
        self.world_id
    }

    pub fn build_movement_action(&self, input: &InputState<Command>) -> Action {
        let player_id = self.id;
        if input.is_key_active(KeyCode::KeyW) && input.is_key_active(KeyCode::KeyD) {
            Action::Move {
                player_id,
                direction: Direction::E,
            }
        } else if input.is_key_active(KeyCode::KeyW) && input.is_key_active(KeyCode::KeyA) {
            Action::Move {
                player_id,
                direction: Direction::N,
            }
        } else if input.is_key_active(KeyCode::KeyS) && input.is_key_active(KeyCode::KeyD) {
            Action::Move {
                player_id,
                direction: Direction::S,
            }
        } else if input.is_key_active(KeyCode::KeyS) && input.is_key_active(KeyCode::KeyA) {
            Action::Move {
                player_id,
                direction: Direction::W,
            }
        } else if input.is_key_active(KeyCode::KeyW) {
            Action::Move {
                player_id,
                direction: Direction::NE,
            }
        } else if input.is_key_active(KeyCode::KeyD) {
            Action::Move {
                player_id,
                direction: Direction::SE,
            }
        } else if input.is_key_active(KeyCode::KeyA) {
            Action::Move {
                player_id,
                direction: Direction::NW,
            }
        } else if input.is_key_active(KeyCode::KeyS) {
            Action::Move {
                player_id,
                direction: Direction::SW,
            }
        } else {
            Action::NoMove { player_id }
        }
    }
}
