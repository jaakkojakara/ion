use std::sync::atomic::Ordering;

use ion_common::log_info;
use ion_engine::{KeyCode, core::application::ApplicationEvent};

use crate::{
    state::{GlobalState, Props},
    ui::{ui_debug::draw_ui_debug, ui_pause::draw_ui_pause, ui_tips::draw_ui_tips},
};

// ---------------------------------------------------------- //
// ----------------------- Game state ----------------------- //
// ---------------------------------------------------------- //

pub struct GameState {
    pub show_debug_ui_main: bool,
    pub show_debug_ui_lighting: bool,

    pub is_paused: bool,
}

impl GameState {
    pub fn new(_props: &mut Props) -> Self {
        Self {
            show_debug_ui_main: false,
            show_debug_ui_lighting: false,

            is_paused: false,
        }
    }

    pub fn execute_on_frame(&mut self, props: &mut Props) -> Option<GlobalState> {
        for event in props.app_events.try_iter() {
            match event {
                ApplicationEvent::CloseRequested => {
                    props.engine_running.store(false, Ordering::Relaxed);
                }
                ApplicationEvent::FocusGained => {
                    log_info!("App focus gained");
                }
                ApplicationEvent::FocusLost => {
                    log_info!("App focus lost");
                }
                ApplicationEvent::Resumed => {
                    log_info!("App resumed");
                }
                ApplicationEvent::Suspended => {
                    log_info!("App suspended");
                }
            }
        }

        for event in props.network_events.try_iter() {
            match event {
                _ => panic!("Multiplayer not implemented: {:?}", event),
            }
        }

        if props.ui_input_state.is_key_just_pressed(KeyCode::Escape) {
            self.is_paused = !self.is_paused;
            if self.is_paused {
                props.universe.pause();
            } else {
                props.universe.unpause();
            }
        }

        draw_ui_debug(props, self);
        draw_ui_pause(props, self);
        draw_ui_tips(props);

        None
    }
}
