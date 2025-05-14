use std::{
    collections::BTreeMap,
    net::SocketAddr,
    path::PathBuf,
    sync::{Arc, atomic::AtomicBool, mpsc::Receiver},
};

use ion_common::{PlayerId, net::NetworkPlayerInfo};
use universe::Universe;
use world::WorldType;

use crate::{
    core::application::ApplicationEvent,
    files::Files,
    gfx::{GfxFrameData, renderer::Renderer},
    input::input_state::InputState,
    net::NetworkEvent,
};

pub mod application;
pub mod coordinates;
pub mod universe;
pub mod world;

// ---------------------------------------------------------- //
// --------------------- Core constants --------------------- //
// ---------------------------------------------------------- //

pub const DEFAULT_UPS: u64 = 60;
pub const CHUNK_SIZE: i16 = 16;

// ---------------------------------------------------------- //
// -------------------- Frame properties -------------------- //
// ---------------------------------------------------------- //

pub type FrameId = u64;

/// This is the main handle to the engine.
/// Each (render) frame, a function for the frame is called with this as an argument.
/// It contains access to all major modules with few exceptions:
/// - Logging is handled directly via the `log` module in the commons crate.
/// - Proggressing the in-universe state and logic is done via the [`WorldType`] implementation.
pub struct RenderFrameProps<'a, W: WorldType> {
    /// Engine shutdown handle. Set to false for graceful shutdown.
    pub engine_running: Arc<AtomicBool>,

    /// Renderer module. Allows loading in texture assets, changing rendering config etc.
    pub renderer: &'a mut Renderer,

    /// Universe module. Allows loading in, starting and stopping the universe.
    /// This also allows sending Actions to the universe for example based on UI input. (See [`WorldType::ActionType`] for shortcut)
    pub universe: &'a Universe<W>,

    /// Files module. Access to the filesystem.
    pub files: &'a Files,

    /// Gfx data. Contains all the universe data needed for rendering the current frame.
    /// If universe is not running, this will be `None`.
    pub gfx_data: Option<&'a GfxFrameData>,

    /// UI input state. Contains the current input state for the UI/rendering thread.
    pub ui_input_state: &'a InputState<W::CommandType>,

    /// UI data. Contains all the information sent by the Universe for rendering the UI of the current frame.
    /// If universe is not running, this will be `None`.
    pub ui_data: Option<&'a W::UiDataType>,

    /// UI context. Contains the egui context for building the UI for the current frame.
    pub ui_ctx: &'a egui::Context,

    /// Receiver for application events. These include things like window focus, window close button being pressed etc.
    pub app_events: &'a Receiver<ApplicationEvent>,

    /// Receiver for network events. These include multiplayer events like updates about joining progress or players leaving etc.
    pub network_events: &'a Receiver<NetworkEvent>,
}

pub struct UniverseFrameProps<'a, W: WorldType> {
    // Handle to universe-level data
    pub universe_data: &'a W::UniverseDataType,

    // Multiplayer updates
    pub players_joining: &'a [NetworkPlayerInfo],
    pub players_leaving: &'a [PlayerId],

    // Commands that are executed this frame
    pub actions: &'a BTreeMap<PlayerId, Vec<W::ActionType>>,
}

// ---------------------------------------------------------- //
// -------------- Global constant data formats -------------- //
// ---------------------------------------------------------- //

#[derive(Debug, Clone)]
pub struct Constants {
    pub app_name: &'static str,
    pub gfx: GfxConstants,
    pub net: Option<NetworkConstants>,
}

#[derive(Debug, Clone)]
pub struct NetworkConstants {
    /// Address to bind to for listening for incoming connections
    pub bind_addr: SocketAddr,
    /// Address to connect to for server host service
    pub host_addr: SocketAddr,
}

#[derive(Debug, Clone)]
pub struct GfxConstants {
    /// Path to texture asset directory
    pub asset_path: PathBuf,
    /// Camera angle in degrees from vertical (0 = top-down, 90 = side view)
    pub camera_angle_deg: f32,
    /// Number of pixels per world unit for rendering, measured on screen x-axis
    pub pixels_per_unit: f32,
    /// Total height range in world units from min to max height
    pub height_units_total: f32,
    /// Zero height level scaled to 0-1 range (e.g. 0.25 means zero is 25% up from min height)
    pub height_scaled_zero: f32,
}
