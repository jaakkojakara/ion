use ion_engine::core::coordinates::{ChunkLocation, Location};

use crate::universe::chunk::Chunks;

pub struct Camera {
    pub loc: Location,
    pub scale: f32,

    pub max_scale: f32,
    pub min_scale: f32,
}

impl Camera {
    pub fn chunks_visible(&self, chunks: &Chunks) -> Vec<ChunkLocation> {
        // TODO: Make this calculate the actual chunks visible based on the camera scale, screen aspect ratio etc.
        ChunkLocation::chunks_around(self.loc.into(), 4)
            .into_iter()
            .filter(|chunk| chunks.exists(*chunk))
            .collect()
    }
}

impl Default for Camera {
    fn default() -> Self {
        Self {
            loc: Location { x: 0.0, y: 0.0 },
            scale: 10.0,

            max_scale: 50.0,
            min_scale: 5.0,
        }
    }
}
