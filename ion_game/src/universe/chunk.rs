use bincode::{Decode, Encode};
use ion_common::{Map, Set};
use ion_engine::core::CHUNK_SIZE;

use crate::assets;
use crate::universe::creator::WorldCreator;
use crate::universe::entities::mobs::render_mob;
use crate::universe::entities::structures::{
    Structure, StructureClone, StructureMut, StructureSize, Structures, StructuresSaveState,
};
use crate::universe::world::World;
use ion_engine::core::coordinates::{ChunkLocation, Location, TileLocation};
use ion_engine::gfx::GfxRef;

use super::entities::Entity;

#[derive(Default)]
pub struct Chunks {
    chunks: Map<ChunkLocation, Chunk>,
    structures: Structures,

    dirty_gfx_terrain: Set<ChunkLocation>,
}

impl Chunks {
    // ---------------------------------------------------------- //
    // ------------------- Chunk management --------------------- //
    // ---------------------------------------------------------- //

    pub fn exists(&self, loc: ChunkLocation) -> bool {
        self.chunks.contains_key(&loc)
    }

    pub fn get(&self, loc: ChunkLocation) -> Option<&Chunk> {
        self.chunks.get(&loc)
    }

    pub fn get_mut(&mut self, loc: ChunkLocation) -> Option<&mut Chunk> {
        self.chunks.get_mut(&loc)
    }

    pub fn get_unchecked(&self, loc: ChunkLocation) -> &Chunk {
        self.chunks.get(&loc).expect("Chunk must exist")
    }

    pub fn get_mut_unchecked(&mut self, loc: ChunkLocation) -> &mut Chunk {
        self.chunks.get_mut(&loc).expect("Chunk must exist")
    }

    pub fn create(&mut self, loc: ChunkLocation, creator: &mut WorldCreator) {
        assert!(!self.exists(loc), "Can't create chunk that already exists");

        self.dirty_gfx_terrain.insert(loc);
        self.structures.set_dirty(loc);

        let chunk = creator.gen_chunk(loc);
        let structures = creator.gen_chunk_structures(&chunk);

        self.chunks.insert(loc, chunk);
        let chunk_mut = self.chunks.get_mut(&loc).unwrap();
        for (structure, gfx) in structures {
            let entity = self.structures.add(structure.as_structure());
            chunk_mut.add_structure(structure.loc.into(), structure.size, entity);
            self.structures.set_sprite_of(entity, gfx);
        }
    }

    pub fn create_empty(&mut self, loc: ChunkLocation, creator: &WorldCreator) {
        assert!(!self.exists(loc), "Can't create chunk that already exists");

        let chunk = creator.gen_chunk(loc);
        self.chunks.insert(loc, chunk);
    }

    // ---------------------------------------------------------- //
    // -------------- Structure entity management --------------- //
    // ---------------------------------------------------------- //

    pub fn structure_at(&'_ self, loc: TileLocation) -> Option<Structure<'_>> {
        self.get(loc.into())
            .map(|chunk| chunk.structures[loc.tile_index()])
            .and_then(|entity| entity.and_then(|entity| self.structures.get(entity)))
    }

    pub fn structure_at_mut(&'_ mut self, loc: TileLocation) -> Option<StructureMut<'_>> {
        self.get_mut(loc.into())
            .map(|chunk| chunk.structures[loc.tile_index()])
            .and_then(|entity| entity.and_then(|entity| self.structures.get_mut(entity)))
    }

    pub fn add_structure(&mut self, structure_and_gfx: (StructureClone, GfxRef)) {
        let (stat, gfx) = structure_and_gfx;
        let entity = self.structures.add(stat.as_structure());

        // Find all chunks affected by this structure entity
        let base_loc: TileLocation = stat.loc.into();
        let end_loc = TileLocation {
            x: base_loc.x + stat.size.width as i16 - 1,
            y: base_loc.y + stat.size.height as i16 - 1,
        };

        let start_chunk: ChunkLocation = base_loc.into();
        let end_chunk: ChunkLocation = end_loc.into();

        // Add structure to all affected chunks
        for chunk_x in start_chunk.x..=end_chunk.x {
            for chunk_y in start_chunk.y..=end_chunk.y {
                let chunk_loc = ChunkLocation { x: chunk_x, y: chunk_y };
                self.get_mut_unchecked(chunk_loc)
                    .add_structure(base_loc, stat.size, entity);
                self.structures.set_dirty(chunk_loc);
            }
        }

        self.structures.set_sprite_of(entity, gfx);
    }

    pub fn remove_structure(&mut self, loc: TileLocation) {
        // Find all chunks affected by this structure entity
        let size = *self.structure_at(loc).expect("Structure must exist to remove it").size;
        let end_loc = TileLocation {
            x: loc.x + size.width as i16 - 1,
            y: loc.y + size.height as i16 - 1,
        };

        let start_chunk: ChunkLocation = loc.into();
        let end_chunk: ChunkLocation = end_loc.into();

        let mut removed_entity = None;

        // Remove structure from all affected chunks
        for chunk_x in start_chunk.x..=end_chunk.x {
            for chunk_y in start_chunk.y..=end_chunk.y {
                let chunk_loc = ChunkLocation { x: chunk_x, y: chunk_y };
                if let Some(entity) = self.get_mut_unchecked(chunk_loc).remove_structure(loc, size) {
                    // Only the base chunk should return the entity
                    if chunk_loc == start_chunk {
                        removed_entity = Some(entity);
                    }
                }
                self.structures.set_dirty(chunk_loc);
                self.dirty_gfx_terrain.insert(chunk_loc);
            }
        }

        if let Some(entity) = removed_entity {
            self.structures.remove(entity);
        }
    }

    pub fn build_gfx_chunked_data(
        &mut self,
        creator: &mut WorldCreator,
        loc: ChunkLocation,
        is_cached: bool,
    ) -> Option<Vec<GfxRef>> {
        let chunk = self.chunks.get(&loc).unwrap();
        let dirty_terrain = self.dirty_gfx_terrain.remove(&loc);
        let dirty_structure = self.structures.remove_dirty(loc);
        if !is_cached || dirty_terrain || dirty_structure {
            let new_terrain_gfx = if dirty_terrain {
                Some(creator.gen_chunk_terrain_gfx(chunk, self))
            } else {
                None
            };

            let new_structures_gfx = if dirty_structure {
                Some(
                    chunk
                        .structures()
                        .into_iter()
                        .filter_map(|entity| self.structures.sprite_of(entity))
                        .collect::<Vec<_>>(),
                )
            } else {
                None
            };

            let chunk_mut = self.get_mut_unchecked(loc);

            if let Some(new_terrain_gfx) = new_terrain_gfx {
                chunk_mut.gfx_terrain = new_terrain_gfx;
            }

            if let Some(new_entities_gfx) = new_structures_gfx {
                chunk_mut.gfx_structure = new_entities_gfx;
            }

            let mut gfx = chunk_mut.gfx_terrain.clone();
            gfx.extend(chunk_mut.gfx_structure.clone());
            Some(gfx)
        } else {
            None
        }
    }
}

pub struct Chunk {
    loc: ChunkLocation,
    locs: Box<[TileLocation; CHUNK_SIZE as usize * CHUNK_SIZE as usize]>,
    terrain: Box<[Terrain; CHUNK_SIZE as usize * CHUNK_SIZE as usize]>,

    mobs: Set<Entity>,
    structures: Box<[Option<Entity>; CHUNK_SIZE as usize * CHUNK_SIZE as usize]>,

    gfx_structure: Vec<GfxRef>,
    gfx_terrain: Vec<GfxRef>,
}

impl Chunk {
    pub fn new(loc: ChunkLocation, terrain: Box<[Terrain; CHUNK_SIZE as usize * CHUNK_SIZE as usize]>) -> Self {
        debug_assert!(
            loc.x.abs() < i16::MAX / CHUNK_SIZE,
            "Can't create chunk where tile x-coordinates exceed i16::MAX"
        );
        debug_assert!(
            loc.y.abs() < i16::MAX / CHUNK_SIZE,
            "Can't create chunk where tile y-coordinates exceed i16::MAX"
        );
        debug_assert!(
            terrain.iter().all(|t| t != &Terrain::GroundWithStructure),
            "Structures must be placed after creating the chunk"
        );

        Self {
            loc,
            locs: Chunk::tile_locs_iter(loc).collect::<Vec<_>>().try_into().unwrap(),
            terrain,

            mobs: Set::default(),
            structures: Box::new([None; CHUNK_SIZE as usize * CHUNK_SIZE as usize]),

            gfx_structure: vec![],
            gfx_terrain: vec![],
        }
    }

    pub fn loc(&self) -> ChunkLocation {
        self.loc
    }

    pub fn has_structures(&self) -> bool {
        self.terrain.iter().any(|t| t == &Terrain::GroundWithStructure)
    }

    pub fn terrain(&self) -> &[Terrain] {
        &self.terrain.as_slice()
    }

    pub fn terrain_at(&self, loc: TileLocation) -> Terrain {
        self.terrain[loc.tile_index()]
    }

    pub fn set_terrain_at(&mut self, loc: TileLocation, terrain: Terrain) -> Terrain {
        let prev_terrain = self.terrain[loc.tile_index()];
        self.terrain[loc.tile_index()] = terrain;
        prev_terrain
    }

    pub fn add_mob(&mut self, entity: Entity) {
        self.mobs.insert(entity);
    }

    pub fn remove_mob(&mut self, entity: Entity) {
        self.mobs.remove(&entity);
    }

    fn add_structure(&mut self, loc: TileLocation, size: StructureSize, entity: Entity) {
        // Set terrain and entity for all tiles covered by the structure
        for x_offset in 0..size.width {
            for y_offset in 0..size.height {
                let tile_loc = TileLocation {
                    x: loc.x + x_offset as i16,
                    y: loc.y + y_offset as i16,
                };

                // Only modify tiles that are within this chunk
                if ChunkLocation::from(tile_loc) == self.loc {
                    let tile_idx = tile_loc.tile_index();
                    self.terrain[tile_idx] = Terrain::GroundWithStructure;

                    // Only store the entity reference at the base tile
                    if x_offset == 0 && y_offset == 0 {
                        self.structures[tile_idx] = Some(entity);
                    }
                }
            }
        }
    }

    fn remove_structure(&mut self, loc: TileLocation, size: StructureSize) -> Option<Entity> {
        let mut removed_entity = None;

        // Clear terrain and entity for all tiles covered by the structure
        for x_offset in 0..size.width {
            for y_offset in 0..size.height {
                let tile_loc = TileLocation {
                    x: loc.x + x_offset as i16,
                    y: loc.y + y_offset as i16,
                };

                // Only modify tiles that are within this chunk
                if ChunkLocation::from(tile_loc) == self.loc {
                    let tile_idx = tile_loc.tile_index();
                    self.terrain[tile_idx] = Terrain::Ground;

                    // Only remove the entity reference from the base tile
                    if x_offset == 0 && y_offset == 0 {
                        removed_entity = self.structures[tile_idx].take();
                    }
                }
            }
        }

        removed_entity
    }

    pub fn structures(&self) -> Vec<Entity> {
        let mut structures: Vec<Entity> = self.structures.iter().filter_map(|entity| *entity).collect();
        structures.sort_by_key(|entity| entity.index());
        structures.dedup();
        structures
    }

    pub fn tile_at(&self, loc: TileLocation) -> Tile {
        Tile {
            loc,
            terrain: self.terrain[loc.tile_index()],
            structure: self.structures[loc.tile_index()],
        }
    }

    pub fn tiles_iter(&self) -> impl Iterator<Item = Tile> {
        self.locs
            .iter()
            .zip(self.terrain.iter())
            .zip(self.structures.iter())
            .map(|((loc, terrain), structure)| Tile {
                loc: *loc,
                terrain: *terrain,
                structure: *structure,
            })
    }

    /// Returns an iterator over all tile locations in the chunk.
    /// This is the same order as the order they are stored in the `locs` and `terrain` vectors.
    /// The index of the tile location matches [`TileLocation::tile_index()`].
    pub fn tile_locs_iter(loc: ChunkLocation) -> impl Iterator<Item = TileLocation> {
        (0..CHUNK_SIZE).flat_map(move |x| {
            (0..CHUNK_SIZE).map(move |y| TileLocation {
                x: loc.x * CHUNK_SIZE + x as i16,
                y: loc.y * CHUNK_SIZE + y as i16,
            })
        })
    }

    pub fn build_gfx_dynamic_sprites(&self, world: &World) -> Vec<GfxRef> {
        self.mobs
            .iter()
            .map(|entity| render_mob(*entity, &world.players, &world.systems, &world.mobs))
            .collect()
    }
}

/// A single tile in a chunk.
/// This is a read-only copy of the actual tile data.
/// Modifying the tile data is done via [`Chunks`] and [`Chunk`] methods.
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub struct Tile {
    pub loc: TileLocation,
    pub terrain: Terrain,
    pub structure: Option<Entity>,
}

impl Tile {
    pub fn center(&self) -> Location {
        Location::from(self.loc).update(0.5, 0.5)
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum Terrain {
    Water,
    Ground,
    GroundWithStructure,
}

impl Terrain {
    pub fn cost_multiplier(self) -> f32 {
        match self {
            Terrain::Water => f32::INFINITY,
            Terrain::Ground => 1.0,
            Terrain::GroundWithStructure => 2.0,
        }
    }

    pub fn is_pathable(self) -> bool {
        match self {
            Terrain::Water => false,
            Terrain::Ground => true,
            Terrain::GroundWithStructure => true,
        }
    }
}

// ---------------------------------------------------------- //
// ---------------------- Save states ----------------------- //
// ---------------------------------------------------------- //

#[derive(Debug, Clone, Encode, Decode)]
pub struct ChunksSaveState {
    chunks: Vec<ChunkLocation>,
    structures: StructuresSaveState,
    chunk_mobs: Map<ChunkLocation, Set<Entity>>,
}

impl ChunksSaveState {
    pub fn into_actual(self, creator: &WorldCreator) -> Chunks {
        let mut chunks = Chunks {
            chunks: Map::default(),
            structures: self.structures.into_actual(self.chunks.iter().map(|loc| *loc)),
            dirty_gfx_terrain: self.chunks.iter().cloned().collect(),
        };

        // Recreate chunks with terrain only (no structures generated)
        for chunk_loc in &self.chunks {
            chunks.create_empty(*chunk_loc, creator);
        }

        // Restore structure placements in chunks and regenerate graphics
        let structure_placements: Vec<_> = chunks
            .structures
            .iter()
            .map(|(entity, structure)| {
                (
                    entity,
                    structure.structure_type.clone(),
                    *structure.loc,
                    *structure.size,
                )
            })
            .collect();

        for (entity, structure_type, loc, size) in structure_placements {
            let base_loc: TileLocation = loc.into();
            let end_loc = TileLocation {
                x: base_loc.x + size.width as i16 - 1,
                y: base_loc.y + size.height as i16 - 1,
            };

            let start_chunk: ChunkLocation = base_loc.into();
            let end_chunk: ChunkLocation = end_loc.into();

            // Add structure to all affected chunks that exist
            for chunk_x in start_chunk.x..=end_chunk.x {
                for chunk_y in start_chunk.y..=end_chunk.y {
                    let chunk_loc = ChunkLocation { x: chunk_x, y: chunk_y };
                    if chunks.exists(chunk_loc) {
                        chunks
                            .get_mut_unchecked(chunk_loc)
                            .add_structure(base_loc, size, entity);
                    }
                }
            }

            let gfx = GfxRef::new(assets::gfx_ref(&structure_type.gfx_name()), loc);
            chunks.structures.set_sprite_of(entity, gfx);
        }

        // Restore mob assignments to chunks
        for (chunk_loc, mob_entities) in self.chunk_mobs {
            let chunk = chunks
                .get_mut(chunk_loc)
                .expect("Chunk must exist when loading entities in ChunksSaveState");
            chunk.mobs = mob_entities;
        }

        chunks
    }

    pub fn from_actual(chunks: &Chunks) -> Self {
        let chunk_mobs = chunks
            .chunks
            .iter()
            .map(|(loc, chunk)| (*loc, chunk.mobs.clone()))
            .collect();

        Self {
            chunks: chunks.chunks.keys().cloned().collect(),
            structures: StructuresSaveState::from_actual(&chunks.structures),
            chunk_mobs,
        }
    }
}
