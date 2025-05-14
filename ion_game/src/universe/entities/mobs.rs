use bincode::{Decode, Encode};
use ion_engine::core::{
    coordinates::{ChunkLocation, Direction, Location},
    world::WorldId,
};

use ion_engine::gfx::GfxRef;

use crate::assets;
use crate::universe::players::Players;
use crate::universe::players::player::Player;
use crate::universe::systems::Systems;

use crate::universe::{
    chunk::Chunks,
    entities::{Entity, EntityHandler},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Encode, Decode)]
pub enum MobType {
    Player,
    Enemy,
}

pub struct Mob<'a> {
    pub mob_type: &'a MobType,
    pub loc: &'a Location,
    pub dir: &'a Direction,
    pub speed: &'a f32,
}

impl Mob<'_> {
    pub fn clone(&self) -> MobClone {
        MobClone {
            mob_type: self.mob_type.clone(),
            loc: self.loc.clone(),
            dir: self.dir.clone(),
            speed: self.speed.clone(),
        }
    }
}

pub struct MobMut<'a> {
    pub mob_type: &'a MobType,
    pub loc: &'a mut Location,
    pub dir: &'a mut Direction,
    pub speed: &'a mut f32,
}

impl MobMut<'_> {
    pub fn as_mob(&'_ self) -> Mob<'_> {
        Mob {
            mob_type: self.mob_type,
            loc: self.loc,
            dir: self.dir,
            speed: self.speed,
        }
    }
}

#[derive(Debug, Clone, Encode, Decode)]
pub struct MobClone {
    pub mob_type: MobType,
    pub loc: Location,
    pub dir: Direction,
    pub speed: f32,
}

impl MobClone {
    pub fn as_mob(&'_ self) -> Mob<'_> {
        Mob {
            mob_type: &self.mob_type,
            loc: &self.loc,
            dir: &self.dir,
            speed: &self.speed,
        }
    }
}

#[derive(Clone, Encode, Decode)]
pub struct Mobs {
    world_id: WorldId,
    entity_handler: EntityHandler,

    mob_type: Vec<MobType>,
    loc: Vec<Location>,
    dir: Vec<Direction>,
    speed: Vec<f32>,
}

impl Mobs {
    pub fn new(world_id: WorldId) -> Self {
        Self {
            world_id,
            entity_handler: EntityHandler::new(),

            mob_type: Vec::new(),
            loc: Vec::new(),
            dir: Vec::new(),
            speed: Vec::new(),
        }
    }

    pub fn world_id(&self) -> WorldId {
        self.world_id
    }

    pub fn add(&mut self, mob: Mob, chunks: &mut Chunks) -> Entity {
        let entity_id = self.entity_handler.next_id();
        if entity_id.generation != 0 {
            self.mob_type[entity_id.index as usize] = mob.mob_type.clone();
            self.loc[entity_id.index as usize] = mob.loc.clone();
            self.dir[entity_id.index as usize] = mob.dir.clone();
            self.speed[entity_id.index as usize] = mob.speed.clone();
        } else {
            debug_assert!(entity_id.index() == self.loc.len() as u32);
            self.mob_type.push(mob.mob_type.clone());
            self.loc.push(mob.loc.clone());
            self.dir.push(mob.dir.clone());
            self.speed.push(mob.speed.clone());
        }

        chunks
            .get_mut_unchecked(ChunkLocation::from(*mob.loc))
            .add_mob(entity_id);

        entity_id
    }

    pub fn get(&'_ self, entity_id: Entity) -> Option<Mob<'_>> {
        if self.entity_handler.is_valid(entity_id) {
            Some(Mob {
                mob_type: &self.mob_type[entity_id.index as usize],
                loc: &self.loc[entity_id.index as usize],
                dir: &self.dir[entity_id.index as usize],
                speed: &self.speed[entity_id.index as usize],
            })
        } else {
            None
        }
    }

    pub fn get_mut(&'_ mut self, entity_id: Entity) -> Option<MobMut<'_>> {
        if self.entity_handler.is_valid(entity_id) {
            Some(MobMut {
                mob_type: &self.mob_type[entity_id.index as usize],
                loc: &mut self.loc[entity_id.index as usize],
                dir: &mut self.dir[entity_id.index as usize],
                speed: &mut self.speed[entity_id.index as usize],
            })
        } else {
            None
        }
    }

    pub fn remove(&mut self, entity_id: Entity, chunks: &mut Chunks) {
        self.entity_handler.delete_id(entity_id);
        chunks
            .get_mut_unchecked(ChunkLocation::from(self.loc[entity_id.index as usize]))
            .remove_mob(entity_id);
    }

    pub fn iter(&'_ self) -> impl Iterator<Item = (Entity, Mob<'_>)> {
        self.entity_handler.iter_ids().map(|id| {
            (
                id,
                Mob {
                    mob_type: &self.mob_type[id.index as usize],
                    loc: &self.loc[id.index as usize],
                    dir: &self.dir[id.index as usize],
                    speed: &self.speed[id.index as usize],
                },
            )
        })
    }

    pub fn iter_mut(&'_ mut self) -> impl Iterator<Item = (Entity, MobMut<'_>)> {
        self.entity_handler
            .iter_ids()
            .zip(self.mob_type.iter_mut())
            .zip(self.loc.iter_mut())
            .zip(self.dir.iter_mut())
            .zip(self.speed.iter_mut())
            .map(|((((entity_id, mob_type), loc), dir), speed)| {
                (
                    entity_id,
                    MobMut {
                        mob_type,
                        loc: loc,
                        dir: dir,
                        speed: speed,
                    },
                )
            })
    }
}

pub fn render_mob(entity: Entity, players: &Players, _systems: &Systems, mobs: &Mobs) -> GfxRef {
    let mob = mobs.get(entity).unwrap();

    match mob.mob_type {
        MobType::Player => {
            let player = players.get_by_entity(entity).unwrap();
            render_player(player, mob)
        }
        MobType::Enemy => {
            if mob.speed == &0.0 {
                GfxRef::new_anim(
                    assets::gfx_ref(&format!("z_idle_{}", mob.dir.tex_index())),
                    *mob.loc,
                    entity.index() % 20,
                )
            } else {
                GfxRef::new_anim(
                    assets::gfx_ref(&format!("z_run_{}", mob.dir.tex_index())),
                    *mob.loc,
                    entity.index() % 20,
                )
            }
        }
    }
}

fn render_player(_player: &Player, mob: Mob) -> GfxRef {
    if mob.speed == &0.0 {
        GfxRef::new_anim(
            assets::gfx_ref(&format!("mc_idle_{}", mob.dir.tex_index())),
            *mob.loc,
            0,
        )
    } else {
        GfxRef::new_anim(assets::gfx_ref(&format!("mc_run_{}", mob.dir.tex_index())), *mob.loc, 0)
    }
}
