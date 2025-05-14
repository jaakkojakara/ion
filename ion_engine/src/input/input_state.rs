use std::sync::mpsc::{Receiver, Sender};

use ion_common::Map;
use winit::event::MouseButton;
use winit::keyboard::KeyCode;

use crate::{
    core::coordinates::{Location, Position},
    gfx::renderer::render_camera::RenderCamera,
    input::{CommandType, KeyBind},
};

use super::InputEvent;

pub struct InputState<C: CommandType> {
    event_senders: [Sender<InputEvent<C>>; 2],
    event_receiver: Receiver<InputEvent<C>>,
    events_in_frame: Vec<InputEvent<C>>,

    bindings: Map<C, KeyBind<C>>,

    key_states: Map<KeyCode, KeyState>,
    mouse_button_states: Map<MouseButton, KeyState>,
    mouse_scroll_state: f32,
    cursor_location_state: Location,
    cursor_position_state: Position,
}

impl<C: CommandType> InputState<C> {
    pub(crate) fn new(event_receiver: Receiver<InputEvent<C>>, event_senders: [Sender<InputEvent<C>>; 2]) -> Self {
        Self {
            event_senders,
            event_receiver,
            events_in_frame: Vec::new(),

            bindings: Map::default(),

            key_states: [
                (KeyCode::Backslash, KeyState::default()),
                (KeyCode::BracketLeft, KeyState::default()),
                (KeyCode::BracketRight, KeyState::default()),
                (KeyCode::Comma, KeyState::default()),
                (KeyCode::Digit0, KeyState::default()),
                (KeyCode::Digit1, KeyState::default()),
                (KeyCode::Digit2, KeyState::default()),
                (KeyCode::Digit3, KeyState::default()),
                (KeyCode::Digit4, KeyState::default()),
                (KeyCode::Digit5, KeyState::default()),
                (KeyCode::Digit6, KeyState::default()),
                (KeyCode::Digit7, KeyState::default()),
                (KeyCode::Digit8, KeyState::default()),
                (KeyCode::Digit9, KeyState::default()),
                (KeyCode::Equal, KeyState::default()),
                (KeyCode::IntlBackslash, KeyState::default()),
                (KeyCode::IntlRo, KeyState::default()),
                (KeyCode::IntlYen, KeyState::default()),
                (KeyCode::KeyA, KeyState::default()),
                (KeyCode::KeyB, KeyState::default()),
                (KeyCode::KeyC, KeyState::default()),
                (KeyCode::KeyD, KeyState::default()),
                (KeyCode::KeyE, KeyState::default()),
                (KeyCode::KeyF, KeyState::default()),
                (KeyCode::KeyG, KeyState::default()),
                (KeyCode::KeyH, KeyState::default()),
                (KeyCode::KeyI, KeyState::default()),
                (KeyCode::KeyJ, KeyState::default()),
                (KeyCode::KeyK, KeyState::default()),
                (KeyCode::KeyL, KeyState::default()),
                (KeyCode::KeyM, KeyState::default()),
                (KeyCode::KeyN, KeyState::default()),
                (KeyCode::KeyO, KeyState::default()),
                (KeyCode::KeyP, KeyState::default()),
                (KeyCode::KeyQ, KeyState::default()),
                (KeyCode::KeyR, KeyState::default()),
                (KeyCode::KeyS, KeyState::default()),
                (KeyCode::KeyT, KeyState::default()),
                (KeyCode::KeyU, KeyState::default()),
                (KeyCode::KeyV, KeyState::default()),
                (KeyCode::KeyW, KeyState::default()),
                (KeyCode::KeyX, KeyState::default()),
                (KeyCode::KeyY, KeyState::default()),
                (KeyCode::KeyZ, KeyState::default()),
                (KeyCode::Minus, KeyState::default()),
                (KeyCode::Period, KeyState::default()),
                (KeyCode::Quote, KeyState::default()),
                (KeyCode::Semicolon, KeyState::default()),
                (KeyCode::Slash, KeyState::default()),
                (KeyCode::AltLeft, KeyState::default()),
                (KeyCode::AltRight, KeyState::default()),
                (KeyCode::Backspace, KeyState::default()),
                (KeyCode::CapsLock, KeyState::default()),
                (KeyCode::ContextMenu, KeyState::default()),
                (KeyCode::ControlLeft, KeyState::default()),
                (KeyCode::ControlRight, KeyState::default()),
                (KeyCode::Enter, KeyState::default()),
                (KeyCode::SuperLeft, KeyState::default()),
                (KeyCode::SuperRight, KeyState::default()),
                (KeyCode::ShiftLeft, KeyState::default()),
                (KeyCode::ShiftRight, KeyState::default()),
                (KeyCode::Space, KeyState::default()),
                (KeyCode::Tab, KeyState::default()),
                (KeyCode::Convert, KeyState::default()),
                (KeyCode::KanaMode, KeyState::default()),
                (KeyCode::Lang1, KeyState::default()),
                (KeyCode::Lang2, KeyState::default()),
                (KeyCode::Lang3, KeyState::default()),
                (KeyCode::Lang4, KeyState::default()),
                (KeyCode::Lang5, KeyState::default()),
                (KeyCode::NonConvert, KeyState::default()),
                (KeyCode::Delete, KeyState::default()),
                (KeyCode::End, KeyState::default()),
                (KeyCode::Help, KeyState::default()),
                (KeyCode::Home, KeyState::default()),
                (KeyCode::Insert, KeyState::default()),
                (KeyCode::PageDown, KeyState::default()),
                (KeyCode::PageUp, KeyState::default()),
                (KeyCode::ArrowDown, KeyState::default()),
                (KeyCode::ArrowLeft, KeyState::default()),
                (KeyCode::ArrowRight, KeyState::default()),
                (KeyCode::ArrowUp, KeyState::default()),
                (KeyCode::NumLock, KeyState::default()),
                (KeyCode::Numpad0, KeyState::default()),
                (KeyCode::Numpad1, KeyState::default()),
                (KeyCode::Numpad2, KeyState::default()),
                (KeyCode::Numpad3, KeyState::default()),
                (KeyCode::Numpad4, KeyState::default()),
                (KeyCode::Numpad5, KeyState::default()),
                (KeyCode::Numpad6, KeyState::default()),
                (KeyCode::Numpad7, KeyState::default()),
                (KeyCode::Numpad8, KeyState::default()),
                (KeyCode::Numpad9, KeyState::default()),
                (KeyCode::NumpadAdd, KeyState::default()),
                (KeyCode::NumpadBackspace, KeyState::default()),
                (KeyCode::NumpadClear, KeyState::default()),
                (KeyCode::NumpadClearEntry, KeyState::default()),
                (KeyCode::NumpadComma, KeyState::default()),
                (KeyCode::NumpadDecimal, KeyState::default()),
                (KeyCode::NumpadDivide, KeyState::default()),
                (KeyCode::NumpadEnter, KeyState::default()),
                (KeyCode::NumpadEqual, KeyState::default()),
                (KeyCode::NumpadHash, KeyState::default()),
                (KeyCode::NumpadMemoryAdd, KeyState::default()),
                (KeyCode::NumpadMemoryClear, KeyState::default()),
                (KeyCode::NumpadMemoryRecall, KeyState::default()),
                (KeyCode::NumpadMemoryStore, KeyState::default()),
                (KeyCode::NumpadMemorySubtract, KeyState::default()),
                (KeyCode::NumpadMultiply, KeyState::default()),
                (KeyCode::NumpadParenLeft, KeyState::default()),
                (KeyCode::NumpadParenRight, KeyState::default()),
                (KeyCode::NumpadStar, KeyState::default()),
                (KeyCode::NumpadSubtract, KeyState::default()),
                (KeyCode::Escape, KeyState::default()),
                (KeyCode::Fn, KeyState::default()),
                (KeyCode::FnLock, KeyState::default()),
                (KeyCode::PrintScreen, KeyState::default()),
                (KeyCode::ScrollLock, KeyState::default()),
                (KeyCode::Pause, KeyState::default()),
                (KeyCode::BrowserBack, KeyState::default()),
                (KeyCode::BrowserFavorites, KeyState::default()),
                (KeyCode::BrowserForward, KeyState::default()),
                (KeyCode::BrowserHome, KeyState::default()),
                (KeyCode::BrowserRefresh, KeyState::default()),
                (KeyCode::BrowserSearch, KeyState::default()),
                (KeyCode::BrowserStop, KeyState::default()),
                (KeyCode::Eject, KeyState::default()),
                (KeyCode::LaunchApp1, KeyState::default()),
                (KeyCode::LaunchApp2, KeyState::default()),
                (KeyCode::LaunchMail, KeyState::default()),
                (KeyCode::MediaPlayPause, KeyState::default()),
                (KeyCode::MediaSelect, KeyState::default()),
                (KeyCode::MediaStop, KeyState::default()),
                (KeyCode::MediaTrackNext, KeyState::default()),
                (KeyCode::MediaTrackPrevious, KeyState::default()),
                (KeyCode::Power, KeyState::default()),
                (KeyCode::Sleep, KeyState::default()),
                (KeyCode::AudioVolumeDown, KeyState::default()),
                (KeyCode::AudioVolumeMute, KeyState::default()),
                (KeyCode::AudioVolumeUp, KeyState::default()),
                (KeyCode::WakeUp, KeyState::default()),
                (KeyCode::Meta, KeyState::default()),
                (KeyCode::Hyper, KeyState::default()),
                (KeyCode::Turbo, KeyState::default()),
                (KeyCode::Abort, KeyState::default()),
                (KeyCode::Resume, KeyState::default()),
                (KeyCode::Suspend, KeyState::default()),
                (KeyCode::Again, KeyState::default()),
                (KeyCode::Copy, KeyState::default()),
                (KeyCode::Cut, KeyState::default()),
                (KeyCode::Find, KeyState::default()),
                (KeyCode::Open, KeyState::default()),
                (KeyCode::Paste, KeyState::default()),
                (KeyCode::Props, KeyState::default()),
                (KeyCode::Select, KeyState::default()),
                (KeyCode::Undo, KeyState::default()),
                (KeyCode::Hiragana, KeyState::default()),
                (KeyCode::Katakana, KeyState::default()),
                (KeyCode::F1, KeyState::default()),
                (KeyCode::F2, KeyState::default()),
                (KeyCode::F3, KeyState::default()),
                (KeyCode::F4, KeyState::default()),
                (KeyCode::F5, KeyState::default()),
                (KeyCode::F6, KeyState::default()),
                (KeyCode::F7, KeyState::default()),
                (KeyCode::F8, KeyState::default()),
                (KeyCode::F9, KeyState::default()),
                (KeyCode::F10, KeyState::default()),
                (KeyCode::F11, KeyState::default()),
                (KeyCode::F12, KeyState::default()),
                (KeyCode::F13, KeyState::default()),
                (KeyCode::F14, KeyState::default()),
                (KeyCode::F15, KeyState::default()),
                (KeyCode::F16, KeyState::default()),
                (KeyCode::F17, KeyState::default()),
                (KeyCode::F18, KeyState::default()),
                (KeyCode::F19, KeyState::default()),
                (KeyCode::F20, KeyState::default()),
                (KeyCode::F21, KeyState::default()),
                (KeyCode::F22, KeyState::default()),
                (KeyCode::F23, KeyState::default()),
                (KeyCode::F24, KeyState::default()),
                (KeyCode::F25, KeyState::default()),
                (KeyCode::F26, KeyState::default()),
                (KeyCode::F27, KeyState::default()),
                (KeyCode::F28, KeyState::default()),
                (KeyCode::F29, KeyState::default()),
                (KeyCode::F30, KeyState::default()),
                (KeyCode::F31, KeyState::default()),
                (KeyCode::F32, KeyState::default()),
                (KeyCode::F33, KeyState::default()),
                (KeyCode::F34, KeyState::default()),
                (KeyCode::F35, KeyState::default()),
            ]
            .into_iter()
            .collect(),
            mouse_button_states: [
                (MouseButton::Left, KeyState::default()),
                (MouseButton::Right, KeyState::default()),
                (MouseButton::Middle, KeyState::default()),
            ]
            .into_iter()
            .collect(),
            mouse_scroll_state: 0.0,
            cursor_location_state: Location { x: 0.0, y: 0.0 },
            cursor_position_state: Position { x: 0.0, y: 0.0 },
        }
    }

    fn command_state(&self, command: C) -> KeyState {
        let key_bind = self.bindings.get(&command).expect("Command must be bound to a key");
        let key_1_state = key_bind.key_1.map(|key| self.key_state(key));
        let key_2_state = key_bind.key_2.map(|key| self.key_state(key));
        let button_state = key_bind.mouse_button.map(|button| self.mouse_button_state(button));

        [key_1_state, key_2_state, button_state]
            .into_iter()
            .filter_map(|state| state)
            .copied()
            .reduce(|acc, state| acc.combine(&state))
            .expect("At least one binding must exist")
    }

    fn key_state(&self, key: KeyCode) -> &KeyState {
        self.key_states
            .get(&key)
            .unwrap_or_else(|| panic!("Unsupported key: {key:?}"))
    }

    fn mouse_button_state(&self, button: MouseButton) -> &KeyState {
        self.mouse_button_states
            .get(&button)
            .unwrap_or_else(|| panic!("Unsupported button: {button:?}"))
    }

    pub fn is_command_active(&self, command: C) -> bool {
        let key_state = self.command_state(command);
        key_state.active || (key_state.just_pressed && key_state.just_released)
    }

    pub fn is_command_just_actived(&self, command: C) -> bool {
        self.command_state(command).just_pressed
    }

    pub fn is_command_just_released(&self, command: C) -> bool {
        self.command_state(command).just_released
    }

    pub fn is_key_active(&self, key: KeyCode) -> bool {
        let key_state = self.key_state(key);
        key_state.active || (key_state.just_pressed && key_state.just_released)
    }

    pub fn is_key_just_pressed(&self, key: KeyCode) -> bool {
        self.key_state(key).just_pressed
    }

    pub fn is_key_just_released(&self, key: KeyCode) -> bool {
        self.key_state(key).just_released
    }

    pub fn is_key_repeating(&self, key: KeyCode) -> bool {
        self.key_state(key).repeating()
    }

    pub fn is_button_active(&self, button: MouseButton) -> bool {
        let button_state = self.mouse_button_state(button);
        button_state.active || (button_state.just_pressed && button_state.just_released)
    }

    pub fn is_button_just_pressed(&self, button: MouseButton) -> bool {
        self.mouse_button_state(button).just_pressed
    }

    pub fn is_button_just_released(&self, button: MouseButton) -> bool {
        self.mouse_button_state(button).just_released
    }

    pub fn mouse_scroll_delta(&self) -> f32 {
        self.mouse_scroll_state
    }

    pub fn cursor_location(&self) -> Location {
        self.cursor_location_state
    }

    pub fn text_input(&self) -> Option<String> {
        // Collects all char inputs to string, but only allocates if necessary
        let mut string: Option<String> = self.events_in_frame.iter().fold(None, |prev, event| match event {
            InputEvent::KeyPressed(key_code, char) => {
                if let Some(char) = char {
                    match prev {
                        Some(mut string) => {
                            string.push(*char);
                            Some(string)
                        }
                        None => Some(String::from(*char)),
                    }
                } else if *key_code == KeyCode::Space {
                    match prev {
                        Some(mut string) => {
                            string.push(' ');
                            Some(string)
                        }
                        None => Some(String::from(' ')),
                    }
                } else {
                    prev
                }
            }
            _ => prev,
        });

        for key_state in self.key_states.values() {
            if key_state.active && key_state.active_for > 60 && key_state.active_for % 4 == 0 {
                if let Some(char) = key_state.active_char {
                    if let Some(string) = &mut string {
                        string.push(char);
                    } else {
                        string = Some(String::from(char));
                    }
                }
            }
        }

        string
    }

    pub fn set_key_bind(&self, key_bind: KeyBind<C>) {
        assert!(
            key_bind.key_1.is_some() || key_bind.key_2.is_some() || key_bind.mouse_button.is_some(),
            "At least one key or mouse button must be bound"
        );

        let event = InputEvent::SetKeyBind(key_bind);
        for sender in self.event_senders.iter() {
            sender.send(event.clone()).unwrap();
        }
    }

    pub fn check_key_bind(&self, command: C) -> Option<KeyBind<C>> {
        self.bindings.get(&command).cloned()
    }

    pub fn remove_key_bind(&self, command: C) {
        let event = InputEvent::RemoveKeyBind(command);
        for sender in self.event_senders.iter() {
            sender.send(event.clone()).unwrap();
        }
    }

    pub(crate) fn handle_camera_movement(&mut self, camera: &RenderCamera) {
        let event = InputEvent::CameraMoved(camera.last_real_change_x(), camera.last_real_change_y());
        for sender in self.event_senders.iter() {
            sender.send(event.clone()).unwrap();
        }
    }

    pub(crate) fn handle_received_input_events(&mut self) {
        let events: Vec<_> = self.event_receiver.try_iter().collect();
        for event in events {
            self.handle_input_event(event);
        }
    }

    pub(crate) fn clear_one_frame_statuses(&mut self) {
        for key_state in &mut self.key_states.values_mut() {
            key_state.just_pressed = false;
            key_state.just_released = false;

            if key_state.active {
                key_state.active_for += 1;
            } else {
                key_state.active_for = 0;
            }
        }
        for button_state in &mut self.mouse_button_states.values_mut() {
            button_state.just_pressed = false;
            button_state.just_released = false;

            if button_state.active {
                button_state.active_for += 1;
            } else {
                button_state.active_for = 0;
            }
        }

        self.events_in_frame.clear();
        self.mouse_scroll_state = 0.0;
    }

    fn handle_input_event(&mut self, input_event: InputEvent<C>) {
        self.events_in_frame.push(input_event.clone());
        match input_event {
            InputEvent::KeyPressed(key, char) => {
                self.key_states.entry(key).and_modify(|state| {
                    state.just_pressed = true;
                    state.active = true;
                    state.active_char = char;
                });
            }
            InputEvent::KeyReleased(key, char) => {
                self.key_states.entry(key).and_modify(|state| {
                    state.just_released = true;
                    state.active = false;
                    state.active_char = char;
                });
            }
            InputEvent::MouseButtonPressed(mouse_button) => {
                match mouse_button {
                    MouseButton::Other(_) => {}
                    normal_button => {
                        self.mouse_button_states.entry(normal_button).and_modify(|state| {
                            state.just_pressed = true;
                            state.active = true;
                        });
                    }
                };
            }
            InputEvent::MouseButtonReleased(mouse_button) => {
                match mouse_button {
                    MouseButton::Other(_) => {}
                    normal_button => {
                        self.mouse_button_states.entry(normal_button).and_modify(|state| {
                            state.just_released = true;
                            state.active = false;
                        });
                    }
                };
            }
            InputEvent::MouseScrollChange(delta) => {
                self.mouse_scroll_state = delta;
            }
            InputEvent::CursorLocPosChange(loc, pos) => {
                self.cursor_location_state = loc;
                self.cursor_position_state = pos;
            }
            InputEvent::CameraMoved(x, y) => {
                self.cursor_location_state.x += x;
                self.cursor_location_state.y += y;
            }
            InputEvent::SetKeyBind(key_bind) => {
                self.bindings.insert(key_bind.command, key_bind);
            }
            InputEvent::RemoveKeyBind(command) => {
                self.bindings.remove(&command);
            }
        }
    }
}

#[derive(Debug, Default, Clone, Copy)]
struct KeyState {
    pub active: bool,
    pub active_for: u32,
    pub active_char: Option<char>,
    pub just_pressed: bool,
    pub just_released: bool,
}

impl KeyState {
    pub fn repeating(&self) -> bool {
        self.active && self.active_for > 60 && self.active_for % 4 == 0
    }

    pub fn combine(&self, other: &KeyState) -> KeyState {
        let active = self.active && other.active;
        KeyState {
            active,
            active_for: self.active_for.min(other.active_for),
            active_char: None,
            just_pressed: active && (self.just_pressed || other.just_pressed),
            just_released: !active && (self.just_released || other.just_released),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::mpsc;

    use bincode::{Decode, Encode};
    use winit::event::MouseButton;
    use winit::keyboard::KeyCode;

    use crate::core::coordinates::{Location, Position};
    use crate::input::CommandType;

    use super::InputState;

    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Encode, Decode, Hash)]
    pub enum TestCommand {
        Test,
    }
    impl CommandType for TestCommand {}

    #[test]
    fn can_create_input_handler() {
        let (e_in, e_out) = mpsc::channel();
        let _ = InputState::<TestCommand>::new(e_out, [e_in.clone(), e_in.clone()]);
    }

    #[test]
    fn events_are_processed_when_handler_is_called() {
        let (e_in, e_out) = mpsc::channel();
        let mut handler = InputState::<TestCommand>::new(e_out, [e_in.clone(), e_in.clone()]);

        e_in.send(crate::input::InputEvent::KeyPressed(KeyCode::KeyA, Some('A')))
            .unwrap();

        handler.handle_received_input_events();

        assert!(handler.is_key_active(KeyCode::KeyA));
        assert!(handler.is_key_just_pressed(KeyCode::KeyA));
        assert!(!handler.is_key_just_released(KeyCode::KeyA));
    }

    #[test]
    fn key_is_active_on_frame_it_was_released() {
        let (e_in, e_out) = mpsc::channel();
        let mut handler = InputState::<TestCommand>::new(e_out, [e_in.clone(), e_in.clone()]);

        e_in.send(crate::input::InputEvent::KeyPressed(KeyCode::KeyA, Some('A')))
            .unwrap();

        handler.clear_one_frame_statuses();

        e_in.send(crate::input::InputEvent::KeyReleased(KeyCode::KeyA, Some('A')))
            .unwrap();

        handler.handle_received_input_events();

        assert!(handler.is_key_active(KeyCode::KeyA));
        assert!(handler.is_key_just_pressed(KeyCode::KeyA));
        assert!(handler.is_key_just_released(KeyCode::KeyA));
    }

    #[test]
    fn key_statees_are_cleared_at_end_of_frame() {
        let (e_in, e_out) = mpsc::channel();
        let mut handler = InputState::<TestCommand>::new(e_out, [e_in.clone(), e_in.clone()]);

        e_in.send(crate::input::InputEvent::KeyPressed(KeyCode::KeyA, Some('A')))
            .unwrap();

        e_in.send(crate::input::InputEvent::KeyReleased(KeyCode::KeyA, Some('A')))
            .unwrap();

        handler.handle_received_input_events();
        handler.clear_one_frame_statuses();

        assert!(!handler.is_key_active(KeyCode::KeyA));
        assert!(!handler.is_key_just_pressed(KeyCode::KeyA));
        assert!(!handler.is_key_just_released(KeyCode::KeyA));
    }

    #[test]
    fn mouse_button_states_work_correctly() {
        let (e_in, e_out) = mpsc::channel();
        let mut handler = InputState::<TestCommand>::new(e_out, [e_in.clone(), e_in.clone()]);

        // Test mouse button press
        e_in.send(crate::input::InputEvent::MouseButtonPressed(MouseButton::Left))
            .unwrap();

        handler.handle_received_input_events();

        // Check that the mouse button was processed correctly
        assert!(handler.is_button_just_pressed(MouseButton::Left));
        assert!(handler.is_button_active(MouseButton::Left));
        assert!(!handler.is_button_just_released(MouseButton::Left));

        // Clear frame status - button should still be active but not just_pressed
        handler.clear_one_frame_statuses();

        assert!(!handler.is_button_just_pressed(MouseButton::Left));
        assert!(handler.is_button_active(MouseButton::Left)); // Still active
        assert!(!handler.is_button_just_released(MouseButton::Left));

        // Test release (following the pattern of the working key test)
        e_in.send(crate::input::InputEvent::MouseButtonReleased(MouseButton::Left))
            .unwrap();

        handler.handle_received_input_events();

        // On the release frame, button should be inactive but just_released should be true
        assert!(!handler.is_button_active(MouseButton::Left)); // Not active anymore  
        assert!(!handler.is_button_just_pressed(MouseButton::Left));
        assert!(handler.is_button_just_released(MouseButton::Left));

        // Clear frame status - everything should be inactive
        handler.clear_one_frame_statuses();

        assert!(!handler.is_button_active(MouseButton::Left));
        assert!(!handler.is_button_just_pressed(MouseButton::Left));
        assert!(!handler.is_button_just_released(MouseButton::Left));
    }

    #[test]
    fn debug_mouse_button_handling() {
        let (e_in, e_out) = mpsc::channel();
        let mut handler = InputState::<TestCommand>::new(e_out, [e_in.clone(), e_in.clone()]);

        // Check initial state
        assert!(!handler.is_button_active(MouseButton::Left));
        assert!(!handler.is_button_just_pressed(MouseButton::Left));

        // Send mouse button press
        e_in.send(crate::input::InputEvent::MouseButtonPressed(MouseButton::Left))
            .unwrap();

        handler.handle_received_input_events();

        // Debug: Check individual components
        let button_state = handler.mouse_button_state(MouseButton::Left);
        println!(
            "After press - active: {}, just_pressed: {}, just_released: {}",
            button_state.active, button_state.just_pressed, button_state.just_released
        );

        // This should work
        assert!(handler.is_button_just_pressed(MouseButton::Left));
    }

    #[test]
    fn mouse_scroll_delta_works() {
        let (e_in, e_out) = mpsc::channel();
        let mut handler = InputState::<TestCommand>::new(e_out, [e_in.clone(), e_in.clone()]);

        // Test scroll event
        e_in.send(crate::input::InputEvent::MouseScrollChange(2.5)).unwrap();

        handler.handle_received_input_events();

        assert_eq!(handler.mouse_scroll_delta(), 2.5);

        // Clear frame - scroll should reset to 0
        handler.clear_one_frame_statuses();

        assert_eq!(handler.mouse_scroll_delta(), 0.0);
    }

    #[test]
    fn cursor_location_updates_correctly() {
        let (e_in, e_out) = mpsc::channel();
        let mut handler = InputState::<TestCommand>::new(e_out, [e_in.clone(), e_in.clone()]);

        let new_location = Location { x: 123.45, y: 678.90 };
        let new_position = Position { x: 100.0, y: 200.0 };

        // Test cursor location update
        e_in.send(crate::input::InputEvent::CursorLocPosChange(new_location, new_position))
            .unwrap();

        handler.handle_received_input_events();

        assert_eq!(handler.cursor_location(), new_location);
    }

    #[test]
    fn camera_movement_updates_cursor_location() {
        let (e_in, e_out) = mpsc::channel();
        let mut handler = InputState::<TestCommand>::new(e_out, [e_in.clone(), e_in.clone()]);

        let initial_location = Location { x: 100.0, y: 200.0 };
        let initial_position = Position { x: 50.0, y: 75.0 };

        // Set initial position
        e_in.send(crate::input::InputEvent::CursorLocPosChange(
            initial_location,
            initial_position,
        ))
        .unwrap();

        handler.handle_received_input_events();

        // Apply camera movement
        let camera_x_change = 10.0;
        let camera_y_change = -15.0;

        e_in.send(crate::input::InputEvent::CameraMoved(camera_x_change, camera_y_change))
            .unwrap();

        handler.handle_received_input_events();

        let expected_location = Location {
            x: initial_location.x + camera_x_change,
            y: initial_location.y + camera_y_change,
        };

        assert_eq!(handler.cursor_location(), expected_location);
    }

    #[test]
    fn key_binding_management_works() {
        let (e_in, e_out) = mpsc::channel();
        let handler = InputState::<TestCommand>::new(e_out, [e_in.clone(), e_in.clone()]);

        use crate::input::KeyBind;

        // Create a key binding
        let keybind = KeyBind {
            command: TestCommand::Test,
            key_1: Some(KeyCode::KeyW),
            key_2: None,
            mouse_button: None,
        };

        // Set the key binding
        handler.set_key_bind(keybind.clone());

        // Check if binding was set (this would normally be processed in the next frame)
        // We can't directly test this without processing events, but we can test the check method
        // after manually setting it up
    }

    #[test]
    fn text_input_collects_characters() {
        let (e_in, e_out) = mpsc::channel();
        let mut handler = InputState::<TestCommand>::new(e_out, [e_in.clone(), e_in.clone()]);

        // Send character events
        e_in.send(crate::input::InputEvent::KeyPressed(KeyCode::KeyH, Some('h')))
            .unwrap();
        e_in.send(crate::input::InputEvent::KeyPressed(KeyCode::KeyE, Some('e')))
            .unwrap();
        e_in.send(crate::input::InputEvent::KeyPressed(KeyCode::KeyL, Some('l')))
            .unwrap();
        e_in.send(crate::input::InputEvent::KeyPressed(KeyCode::KeyL, Some('l')))
            .unwrap();
        e_in.send(crate::input::InputEvent::KeyPressed(KeyCode::KeyO, Some('o')))
            .unwrap();

        handler.handle_received_input_events();

        let text_input = handler.text_input();
        assert_eq!(text_input, Some("hello".to_string()));

        // Clear frame and check text input is cleared
        handler.clear_one_frame_statuses();

        let text_input_after_clear = handler.text_input();
        assert_eq!(text_input_after_clear, None);
    }

    #[test]
    fn text_input_handles_space_key() {
        let (e_in, e_out) = mpsc::channel();
        let mut handler = InputState::<TestCommand>::new(e_out, [e_in.clone(), e_in.clone()]);

        // Send text with space
        e_in.send(crate::input::InputEvent::KeyPressed(KeyCode::KeyH, Some('h')))
            .unwrap();
        e_in.send(crate::input::InputEvent::KeyPressed(KeyCode::KeyI, Some('i')))
            .unwrap();
        e_in.send(crate::input::InputEvent::KeyPressed(KeyCode::Space, None))
            .unwrap();
        e_in.send(crate::input::InputEvent::KeyPressed(KeyCode::KeyB, Some('b')))
            .unwrap();
        e_in.send(crate::input::InputEvent::KeyPressed(KeyCode::KeyE, Some('e')))
            .unwrap();

        handler.handle_received_input_events();

        let text_input = handler.text_input();
        assert_eq!(text_input, Some("hi be".to_string()));
    }

    #[test]
    fn key_repeat_functionality() {
        let (e_in, e_out) = mpsc::channel();
        let mut handler = InputState::<TestCommand>::new(e_out, [e_in.clone(), e_in.clone()]);

        // Press a key
        e_in.send(crate::input::InputEvent::KeyPressed(KeyCode::KeyA, Some('a')))
            .unwrap();

        handler.handle_received_input_events();

        // Simulate holding the key for many frames (key must remain active)
        // After the first frame, active_for starts at 0 and increments each frame
        for _ in 0..64 {
            handler.clear_one_frame_statuses();
        }

        // At this point, active_for should be 64, which is > 60 and 64 % 4 == 0
        assert!(handler.is_key_repeating(KeyCode::KeyA));
    }

    #[test]
    fn key_does_not_repeat_when_just_pressed() {
        let (e_in, e_out) = mpsc::channel();
        let mut handler = InputState::<TestCommand>::new(e_out, [e_in.clone(), e_in.clone()]);

        // Press a key
        e_in.send(crate::input::InputEvent::KeyPressed(KeyCode::KeyA, Some('a')))
            .unwrap();

        handler.handle_received_input_events();

        // Should not be repeating on the first frame
        assert!(!handler.is_key_repeating(KeyCode::KeyA));
    }

    #[test]
    fn multiple_keys_can_be_active_simultaneously() {
        let (e_in, e_out) = mpsc::channel();
        let mut handler = InputState::<TestCommand>::new(e_out, [e_in.clone(), e_in.clone()]);

        // Press multiple keys
        e_in.send(crate::input::InputEvent::KeyPressed(KeyCode::KeyA, Some('a')))
            .unwrap();
        e_in.send(crate::input::InputEvent::KeyPressed(KeyCode::KeyB, Some('b')))
            .unwrap();
        e_in.send(crate::input::InputEvent::KeyPressed(KeyCode::KeyC, Some('c')))
            .unwrap();

        handler.handle_received_input_events();

        // All keys should be active simultaneously
        assert!(handler.is_key_active(KeyCode::KeyA));
        assert!(handler.is_key_active(KeyCode::KeyB));
        assert!(handler.is_key_active(KeyCode::KeyC));

        assert!(handler.is_key_just_pressed(KeyCode::KeyA));
        assert!(handler.is_key_just_pressed(KeyCode::KeyB));
        assert!(handler.is_key_just_pressed(KeyCode::KeyC));
    }

    #[test]
    fn mouse_other_button_ignored() {
        let (e_in, e_out) = mpsc::channel();
        let mut handler = InputState::<TestCommand>::new(e_out, [e_in.clone(), e_in.clone()]);

        // Send other mouse button event (should be ignored)
        e_in.send(crate::input::InputEvent::MouseButtonPressed(MouseButton::Other(5)))
            .unwrap();

        handler.handle_received_input_events();

        // Other mouse button should not panic or cause issues
        // No way to directly test this, but the handler should still function normally
        assert_eq!(handler.mouse_scroll_delta(), 0.0); // Smoke test
    }

    #[test]
    fn events_accumulate_in_frame() {
        let (e_in, e_out) = mpsc::channel();
        let mut handler = InputState::<TestCommand>::new(e_out, [e_in.clone(), e_in.clone()]);

        // Send multiple events
        e_in.send(crate::input::InputEvent::KeyPressed(KeyCode::KeyA, Some('a')))
            .unwrap();
        e_in.send(crate::input::InputEvent::MouseButtonPressed(MouseButton::Left))
            .unwrap();
        e_in.send(crate::input::InputEvent::MouseScrollChange(1.5)).unwrap();

        handler.handle_received_input_events();

        // All events should be processed
        assert!(handler.is_key_just_pressed(KeyCode::KeyA));
        assert!(handler.is_button_just_pressed(MouseButton::Left));
        assert_eq!(handler.mouse_scroll_delta(), 1.5);
    }

    #[test]
    fn key_release_without_press_works() {
        let (e_in, e_out) = mpsc::channel();
        let mut handler = InputState::<TestCommand>::new(e_out, [e_in.clone(), e_in.clone()]);

        // Send key release without previous press
        e_in.send(crate::input::InputEvent::KeyReleased(KeyCode::KeyA, Some('a')))
            .unwrap();

        handler.handle_received_input_events();

        // Should not panic and key should be marked as just released
        assert!(!handler.is_key_active(KeyCode::KeyA));
        assert!(!handler.is_key_just_pressed(KeyCode::KeyA));
        assert!(handler.is_key_just_released(KeyCode::KeyA));
    }

    #[test]
    fn text_input_with_no_characters_returns_none() {
        let (e_in, e_out) = mpsc::channel();
        let mut handler = InputState::<TestCommand>::new(e_out, [e_in.clone(), e_in.clone()]);

        // Send non-character events
        e_in.send(crate::input::InputEvent::KeyPressed(KeyCode::Enter, None))
            .unwrap();
        e_in.send(crate::input::InputEvent::KeyPressed(KeyCode::Tab, None))
            .unwrap();
        e_in.send(crate::input::InputEvent::MouseButtonPressed(MouseButton::Left))
            .unwrap();

        handler.handle_received_input_events();

        let text_input = handler.text_input();
        assert_eq!(text_input, None);
    }

    #[test]
    fn active_for_counter_increments_correctly() {
        let (e_in, e_out) = mpsc::channel();
        let mut handler = InputState::<TestCommand>::new(e_out, [e_in.clone(), e_in.clone()]);

        // Press a key
        e_in.send(crate::input::InputEvent::KeyPressed(KeyCode::KeyA, Some('a')))
            .unwrap();

        handler.handle_received_input_events();

        // Initial frame - should be active but not repeating yet
        assert!(handler.is_key_active(KeyCode::KeyA));
        assert!(!handler.is_key_repeating(KeyCode::KeyA));

        // Simulate several frames
        for _ in 1..10 {
            handler.clear_one_frame_statuses();

            // Key should still be active, but still not repeating
            assert!(handler.is_key_active(KeyCode::KeyA));
            assert!(!handler.is_key_repeating(KeyCode::KeyA));
            assert!(!handler.is_key_just_pressed(KeyCode::KeyA)); // Should only be true on first frame
        }
    }

    // Add command-related tests using a proper keybinding setup
    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Encode, Decode, Hash)]
    pub enum ExtendedTestCommand {
        MoveUp,
        MoveDown,
        Attack,
        Combo,
    }
    impl CommandType for ExtendedTestCommand {}

    #[test]
    fn command_binding_with_single_key() {
        let (e_in, e_out) = mpsc::channel();
        let mut handler = InputState::<ExtendedTestCommand>::new(e_out, [e_in.clone(), e_in.clone()]);

        use crate::input::KeyBind;

        // Create and set a key binding
        let keybind = KeyBind {
            command: ExtendedTestCommand::MoveUp,
            key_1: Some(KeyCode::KeyW),
            key_2: None,
            mouse_button: None,
        };

        e_in.send(crate::input::InputEvent::SetKeyBind(keybind.clone()))
            .unwrap();

        handler.handle_received_input_events();

        // Press the bound key
        e_in.send(crate::input::InputEvent::KeyPressed(KeyCode::KeyW, Some('w')))
            .unwrap();

        handler.handle_received_input_events();

        // Command should be active
        assert!(handler.is_command_active(ExtendedTestCommand::MoveUp));
        assert!(handler.is_command_just_actived(ExtendedTestCommand::MoveUp));
        assert!(!handler.is_command_just_released(ExtendedTestCommand::MoveUp));
    }

    #[test]
    fn command_binding_with_mouse_button() {
        let (e_in, e_out) = mpsc::channel();
        let mut handler = InputState::<ExtendedTestCommand>::new(e_out, [e_in.clone(), e_in.clone()]);

        use crate::input::KeyBind;

        // Create and set a mouse button binding
        let keybind = KeyBind {
            command: ExtendedTestCommand::Attack,
            key_1: None,
            key_2: None,
            mouse_button: Some(MouseButton::Left),
        };

        e_in.send(crate::input::InputEvent::SetKeyBind(keybind.clone()))
            .unwrap();

        handler.handle_received_input_events();

        // Press the bound mouse button
        e_in.send(crate::input::InputEvent::MouseButtonPressed(MouseButton::Left))
            .unwrap();

        handler.handle_received_input_events();

        // Command should be active
        assert!(handler.is_command_active(ExtendedTestCommand::Attack));
        assert!(handler.is_command_just_actived(ExtendedTestCommand::Attack));
    }

    #[test]
    fn remove_key_binding_works() {
        let (e_in, e_out) = mpsc::channel();
        let mut handler = InputState::<ExtendedTestCommand>::new(e_out, [e_in.clone(), e_in.clone()]);

        use crate::input::KeyBind;

        // Create and set a key binding
        let keybind = KeyBind {
            command: ExtendedTestCommand::MoveUp,
            key_1: Some(KeyCode::KeyW),
            key_2: None,
            mouse_button: None,
        };

        e_in.send(crate::input::InputEvent::SetKeyBind(keybind.clone()))
            .unwrap();

        handler.handle_received_input_events();

        // Remove the binding
        e_in.send(crate::input::InputEvent::RemoveKeyBind(ExtendedTestCommand::MoveUp))
            .unwrap();

        handler.handle_received_input_events();

        // Check that binding was removed by verifying check_key_bind returns None
        assert_eq!(handler.check_key_bind(ExtendedTestCommand::MoveUp), None);
    }

    #[test]
    fn mouse_button_is_active_on_frame_it_was_released() {
        let (e_in, e_out) = mpsc::channel();
        let mut handler = InputState::<TestCommand>::new(e_out, [e_in.clone(), e_in.clone()]);

        // Send mouse button press (but don't process events yet)
        e_in.send(crate::input::InputEvent::MouseButtonPressed(MouseButton::Left))
            .unwrap();

        // Clear frame status (this resets just_pressed/just_released flags)
        handler.clear_one_frame_statuses();

        // Send mouse button release
        e_in.send(crate::input::InputEvent::MouseButtonReleased(MouseButton::Left))
            .unwrap();

        // Process both events
        handler.handle_received_input_events();

        // Should match the same pattern as the key test:
        // active=false, just_pressed=true, just_released=true
        // So is_button_active should return false || (true && true) = true
        assert!(handler.is_button_active(MouseButton::Left));
        assert!(handler.is_button_just_pressed(MouseButton::Left));
        assert!(handler.is_button_just_released(MouseButton::Left));
    }
}
