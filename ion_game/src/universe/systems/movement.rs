use ion_engine::core::coordinates::{ChunkLocation, Direction};

use crate::universe::actions::Action;
use crate::universe::chunk::Chunks;
use crate::universe::entities::mobs::{MobType, Mobs};
use crate::universe::world::World;

pub struct Movement {}

impl Movement {
    pub fn new() -> Self {
        Self {}
    }

    pub fn update_movement(&self, chunks: &mut Chunks, mobs: &mut Mobs) {
        for (id, mob) in mobs.iter_mut() {
            if *mob.speed > 0.0 {
                let orig_loc = *mob.loc;
                let new_loc = orig_loc.towards_dir(*mob.dir, *mob.speed);

                if chunks.exists(ChunkLocation::from(new_loc)) {
                    let target_terrain = chunks
                        .get_unchecked(ChunkLocation::from(new_loc))
                        .terrain_at(new_loc.into());
                    if target_terrain.is_pathable() {
                        if ChunkLocation::from(new_loc) != ChunkLocation::from(orig_loc) {
                            chunks.get_mut_unchecked(ChunkLocation::from(orig_loc)).remove_mob(id);
                            chunks.get_mut_unchecked(ChunkLocation::from(new_loc)).add_mob(id);
                            *mob.loc = new_loc;
                        } else {
                            *mob.loc = new_loc;
                        }
                    } else if *mob.mob_type == MobType::Player {
                        if let Some(slide_loc) = try_slide_movement(orig_loc, *mob.dir, *mob.speed / 2.0, chunks) {
                            if ChunkLocation::from(slide_loc) != ChunkLocation::from(orig_loc) {
                                chunks.get_mut_unchecked(ChunkLocation::from(orig_loc)).remove_mob(id);
                                chunks.get_mut_unchecked(ChunkLocation::from(slide_loc)).add_mob(id);
                                *mob.loc = slide_loc;
                            } else {
                                *mob.loc = slide_loc;
                            }
                        }
                    }
                }
            };
        }
    }

    pub fn handle_player_movement_action(action: &Action, world: &mut World) {
        match action {
            Action::Move { player_id, direction } => {
                let player = world.players.get(*player_id);
                let mob = world.mobs.get_mut(player.entity_id()).unwrap();
                *mob.dir = *direction;
                *mob.speed = player.move_speed;
            }
            Action::NoMove { player_id } => {
                let player = world.players.get(*player_id);
                let mob = world.mobs.get_mut(player.entity_id()).unwrap();
                *mob.speed = 0.0;
            }
            _ => {}
        }
    }
}

fn try_slide_movement(
    orig_loc: ion_engine::core::coordinates::Location,
    direction: Direction,
    speed: f32,
    chunks: &Chunks,
) -> Option<ion_engine::core::coordinates::Location> {
    let alternative_directions = match direction {
        Direction::NE => vec![Direction::N, Direction::E],
        Direction::SE => vec![Direction::S, Direction::E],
        Direction::SW => vec![Direction::S, Direction::W],
        Direction::NW => vec![Direction::N, Direction::W],
        _ => vec![],
    };

    for alt_dir in alternative_directions {
        let slide_loc = orig_loc.towards_dir(alt_dir, speed);

        if chunks.exists(ChunkLocation::from(slide_loc)) {
            let slide_terrain = chunks
                .get_unchecked(ChunkLocation::from(slide_loc))
                .terrain_at(slide_loc.into());
            if slide_terrain.is_pathable() {
                return Some(slide_loc);
            }
        }
    }

    None
}
