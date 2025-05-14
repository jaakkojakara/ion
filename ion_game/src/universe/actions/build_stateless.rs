use ion_engine::MouseButton;
use ion_engine::core::coordinates::{ChunkLocation, TileLocation};
use ion_engine::input::input_state::InputState;

use crate::config::bindings::Command;
use crate::universe::world::World;

use super::Action;

pub fn build_stateless_actions(input: &InputState<Command>, is_active_world: bool, world: &World) -> Vec<Action> {
    if input.is_button_just_pressed(MouseButton::Left) {
        println!(
            "{:?} - {:?} - {:?}",
            input.cursor_location(),
            TileLocation::from(input.cursor_location()),
            ChunkLocation::from(TileLocation::from(input.cursor_location()))
        );
    }

    let mut actions = Vec::new();

    if is_active_world && input.mouse_scroll_delta() != 0.0 {
        actions.push(Action::SetCameraScale(
            (world.camera.scale + input.mouse_scroll_delta() * 3.0).max(0.02),
        ))
    }

    actions
}
