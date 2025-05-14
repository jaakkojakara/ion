use bincode::{Decode, Encode};
use ion_common::PlayerId;

use ion_engine::core::coordinates::{Direction, Location};
use ion_engine::core::world::ActionType;

pub(super) mod build_stateful;
pub(super) mod build_stateless;
pub(super) mod process_actions;

#[derive(Debug, Clone, PartialEq, Encode, Decode)]
pub enum Action {
    DebugSysEnabled(bool),
    DebugToggleChunkBorders,
    DebugToggleTileBorders,
    DebugToggleNoise1,
    DebugToggleFlowField,

    SetCameraScale(f32),
    SetLightingSun(f32),
    SetLightingAmbient(f32),
    Move { player_id: PlayerId, direction: Direction },
    NoMove { player_id: PlayerId },
    Shoot { loc: Location },
    SpawnMobs { loc: Location },
}

impl ActionType for Action {
    fn is_stateful(&self) -> bool {
        match self {
            Action::DebugToggleChunkBorders => false,
            Action::DebugToggleTileBorders => false,
            Action::DebugToggleNoise1 => false,
            Action::DebugToggleFlowField => false,
            Action::SetCameraScale(_) => false,
            Action::SetLightingSun(_) => false,
            Action::SetLightingAmbient(_) => false,
            Action::Shoot { loc: _ } => true,
            _ => true,
        }
    }
}
