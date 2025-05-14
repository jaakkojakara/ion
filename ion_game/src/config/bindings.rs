use bincode::{Decode, Encode};
use ion_common::Map;
use ion_engine::{KeyCode, core::world::CommandType};

// ---------------------------------------------------------- //
// ---------------------- Key bindings ---------------------- //
// ---------------------------------------------------------- //

// Usage not implemented yet.

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Encode, Decode)]
pub enum Command {
    RunUp,
    RunDown,
    RunLeft,
    RunRight,
}

impl CommandType for Command {}

pub fn default_key_bindings() -> Map<Command, KeyCode> {
    Map::from_iter(vec![
        (Command::RunUp, KeyCode::KeyW),
        (Command::RunDown, KeyCode::KeyS),
        (Command::RunLeft, KeyCode::KeyA),
        (Command::RunRight, KeyCode::KeyD),
    ])
}
