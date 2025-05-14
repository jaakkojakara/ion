use std::sync::atomic::Ordering;

use crate::{
    assets::texture_assets,
    config::{default_gfx_config, splash_screen_gfx_config},
    ui::ui_init::draw_ui_init_screen,
    universe::creator::{UniverseParams, create_universe},
};
use ion_common::{log_info, net::NetworkPlayerInfo};
use ion_engine::core::application::ApplicationEvent;

use crate::state::{GlobalState, Props};

// ---------------------------------------------------------- //
// ----------------------- Init state ----------------------- //
// ---------------------------------------------------------- //

pub struct InitState {}

impl InitState {
    pub fn new(props: &mut Props) -> Self {
        props.renderer.load_texture_assets(texture_assets());
        props.renderer.set_config(splash_screen_gfx_config(props));
        #[cfg(target_arch = "wasm32")]
        props
            .renderer
            .set_wasm_window_size(splash_screen_gfx_config(props).frame_resolution);

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

        draw_ui_init_screen(props, props.renderer.texture_assets_progress().unwrap_or(0.0));

        let done = props.renderer.texture_assets_ready().unwrap_or(false);

        if done {
            // When all is done, proceed to next game state
            // Normally main menu, for testing we go straight to game state.

            props.renderer.set_config(default_gfx_config());
            #[cfg(target_arch = "wasm32")]
            props
                .renderer
                .set_wasm_window_size(default_gfx_config().frame_resolution);

            let universe_params = UniverseParams {
                name: "Test Universe".to_string(),
                seed: 6764,
                server: None,
                player: Some(NetworkPlayerInfo {
                    id: 0,
                    name: "player_main".to_string(),
                    addr: "127.0.0.1:0".parse().unwrap(),
                }),
            };
            let (universe_data, worlds) = create_universe(universe_params);
            props.universe.load_universe(universe_data, worlds, None);
            props.universe.set_active_world_by_name("default_world");
            props.universe.unpause();

            Some(GlobalState::game_state(props))
        } else {
            None
        }
    }
}
