use ion_common::math::rand::Rng;
use ion_common::net::{NetworkPlayerInfo, NetworkServerInfo};

use ion_engine::core::coordinates::{ChunkLocation, Location};
use ion_engine::gfx::GfxRef;

use crate::assets::gfx_ref;
use crate::universe::UniverseData;
use crate::universe::chunk::Terrain;
use crate::universe::chunk::{Chunk, Chunks};
use crate::universe::creator::noise::Noise;
use crate::universe::entities::entity_creators::{create_bollard, create_lamp};
use crate::universe::entities::structures::StructureClone;
use crate::universe::world::World;

mod noise;

// ---------------------------------------------------------- //
// ------------------ Universe generation ------------------- //
// ---------------------------------------------------------- //

pub struct UniverseParams {
    pub name: String,
    pub seed: u64,
    pub player: Option<NetworkPlayerInfo>,
    pub server: Option<NetworkServerInfo>,
}

pub fn create_universe(params: UniverseParams) -> (UniverseData, Vec<World>) {
    let world = World::new("default_world", params.seed, params.player.as_ref().map(|info| info.id));
    (
        UniverseData::new(params.name, params.seed, params.server, params.player),
        vec![world],
    )
}

// ---------------------------------------------------------- //
// -------------------- World generation -------------------- //
// ---------------------------------------------------------- //

pub struct WorldCreator {
    pub seed: u64,
    pub random: Rng,
    noise_ground: Noise,
    _noise_forest: Noise,
}

impl WorldCreator {
    pub fn new(seed: u64) -> Self {
        Self {
            seed,
            random: Rng::new(Some(seed as u128 + 682723)),
            noise_ground: Noise::new(seed + 723, 0.07, 6, 1.5),
            _noise_forest: Noise::new(seed + 45345, 0.07, 4, 1.6),
        }
    }

    pub fn gen_chunk(&self, loc: ChunkLocation) -> Chunk {
        let terrain: Box<[Terrain]> = Chunk::tile_locs_iter(loc)
            .map(|loc| {
                if Location::orig().dist(loc.into()) > 20.0 && self.noise_ground.at(loc) < 0.5 {
                    Terrain::Water
                } else {
                    Terrain::Ground
                }
            })
            .collect::<Vec<Terrain>>()
            .into_boxed_slice();

        Chunk::new(loc, terrain.try_into().unwrap())
    }

    pub fn gen_chunk_structures(&mut self, chunk: &Chunk) -> Vec<(StructureClone, GfxRef)> {
        let loc = chunk.loc();
        let mut structures = vec![];

        if loc.x == 0 && loc.y == 0 {
            structures.push(create_bollard(Location { x: 8.2, y: 8.2 }));
            structures.push(create_lamp(Location { x: 1.0, y: 1.0 }));
            structures.push(create_lamp(Location { x: 11.0, y: 11.0 }));
        }

        structures
    }

    pub fn gen_chunk_noise_gfx(&self, chunk: &Chunk) -> Vec<GfxRef> {
        let gfx: Vec<_> = chunk
            .tiles_iter()
            .map(|tile| {
                let n = self.noise_ground.at(tile.loc) * 255.0;
                GfxRef::new(gfx_ref(&format!("noise_{}", n as u8)), tile.loc.into())
            })
            .collect();

        gfx
    }

    pub fn gen_chunk_terrain_gfx(&mut self, chunk: &Chunk, _chunks: &Chunks) -> Vec<GfxRef> {
        let mut gfx: Vec<_> = chunk
            .tiles_iter()
            .map(|tile| {
                let is_even = (tile.loc.x % 2).abs() == (tile.loc.y % 2).abs();
                let ground = if is_even {
                    GfxRef::new(gfx_ref("test_tile_dark"), tile.loc.into())
                } else {
                    GfxRef::new(gfx_ref("test_tile"), tile.loc.into())
                };
                let water = if is_even {
                    GfxRef::new(gfx_ref("warn_tile"), tile.loc.into())
                } else {
                    GfxRef::new(gfx_ref("warn_tile"), tile.loc.into())
                };
                let structure = GfxRef::new(gfx_ref("blue_tile"), tile.loc.into());

                match tile.terrain {
                    Terrain::Ground => ground,
                    Terrain::Water => water,
                    Terrain::GroundWithStructure => structure,
                }
            })
            .collect();

        if chunk.loc().x == 0 && chunk.loc().y == 0 {
            for _ in 0..300 {
                let loc = Location::from(chunk.loc()).update(
                    self.random.gen_range_f32(0.0, 16.0),
                    self.random.gen_range_f32(0.0, 16.0),
                );
                let r_id = self.random.gen_range_u32(1, 4);
                if chunk.tile_at(loc.into()).terrain == Terrain::Ground {
                    gfx.push(GfxRef::new(gfx_ref(&format!("shrub_{}", r_id)), loc));
                }
            }
        }

        gfx
    }
}
