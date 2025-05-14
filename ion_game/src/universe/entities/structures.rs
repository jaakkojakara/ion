use bincode::{Decode, Encode};
use ion_common::{Map, Set};
use ion_engine::{
    core::coordinates::{ChunkLocation, Location},
    gfx::GfxRef,
};

use crate::universe::entities::{Entity, EntityHandler};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Encode, Decode)]
pub enum StructureType {
    Torus,
    Wall,
    Lamp,
    Bollard,
    Tree,
}

impl StructureType {
    pub fn gfx_name(&self) -> String {
        format!("{:?}", self).to_lowercase()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Encode, Decode)]
pub struct StructureSize {
    pub width: u8,
    pub height: u8,
}

pub struct Structure<'a> {
    pub structure_type: &'a StructureType,
    pub loc: &'a Location,
    pub size: &'a StructureSize,
}

pub struct StructureMut<'a> {
    pub structure_type: &'a StructureType,
    pub loc: &'a Location,
    pub size: &'a StructureSize,

    id: Entity,
    gfx_sprites: &'a mut Map<Entity, GfxRef>,
    gfx_dirty_chunks: &'a mut Set<ChunkLocation>,
}

impl<'a> StructureMut<'a> {
    pub fn set_sprite(&mut self, sprite: GfxRef) {
        self.gfx_sprites.insert(self.id, sprite);
        self.gfx_dirty_chunks.insert((*self.loc).into());
    }

    pub fn entity_id(&self) -> Entity {
        self.id
    }
}

#[allow(dead_code)]
pub struct StructureClone {
    pub structure_type: StructureType,
    pub loc: Location,
    pub size: StructureSize,
}

impl StructureClone {
    pub fn as_structure(&'_ self) -> Structure<'_> {
        Structure {
            structure_type: &self.structure_type,
            loc: &self.loc,
            size: &self.size,
        }
    }
}

pub struct Structures {
    entity_handler: EntityHandler,

    structure_type: Vec<StructureType>,
    loc: Vec<Location>,
    size: Vec<StructureSize>,

    gfx_sprites: Map<Entity, GfxRef>,
    dirty_gfx_structure: Set<ChunkLocation>,
}

impl Structures {
    pub fn new() -> Self {
        Self {
            entity_handler: EntityHandler::new(),

            structure_type: Vec::new(),
            loc: Vec::new(),
            size: Vec::new(),

            gfx_sprites: Map::default(),
            dirty_gfx_structure: Set::default(),
        }
    }

    pub fn sprite_of(&self, entity: Entity) -> Option<GfxRef> {
        self.gfx_sprites.get(&entity).cloned()
    }

    pub fn set_sprite_of(&mut self, entity: Entity, sprite: GfxRef) {
        self.gfx_sprites.insert(entity, sprite);
    }

    pub fn add(&mut self, stat: Structure) -> Entity {
        let entity_id = self.entity_handler.next_id();
        if entity_id.generation != 0 {
            self.structure_type[entity_id.index as usize] = stat.structure_type.clone();
            self.loc[entity_id.index as usize] = stat.loc.clone();
            self.size[entity_id.index as usize] = stat.size.clone();
        } else {
            debug_assert!(entity_id.index() == self.loc.len() as u32);
            self.structure_type.push(stat.structure_type.clone());
            self.loc.push(stat.loc.clone());
            self.size.push(stat.size.clone());
        }
        entity_id
    }

    pub fn get(&'_ self, id: Entity) -> Option<Structure<'_>> {
        if self.entity_handler.is_valid(id) {
            Some(Structure {
                structure_type: &self.structure_type[id.index as usize],
                loc: &self.loc[id.index as usize],
                size: &self.size[id.index as usize],
            })
        } else {
            None
        }
    }

    pub fn get_mut(&'_ mut self, id: Entity) -> Option<StructureMut<'_>> {
        if self.entity_handler.is_valid(id) {
            Some(StructureMut {
                structure_type: &self.structure_type[id.index as usize],
                loc: &self.loc[id.index as usize],
                size: &self.size[id.index as usize],
                id,
                gfx_sprites: &mut self.gfx_sprites,
                gfx_dirty_chunks: &mut self.dirty_gfx_structure,
            })
        } else {
            None
        }
    }

    pub fn remove(&mut self, id: Entity) {
        self.entity_handler.delete_id(id);
    }

    pub fn iter(&'_ self) -> impl Iterator<Item = (Entity, Structure<'_>)> {
        self.entity_handler.iter_ids_valid().map(|id| {
            (
                id,
                Structure {
                    structure_type: &self.structure_type[id.index as usize],
                    loc: &self.loc[id.index as usize],
                    size: &self.size[id.index as usize],
                },
            )
        })
    }

    pub fn set_dirty(&mut self, loc: ChunkLocation) {
        self.dirty_gfx_structure.insert(loc);
    }

    pub fn remove_dirty(&mut self, loc: ChunkLocation) -> bool {
        self.dirty_gfx_structure.remove(&loc)
    }
}

impl Default for Structures {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------- //
// ---------------------- Save states ----------------------- //
// ---------------------------------------------------------- //

#[derive(Debug, Clone, bincode::Encode, bincode::Decode)]
pub struct StructuresSaveState {
    entity_handler: EntityHandler,
    structure_type: Vec<StructureType>,
    loc: Vec<Location>,
    size: Vec<StructureSize>,
}

impl StructuresSaveState {
    pub fn into_actual(self, chunks: impl Iterator<Item = ChunkLocation>) -> Structures {
        Structures {
            entity_handler: self.entity_handler,
            structure_type: self.structure_type,
            loc: self.loc,
            size: self.size,
            gfx_sprites: ion_common::Map::default(),
            dirty_gfx_structure: chunks.collect(),
        }
    }

    pub fn from_actual(structures: &Structures) -> Self {
        Self {
            entity_handler: structures.entity_handler.clone(),
            structure_type: structures.structure_type.clone(),
            loc: structures.loc.clone(),
            size: structures.size.clone(),
        }
    }
}
