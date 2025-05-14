use crate::state::state_game::GameState;
use crate::state::state_init::InitState;
use crate::state::state_menu::MenuState;
use crate::universe::world::World;

use ion_common::log_info;
use ion_engine::core::RenderFrameProps;

pub mod state_game;
pub mod state_init;
pub mod state_menu;

// ---------------------------------------------------------- //
// ----------------- Core types and consts ------------------ //
// ---------------------------------------------------------- //

pub type Props<'a> = RenderFrameProps<'a, World>;

// ---------------------------------------------------------- //
// ------------------ Global state holder ------------------- //
// ---------------------------------------------------------- //

#[allow(clippy::large_enum_variant)]
pub enum GlobalState {
    Empty,
    Init(InitState),
    Menu(MenuState),
    Game(GameState),
}

impl GlobalState {
    pub fn init_state(props: &mut Props) -> Self {
        log_info!("Switching to init state");
        Self::Init(InitState::new(props))
    }

    pub fn menu_state(props: &mut Props) -> Self {
        log_info!("Switching to menu state");
        Self::Menu(MenuState::new(props))
    }

    pub fn game_state(props: &mut Props) -> Self {
        log_info!("Switching to game state");
        Self::Game(GameState::new(props))
    }

    /// Executes a frame based on current state.
    /// Returns an optional new state, or None if no state change happens.
    pub fn execute_frame_on_state(&mut self, props: &mut Props) -> Option<Self> {
        match self {
            GlobalState::Empty => Some(GlobalState::init_state(props)),
            GlobalState::Init(state) => state.execute_on_frame(props),
            GlobalState::Menu(state) => state.execute_on_frame(props),
            GlobalState::Game(state) => state.execute_on_frame(props),
        }
    }
}
