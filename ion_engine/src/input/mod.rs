use std::cell::Cell;
use std::sync::mpsc;
use std::sync::mpsc::Sender;

use winit::event::{ElementState, MouseButton, MouseScrollDelta, WindowEvent};
use winit::keyboard::PhysicalKey::Code;
use winit::keyboard::{Key, KeyCode};

use crate::core::coordinates::{Location, Position};
use crate::core::world::CommandType;
use crate::input::input_state::InputState;
use std::fmt::Debug;

pub mod input_state;

/// A key binding for a command.
/// Binds a command to a key or a combination of keys.
/// If key_2 is None, the command is executed if key_1 is pressed.
/// If key_2 is Some, the command is only executed if both keys are pressed.
#[derive(Debug, Clone, PartialEq)]
pub struct KeyBind<C: CommandType> {
    command: C,
    key_1: Option<KeyCode>,
    key_2: Option<KeyCode>,
    mouse_button: Option<MouseButton>,
}

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub(crate) enum InputEvent<C: CommandType> {
    KeyPressed(KeyCode, Option<char>),
    KeyReleased(KeyCode, Option<char>),
    MouseButtonPressed(MouseButton),
    MouseButtonReleased(MouseButton),
    MouseScrollChange(f32),
    CursorLocPosChange(Location, Position),
    CameraMoved(f32, f32),
    SetKeyBind(KeyBind<C>),
    RemoveKeyBind(C),
}

pub struct Input<C: CommandType> {
    input_event_sender_ui: Sender<InputEvent<C>>,
    input_event_sender_universe: Sender<InputEvent<C>>,

    input_state_ui: Cell<Option<InputState<C>>>,
    input_state_universe: Cell<Option<InputState<C>>>,
}

#[allow(clippy::new_without_default)]
impl<C: CommandType> Input<C> {
    pub fn new() -> Input<C> {
        let (input_event_sender_ui, input_event_receiver_ui) = mpsc::channel::<InputEvent<C>>();
        let (input_event_sender_universe, input_event_receiver_universe) = mpsc::channel::<InputEvent<C>>();

        let input_state_ui = Cell::new(Some(InputState::new(
            input_event_receiver_ui,
            [input_event_sender_ui.clone(), input_event_sender_universe.clone()],
        )));
        let input_state_universe = Cell::new(Some(InputState::new(
            input_event_receiver_universe,
            [input_event_sender_ui.clone(), input_event_sender_universe.clone()],
        )));

        Self {
            input_event_sender_ui,
            input_event_sender_universe,

            input_state_ui,
            input_state_universe,
        }
    }

    pub(crate) fn input_state_ui(&self) -> InputState<C> {
        self.input_state_ui
            .take()
            .expect("Only one input state for ui exists. This should be called only once.")
    }

    pub(crate) fn input_state_universe(&self) -> InputState<C> {
        self.input_state_universe
            .take()
            .expect("Only one input state for universe exists. This should be called only once.")
    }

    pub(crate) fn handle_keyboard_event(&self, event: &WindowEvent) {
        if let WindowEvent::KeyboardInput {
            event, is_synthetic, ..
        } = event
        {
            if !is_synthetic && !event.repeat {
                let char_opt = match &event.logical_key {
                    Key::Character(str) => str.as_str().chars().next(),
                    _ => None,
                };
                match event.state {
                    ElementState::Pressed => {
                        if let Code(keycode) = event.physical_key {
                            self.input_event_sender_ui
                                .send(InputEvent::KeyPressed(keycode, char_opt))
                                .unwrap();

                            self.input_event_sender_universe
                                .send(InputEvent::KeyPressed(keycode, char_opt))
                                .unwrap();
                        }
                    }
                    ElementState::Released => {
                        if let Code(keycode) = event.physical_key {
                            self.input_event_sender_ui
                                .send(InputEvent::KeyReleased(keycode, char_opt))
                                .unwrap();
                            self.input_event_sender_universe
                                .send(InputEvent::KeyReleased(keycode, char_opt))
                                .unwrap();
                        }
                    }
                }
            }
        }
    }

    pub(crate) fn handle_mouse_button_event(&self, event: &WindowEvent) {
        if let WindowEvent::MouseInput { state, button, .. } = event {
            match state {
                ElementState::Pressed => {
                    self.input_event_sender_ui
                        .send(InputEvent::MouseButtonPressed(*button))
                        .unwrap();
                    self.input_event_sender_universe
                        .send(InputEvent::MouseButtonPressed(*button))
                        .unwrap();
                }
                ElementState::Released => {
                    self.input_event_sender_ui
                        .send(InputEvent::MouseButtonReleased(*button))
                        .unwrap();

                    self.input_event_sender_universe
                        .send(InputEvent::MouseButtonReleased(*button))
                        .unwrap();
                }
            }
        }
    }

    pub(crate) fn handle_mouse_scroll_event(&self, event: &WindowEvent) {
        if let WindowEvent::MouseWheel { delta, .. } = event {
            let delta = match delta {
                MouseScrollDelta::LineDelta(_, value) => *value / -10.,
                MouseScrollDelta::PixelDelta(value) => value.y as f32 / 1000.,
            };

            self.input_event_sender_ui
                .send(InputEvent::MouseScrollChange(delta))
                .unwrap();
            self.input_event_sender_universe
                .send(InputEvent::MouseScrollChange(delta))
                .unwrap();
        }
    }

    pub(crate) fn handle_mouse_move_event(&self, loc: Location, pos: Position) {
        self.input_event_sender_ui
            .send(InputEvent::CursorLocPosChange(loc, pos))
            .unwrap();

        self.input_event_sender_universe
            .send(InputEvent::CursorLocPosChange(loc, pos))
            .unwrap();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bincode::{Decode, Encode};

    // Test command type for testing
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Encode, Decode)]
    enum TestCommand {
        MoveUp,
        MoveDown,
        Attack,
        Menu,
    }
    impl CommandType for TestCommand {}

    #[test]
    fn can_create_input() {
        let _input = Input::<TestCommand>::new();
        // Should not panic
    }

    #[test]
    fn can_retrieve_input_states_once() {
        let input = Input::<TestCommand>::new();

        // Should be able to retrieve each state once
        let _ui_state = input.input_state_ui();
        let _universe_state = input.input_state_universe();
    }

    #[test]
    #[should_panic(expected = "Only one input state for ui exists")]
    fn cannot_retrieve_ui_state_twice() {
        let input = Input::<TestCommand>::new();
        let _first = input.input_state_ui();
        let _second = input.input_state_ui(); // Should panic
    }

    #[test]
    #[should_panic(expected = "Only one input state for universe exists")]
    fn cannot_retrieve_universe_state_twice() {
        let input = Input::<TestCommand>::new();
        let _first = input.input_state_universe();
        let _second = input.input_state_universe(); // Should panic
    }

    #[test]
    fn mouse_move_handled_correctly() {
        let input = Input::<TestCommand>::new();
        let mut ui_state = input.input_state_ui();
        let mut universe_state = input.input_state_universe();

        let loc = Location { x: 100.0, y: 200.0 };
        let pos = Position { x: 150.0, y: 250.0 };

        input.handle_mouse_move_event(loc, pos);

        // Process events in both states
        ui_state.handle_received_input_events();
        universe_state.handle_received_input_events();

        // Both states should register the cursor location
        assert_eq!(ui_state.cursor_location(), loc);
        assert_eq!(universe_state.cursor_location(), loc);
    }

    #[test]
    fn input_event_cloning_works() {
        let event1 = InputEvent::<TestCommand>::KeyPressed(KeyCode::KeyA, Some('a'));
        let event2 = event1.clone();

        match (&event1, &event2) {
            (InputEvent::KeyPressed(key1, char1), InputEvent::KeyPressed(key2, char2)) => {
                assert_eq!(key1, key2);
                assert_eq!(char1, char2);
            }
            _ => panic!("Event cloning failed"),
        }
    }

    #[test]
    fn input_event_debug_formatting() {
        let event = InputEvent::<TestCommand>::KeyPressed(KeyCode::KeyA, Some('a'));
        let debug_str = format!("{:?}", event);
        assert!(debug_str.contains("KeyPressed"));
        assert!(debug_str.contains("KeyA"));
    }

    #[test]
    fn input_creation_successful() {
        // Simple smoke test for Input creation and basic operations
        let input = Input::<TestCommand>::new();
        let loc = Location { x: 100.0, y: 200.0 };
        let pos = Position { x: 150.0, y: 250.0 };

        // These should not panic
        input.handle_mouse_move_event(loc, pos);

        // Can retrieve states
        let _ui_state = input.input_state_ui();
        let _universe_state = input.input_state_universe();
    }
}
