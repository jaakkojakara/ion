use std::fmt::{Debug, Formatter};

use ion_common::bincode;
use ion_common::bincode::config::Configuration;
use ion_common::bincode::{Decode, Encode};
use ion_common::net::NetworkPlayerInfo;
use ion_common::{Map, PlayerId};

use ion_engine::core::FrameId;
use ion_engine::core::UniverseFrameProps;
use ion_engine::core::coordinates::ChunkLocation;
use ion_engine::core::world::{WorldId, WorldType};
use ion_engine::gfx::GfxDebugData;
use ion_engine::gfx::GfxGlobalData;
use ion_engine::gfx::GfxSpriteData;
use ion_engine::input::input_state::InputState;

use crate::config::bindings::Command;
use crate::ui::UiData;
use crate::ui::UiDebugData;
use crate::universe::UniverseData;
use crate::universe::actions::Action;
use crate::universe::actions::process_actions::process_action;
use crate::universe::chunk::{Chunks, ChunksSaveState};
use crate::universe::debug::build_gfx_debug_data;
use crate::universe::entities::mobs::Mobs;
use crate::universe::players::{Players, PlayersSaveState};
use crate::universe::systems::{Systems, SystemsSaveState};

use super::actions::build_stateful::build_stateful_actions;
use super::actions::build_stateless::build_stateless_actions;
use super::camera::Camera;
use super::debug::DebugConfig;

pub struct Lighting {
    pub sun: f32,
    pub ambient: f32,
}

pub struct World {
    pub id: WorldId,
    pub name: String,
    pub seed: u64,

    pub(super) camera: Camera,
    pub(super) players: Players,

    pub(super) mobs: Mobs,
    pub(super) chunks: Chunks,
    pub(super) systems: Systems,

    pub(super) lighting: Lighting,

    pub(super) debug_config: DebugConfig,
}

impl World {
    pub fn new(name: &str, world_seed: u64, active_player_id: Option<PlayerId>) -> Self {
        let id = 34;

        let mut systems = Systems::new(id, world_seed);
        let mut chunks = Chunks::default();

        // Create the [0,0] chunk so that the world is not empty
        systems.create_chunk(ChunkLocation::orig(), &mut chunks);

        let mut debug_config = DebugConfig::default();
        debug_config.debug_sys_enabled = true;

        Self {
            id,
            name: name.to_string(),
            seed: world_seed,
            camera: Camera::default(),
            players: Players::empty(active_player_id),

            mobs: Mobs::new(id),
            chunks,
            systems,

            lighting: Lighting { sun: 1.7, ambient: 0.5 },

            debug_config,
        }
    }
}

impl Debug for World {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.name.as_str())
    }
}

impl WorldType for World {
    type CommandType = Command;
    type ActionType = Action;
    type UniverseDataType = UniverseData;
    type UiDataType = UiData;

    fn id(&self) -> WorldId {
        self.id
    }

    fn name(&self) -> &str {
        self.name.as_str()
    }

    fn from_bytes(bytes: &[u8], active_player_info: Option<NetworkPlayerInfo>) -> Option<Self> {
        let conf = bincode::config::standard();
        let decode_attempt = bincode::decode_from_slice::<WorldSaveState, Configuration>(bytes, conf);

        decode_attempt
            .map(|(save_state, _)| WorldSaveState::into_actual(save_state, active_player_info.map(|info| info.id)))
            .ok()
    }

    fn as_bytes(&self) -> Vec<u8> {
        let save_state = WorldSaveState::from_actual(self);
        bincode::encode_to_vec(save_state, bincode::config::standard())
            .expect("Converting WorldSaveState to byte vec must succeed")
    }

    fn build_stateful_actions(&self, input: &InputState<Command>, is_active_world: bool) -> Vec<Action> {
        build_stateful_actions(input, is_active_world, &self)
    }

    fn build_stateless_actions(&self, input: &InputState<Command>, is_active_world: bool) -> Vec<Action> {
        build_stateless_actions(input, is_active_world, self)
    }

    fn execute_on_universe_frame(&mut self, props: UniverseFrameProps<Self>) {
        // Handle joining players
        for player_info in props.players_joining {
            let mut all_players = props.universe_data.players.lock().unwrap();
            if let Some(new_player) =
                all_players.player_joining(&player_info, &mut self.players, &mut self.chunks, &mut self.mobs)
            {
                self.players.add_player(new_player);
            }
        }

        // Handle leaving players
        for player_id in props.players_leaving {
            let mut all_players = props.universe_data.players.lock().unwrap();
            if let Some(player) = self.players.remove_player(*player_id) {
                all_players.player_leaving(player, &mut self.mobs);
            }
        }

        // Process actions
        for (player_id, actions) in props.actions {
            for action in actions {
                process_action(&props, *player_id, action, self);
            }
        }

        // Update all systems for this frame
        self.systems.update_systems(&mut self.mobs, &mut self.chunks);

        // Update camera location based on latest changes
        self.systems.update_camera(&mut self.camera, &self.players, &self.mobs);

        // Generate new chunks if needed
        self.systems
            .create_chunks(&mut self.players, &mut self.chunks, &mut self.mobs);
    }

    fn build_render_data(
        &mut self,
        frame: FrameId,
        cached: &[ChunkLocation],
    ) -> (GfxGlobalData, GfxSpriteData, GfxDebugData) {
        let chunks_to_render = self.camera.chunks_visible(&self.chunks);
        let mut chunked_gfx = Map::default();
        let mut dynamic_gfx = Vec::new();

        for chunk_loc in &chunks_to_render {
            let chunked_gfx_data =
                self.chunks
                    .build_gfx_chunked_data(&mut self.systems.creator, *chunk_loc, cached.contains(chunk_loc));

            chunked_gfx.insert(*chunk_loc, chunked_gfx_data);
            dynamic_gfx.extend(self.chunks.get_unchecked(*chunk_loc).build_gfx_dynamic_sprites(self));
        }

        let globals = GfxGlobalData {
            frame,
            camera_loc: self.camera.loc,
            camera_scale: self.camera.scale,
            lighting_ambient: self.lighting.ambient,
            lighting_sun: self.lighting.sun,
            post_bloom: 0.2,
        };

        let objects = GfxSpriteData {
            chunked_gfx,
            dynamic_gfx,
        };

        (globals, objects, build_gfx_debug_data(self))
    }

    fn build_ui_data(&self, _frame: FrameId) -> Self::UiDataType {
        let debug = if self.debug_config.debug_sys_enabled {
            Some(UiDebugData {
                lighting_sun: self.lighting.sun,
                lighting_ambient: self.lighting.ambient,
            })
        } else {
            None
        };

        UiData { debug }
    }
}

// ---------------------------------------------------------- //
// ---------------------- Save states ----------------------- //
// ---------------------------------------------------------- //

#[derive(Clone, Encode, Decode)]
struct WorldSaveState {
    id: WorldId,
    name: String,
    seed: u64,
    players: PlayersSaveState,
    systems: SystemsSaveState,

    mobs: Mobs,
    chunks: ChunksSaveState,
}

impl WorldSaveState {
    fn into_actual(self, active_player_id: Option<PlayerId>) -> World {
        let systems = self.systems.into_actual(self.seed);
        let chunks = self.chunks.into_actual(&systems.creator);

        World {
            id: self.id,
            name: self.name,
            seed: self.seed,
            camera: Camera::default(),
            players: self.players.into_actual(active_player_id),
            mobs: self.mobs,
            chunks,
            systems,
            lighting: Lighting { sun: 1.7, ambient: 0.5 },
            debug_config: DebugConfig::default(),
        }
    }

    fn from_actual(world: &World) -> Self {
        Self {
            id: world.id,
            name: world.name.clone(),
            seed: world.seed,
            players: PlayersSaveState::from_actual(&world.players, &world.mobs),
            systems: SystemsSaveState::from_actual(&world.systems),
            chunks: ChunksSaveState::from_actual(&world.chunks),
            mobs: world.mobs.clone(),
        }
    }
}
