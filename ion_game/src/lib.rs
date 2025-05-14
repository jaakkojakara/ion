use ion_common::LogLevel;
use state::GlobalState;
use universe::{UniverseData, actions::Action, world::World};

use crate::config::bindings::Command;
use crate::config::constants;
use crate::ui::UiData;

#[cfg(target_arch = "wasm32")]
use ion_common::wasm_bindgen;
#[cfg(target_arch = "wasm32")]
use ion_common::wasm_bindgen::prelude::*;

pub mod assets;
pub mod config;
pub mod state;
pub mod ui;
pub mod universe;

pub const APP_NAME: &str = "ION";
pub const APP_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg_attr(target_arch = "wasm32", wasm_bindgen)]
pub fn run() {
    ion_common::set_logger_on(LogLevel::Info);
    #[cfg(not(target_arch = "wasm32"))]
    ion_common::set_log_file_write_on(ion_engine::files::file_paths::log_dir(&APP_NAME));

    // Run the game
    let mut global_state = GlobalState::Empty;
    ion_engine::run::<_, UniverseData, World, Command, Action, UiData>(constants(), move |mut props| {
        if let Some(new_state) = global_state.execute_frame_on_state(&mut props) {
            global_state = new_state;
        }
    });
}
