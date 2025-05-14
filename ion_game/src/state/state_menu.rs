use std::sync::atomic::Ordering;

use ion_common::log_info;
use ion_engine::core::application::ApplicationEvent;

use crate::state::{GlobalState, Props};

pub struct MenuState {
    //active_menu: Option<ActiveMenu>,
}

impl MenuState {
    pub fn new(_props: &mut Props) -> Self {
        Self {}
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

        // TODO: Implement menu state.

        None
    }
}
