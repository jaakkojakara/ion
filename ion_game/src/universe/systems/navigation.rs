use std::collections::VecDeque;

use crate::universe::chunk::{Chunk, Chunks};
use ion_common::{
    Map,
    bincode::{Decode, Encode},
};
use ion_engine::core::CHUNK_SIZE;
use ion_engine::core::coordinates::{ChunkLocation, Direction, Location};

// A placeholder for the navigation system.

#[derive(Debug, Clone, Encode, Decode)]
pub struct NavChunk {
    loc: ChunkLocation,
    tiles: Box<[Direction; CHUNK_SIZE as usize * CHUNK_SIZE as usize]>,
}

impl NavChunk {
    pub fn new(loc: ChunkLocation) -> Self {
        Self {
            loc,
            tiles: Box::new([Direction::N; CHUNK_SIZE as usize * CHUNK_SIZE as usize]),
        }
    }

    pub fn update_dirs(&mut self, _chunk: &Chunk) {
        for (dir, loc) in self.tiles.iter_mut().zip(Chunk::tile_locs_iter(self.loc)) {
            *dir = Direction::new(loc.into(), Location::orig());
        }
    }
}

#[derive(Debug, Clone, Encode, Decode)]
pub struct Navigation {
    pub nav_chunks: Map<ChunkLocation, NavChunk>,
    pub dirty_chunks: VecDeque<ChunkLocation>,
}

impl Navigation {
    pub fn new() -> Self {
        Self {
            nav_chunks: Map::default(),
            dirty_chunks: VecDeque::default(),
        }
    }

    pub fn update_navigation(&mut self, chunks: &Chunks) {
        if let Some(loc) = self.dirty_chunks.pop_front() {
            let nav_chunk = self.nav_chunks.entry(loc).or_insert(NavChunk::new(loc));
            let chunk = chunks.get_unchecked(loc);
            nav_chunk.update_dirs(chunk);
        }
    }

    pub fn mark_dirty(&mut self, loc: ChunkLocation) {
        self.dirty_chunks.push_back(loc);
    }
}
