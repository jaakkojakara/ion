use ion_engine::KeyCode;
use ion_engine::input::input_state::InputState;

use crate::config::bindings::Command;
use crate::universe::world::World;

use super::Action;

pub fn build_stateful_actions(input: &InputState<Command>, is_active_world: bool, world: &World) -> Vec<Action> {
    let mut actions = Vec::new();

    if is_active_world {
        if let Some(player) = world.players.active() {
            actions.push(player.build_movement_action(input));
        }

        if input.is_key_just_pressed(KeyCode::KeyZ) {
            actions.push(Action::SpawnMobs {
                loc: input.cursor_location().into(),
            });
        }

        if input.is_key_just_pressed(KeyCode::Space) {
            actions.push(Action::Shoot {
                loc: input.cursor_location(),
            });
        }
    }

    actions
}
