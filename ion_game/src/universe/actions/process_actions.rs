use crate::universe::actions::Action;
use crate::universe::entities::mobs::{MobClone, MobType};
use crate::universe::systems::movement::Movement;
use crate::universe::world::World;
use ion_common::PlayerId;
use ion_engine::core::UniverseFrameProps;
use ion_engine::core::coordinates::Direction;

pub fn process_action(_props: &UniverseFrameProps<World>, player_id: PlayerId, action: &Action, world: &mut World) {
    match action {
        Action::SetCameraScale(scale) => {
            world.camera.scale = (*scale).min(world.camera.max_scale).max(world.camera.min_scale);
        }
        action @ Action::Move { .. } => {
            Movement::handle_player_movement_action(action, world);
        }
        action @ Action::NoMove { .. } => {
            Movement::handle_player_movement_action(action, world);
        }
        Action::SetLightingSun(sun) => world.lighting.sun = *sun,
        Action::SetLightingAmbient(ambient) => world.lighting.ambient = *ambient,
        Action::DebugToggleNoise1 => {
            world.debug_config.show_noise = !world.debug_config.show_noise;
        }
        Action::DebugSysEnabled(enabled) => {
            world.debug_config.debug_sys_enabled = *enabled;
        }
        Action::DebugToggleChunkBorders => {
            world.debug_config.draw_chunk_borders = !world.debug_config.draw_chunk_borders;
        }
        Action::DebugToggleTileBorders => {
            world.debug_config.draw_tile_borders = !world.debug_config.draw_tile_borders;
        }
        Action::DebugToggleFlowField => {
            world.debug_config.draw_flow_field = !world.debug_config.draw_flow_field;
        }
        Action::SpawnMobs { loc } => {
            world.mobs.add(
                MobClone {
                    mob_type: MobType::Enemy,
                    loc: *loc,
                    dir: Direction::NE,
                    speed: 0.05,
                }
                .as_mob(),
                &mut world.chunks,
            );
        }
        Action::Shoot { loc } => {
            let player_loc = world.mobs.get(world.players.get(player_id).entity_id()).unwrap().loc;
            let tiles_on_line = player_loc.tiles_on_line(*loc);
            for tile in tiles_on_line {
                if let Some(_) = world.chunks.structure_at(tile) {
                    world.chunks.remove_structure(tile);
                }
            }
        }
    }
}
