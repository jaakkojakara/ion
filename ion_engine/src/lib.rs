//! # Ion Engine
//!
//! The Ion Engine is a game engine for creating 2.5D games.
//! See the [crate::run] function for the entry point to the engine and documentation.

use core::{
    Constants, RenderFrameProps,
    application::run_render_loop,
    coordinates::ChunkLocation,
    universe::{Universe, UniverseDataType},
    world::{ActionType, UiDataType, WorldId, WorldType},
};
use gfx::*;
use input::Input;
use ion_common::Instant;
#[cfg(not(target_arch = "wasm32"))]
use ion_common::util::native_spin_sleep;
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
    mpsc,
};
use std::time::Duration;
use util::concurrency::spawn_thread;

use crate::{
    core::{UniverseFrameProps, application::ApplicationEvent, world::CommandType},
    files::Files,
    net::{Network, NetworkEvent},
};

pub use egui;
pub use winit::event::MouseButton;
pub use winit::keyboard::KeyCode;

pub mod core;
pub mod files;
pub mod gfx;
pub mod input;
pub mod net;
pub mod util;

/// Entry point to the Ion game engine.
///
/// This function initializes and runs the game engine with separate universe simulation and rendering threads.
/// The engine provides a complete game framework with networking, graphics rendering, input handling, and file management.
///
/// ## Architecture Overview
///
/// For more details about the archictecture, see the documentations for individual modules, especially the [core::world] module.
///
/// The Ion engine uses a dual-threaded architecture:
///
/// - **Universe Thread**: Handles game simulation, physics, AI, networking synchronization,
///   and all game logic. Runs at a fixed timestep independent of rendering. Universe can contain multiple worlds,
///   which all run in parallel.
///
/// - **Render Thread**: Handles graphics rendering, UI updates, and user input. Runs at
///   variable refresh rate and interpolates between universe frames.
///
/// ### Both threads follow a strict data flow each frame:
///
/// #### Universe thread:
/// ```text
/// Render Thread |                       Universe Thread                                 |  Render Thread
/// ------------- | --------------------------------------------------------------------- | -------------
/// User Input  → | → Commands  →  Actions → World State Update → Render Data & UI Data → | → Render
///               |      ↓            ↓                                                   |
///               | Key Bindings   Network Sync                                           |
/// ```
///
/// #### Render thread:
/// ```text
///     Universe Thread     |                Render Thread
/// ----------------------- | -------------------------------------------
/// Render Data & UI Data → | → UI Logic → Render → Wait until next frame
///                         |      ↑
///                         |  User Input
/// ```
///
/// ## Engine Modules
///
/// The engine is organized into several modules:
///
/// - **[`core`]**: Core engine functionality and types, including application lifecycle, coordinate systems,
///   universe management, and world type definitions.
///
/// - **[`gfx`]**: Graphics and rendering system built on top of WGPU. Handles GPU resource
///   management, shader compilation, render passes, and provides a 2.5D rendering system with
///   features like HDR-pipeline, dynamic lighting, depth-testing, ambient occlusion, and more.
///
/// - **[`input`]**: Input handling system that processes keyboard, mouse, and other input
///   devices. Provides both raw access to input state, as well as key binding to user defined commands.
///
/// - **[`network`]**: Networking and multiplayer support. Handles client-server architecture,
///   action synchronization, joining/leaving, and player management for networked games.
///
/// - **[`files`]**: File system abstraction and asset management. Provides cross-platform
///   file access and handles loading of game assets like textures, sounds, and data files.
///
/// - **[`util`]**: Utility functions and helper types including concurrency primitives,
///   configuration management, and platform-specific functionality.
///
/// ## Required Types
///
/// Games using the Ion engine must implement several core traits and types:
///
/// ### [`UniverseDataType`] (U)
/// Global game data that persists across all worlds, such as player profiles, global settings,
/// or campaign progress. Shared between all worlds in the universe.
///
/// ### [`WorldType`] (W)
/// The main game world/state container that holds all game data for a specific world or level.
/// Must implement methods for:
/// - Processing game actions and updating world state
/// - Building render data for the graphics system  
/// - Handling player joins/leaves in multiplayer
/// - Managing world-specific UI data
///
/// ### [`CommandType`] (C)
/// Input commands that are mapped to user input (keyboard, mouse) events. These are generated
/// by the input system and converted to actions by the game logic.
///
/// ### [`ActionType`] (A)
/// Represents all possible game actions that can be performed (player moves, attacks, building, etc.).
/// Actions are synchronized across network in multiplayer games and processed deterministically
/// by the universe thread.
///
/// ### [`UiDataType`] (D)
/// Data structure containing all information needed to render the game's user interface.
/// Built by the world each frame and sent to the render thread for display.
///
/// ## Parameters
///
/// - `constants`: Configuration parameters for the engine including graphics settings,
///   networking configuration, and game-specific constants. These do not change during runtime.
///
/// - `on_render_frame`: Callback function invoked each render frame. Receives a
///   [`RenderFrameProps`] containing the render thread properties.
///   This is where games implement all their "non-universe" logic.
///
/// ## Platform Support
///
/// The engine supports both native (desktop) and web (WASM) platforms. On native platforms,
/// the universe and render threads run independently with precise timing control. On web,
/// the threads are synchronized and rely on browser vsync for frame pacing.
/// Additionally, the WASM platform has some limitations compared to native:
/// - No multiplayer support, since it is not possible to send UDP packets from WASM.
/// - No fully independent render / universe threads. Instead they run in sync.
/// - No native file system access
/// - No debug tools or debug rendering
///
/// ## Major missing features
/// - Multiplayer support (Old version in place but does not work yet)
pub fn run<F, U, W, C, A, D>(constants: Constants, mut on_render_frame: F)
where
    F: 'static + Send + Sync + FnMut(RenderFrameProps<W>),
    U: UniverseDataType<WorldType = W>,
    W: WorldType<ActionType = A, UiDataType = D, UniverseDataType = U>,
    C: CommandType,
    A: ActionType,
    D: UiDataType,
{
    let (app_event_sender, app_event_receiver) = mpsc::channel::<ApplicationEvent>();
    let (network_event_sender, network_event_receiver) = mpsc::channel::<NetworkEvent>();
    let (gfx_data_sender, gfx_data_receiver) = mpsc::sync_channel::<(GfxFrameData, D)>(0);

    let engine_running = Arc::new(AtomicBool::new(true));

    let files: Files = Files::new(&constants);
    let input: Input<W::CommandType> = Input::new();
    let network: Arc<Network<W>> = Arc::new(Network::new(&constants, network_event_sender.clone()));
    let universe: Arc<Universe<W>> = Arc::new(Universe::new());

    // ---------------------------------------------------------- //
    // -------------------- Universe loop ----------------------- //
    // ---------------------------------------------------------- //

    spawn_thread(Some("Universe"), {
        let mut universe_frame_last = Instant::now();
        let mut universe_frame_duration = Duration::ZERO;
        let mut prev_frame_active_world_id: Option<WorldId> = None;
        let mut prev_frame_render_chunks: Vec<ChunkLocation> = Vec::new();

        let universe = universe.clone();

        let engine_running = engine_running.clone();

        let mut input_state = input.input_state_universe();

        move || {
            while engine_running.load(Ordering::Relaxed) {
                if universe.is_running() {
                    let mut worlds_data_lock = universe.lock_worlds_data();
                    let universe_data_lock = universe.lock_universe_data();
                    let universe_data = universe_data_lock.as_ref().unwrap();

                    let sync_results = {
                        input_state.handle_received_input_events();

                        let (stateful_actions, stateless_actions) =
                            universe.build_actions(&mut worlds_data_lock, &input_state);

                        input_state.clear_one_frame_statuses();

                        network.mp_sync_actions(
                            universe_data.active_player(),
                            stateful_actions,
                            stateless_actions,
                            &universe,
                            &mut worlds_data_lock,
                        )
                    };

                    if let Some(sync_results) = sync_results {
                        // TODO: Multithreaded universe frame execution
                        for world in worlds_data_lock.values_mut() {
                            let frame_props = UniverseFrameProps {
                                universe_data,
                                players_joining: &sync_results.players_joined,
                                players_leaving: &sync_results.players_left,
                                actions: sync_results.actions.get(&world.id()).unwrap(),
                            };
                            world.execute_on_universe_frame(frame_props);
                        }

                        universe.next_frame();

                        if let Some(active_world_id) = universe.active_world_id() {
                            if prev_frame_active_world_id != Some(active_world_id) {
                                prev_frame_render_chunks.clear();
                            }

                            if sync_results.is_at_sync {
                                let active_world = worlds_data_lock.get_mut(&active_world_id).unwrap();
                                let (global_data, sprite_data, debug_data) = {
                                    active_world.build_render_data(universe.active_frame(), &prev_frame_render_chunks)
                                };

                                let ui_data = active_world.build_ui_data(universe.active_frame());

                                let timing_data = GfxTimingData {
                                    universe_frame_duration,
                                    render_frame_duration: Duration::ZERO,
                                    render_frame_offset: 0.0,
                                    render_data_use_count: 0,
                                };

                                // Keep track of which chunks are cached by the renderer in the rendering thread
                                prev_frame_active_world_id = Some(active_world_id);
                                prev_frame_render_chunks = sprite_data.chunks_to_vec();

                                // Next step blocks until render thread is ready, so drop all locks here
                                drop(universe_data_lock);
                                drop(worlds_data_lock);

                                gfx_data_sender
                                    .send((
                                        GfxFrameData {
                                            global_data,
                                            timing_data,
                                            sprite_data,
                                            debug_data,
                                        },
                                        ui_data,
                                    ))
                                    .ok();
                            }
                        }
                    } else {
                        // Command syncing failed, unloading universe
                        drop(worlds_data_lock);
                        universe.pause();
                        universe.unload_universe();
                    }
                } else {
                    // Don't store commands that are received while paused
                    input_state.handle_received_input_events();
                    input_state.clear_one_frame_statuses();
                    universe.clear_actions();

                    // Process network messages for multiplayer joining
                    network.mp_sync_join_process(&universe);

                    //TODO: Sleep here for a bit to avoid busy-waiting
                }

                universe_frame_duration = universe_frame_last.elapsed();
                universe_frame_last = Instant::now();
            }
        }
    });

    // ---------------------------------------------------------- //
    // ---------------------- Render loop ----------------------- //
    // ---------------------------------------------------------- //

    let mut universe_frame_time_accumulated = Duration::ZERO;
    let mut latest_gfx_data: Option<(GfxFrameData, D)> = None;

    let mut render_frame_last = Instant::now();
    let mut render_frame_duration = Duration::ZERO;

    let mut input_state = input.input_state_ui();

    run_render_loop(constants, input, app_event_sender, move |renderer| {
        // --------------------- Sync universe thread --------------------- //

        if universe.is_running() {
            let universe_frame_time = universe.universe_frame_time();
            universe_frame_time_accumulated += render_frame_duration;

            if WASM_COMPATIBLE_RENDERING {
                if let Ok(gfx_data) = gfx_data_receiver.try_recv() {
                    latest_gfx_data = Some(gfx_data);
                }
                universe_frame_time_accumulated = Duration::ZERO;
            } else {
                while universe_frame_time_accumulated >= universe_frame_time {
                    if let Ok(gfx_data) = gfx_data_receiver.try_recv() {
                        latest_gfx_data = Some(gfx_data);
                    }
                    universe_frame_time_accumulated -= universe_frame_time;
                }
            }

            if let Some(gfx_data) = &mut latest_gfx_data {
                gfx_data.0.timing_data.render_frame_duration = render_frame_duration;
                gfx_data.0.timing_data.render_frame_offset =
                    universe_frame_time_accumulated.as_micros() as f32 / universe_frame_time.as_micros() as f32;
            }
        }

        let (gfx_data, ui_data) = match &latest_gfx_data {
            Some((gfx_data, ui_data)) => (Some(gfx_data), Some(ui_data)),
            None => (None, None),
        };

        renderer.pre_render(gfx_data);

        // ----------------- Run game logic for render loop ----------------- //

        input_state.handle_camera_movement(&renderer.camera());
        input_state.handle_received_input_events();

        let ui_ctx = renderer.ui_begin_pass();

        on_render_frame(RenderFrameProps {
            engine_running: engine_running.clone(),

            renderer: renderer,
            universe: &universe,
            files: &files,

            gfx_data: gfx_data,
            ui_input_state: &input_state,
            ui_data: ui_data,
            ui_ctx: &ui_ctx,

            app_events: &app_event_receiver,
            network_events: &network_event_receiver,
        });

        input_state.clear_one_frame_statuses();

        // ------------------------ Execute the render ---------------------- //

        renderer.render(gfx_data);

        if let Some(gfx_data) = &mut latest_gfx_data {
            gfx_data.0.timing_data.render_data_use_count += 1;
        }

        renderer.post_render();

        // Can't really sleep accurately on web platform, so we just skip it.
        // On web frame pacing should be done by vsync.
        #[cfg(not(target_arch = "wasm32"))]
        if let Some(frame_time_cap) = renderer.config().frame_time_cap() {
            if let Some(target_sleep_time) = Instant::now()
                .duration_since(render_frame_last)
                .and_then(|d| frame_time_cap.checked_sub(d))
            {
                native_spin_sleep(target_sleep_time);
            }
        }

        render_frame_duration = render_frame_last.elapsed();
        render_frame_last = Instant::now();

        engine_running.load(Ordering::Relaxed)
    })
}
