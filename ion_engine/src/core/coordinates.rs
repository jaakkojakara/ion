use std::ops::Add;

use derive_engine::RawData;
use ion_common::bincode::{Decode, Encode};
use winit::dpi::PhysicalPosition;

use crate::core::CHUNK_SIZE;
use crate::gfx::gfx_config::Resolution;
use crate::util::casting::RawData;

// ---------------------------------------------------------- //
// ------------------- Core Position Type-------------------- //
// ---------------------------------------------------------- //

/// Position represents a position on the screen, as opposed to the game world.
/// Coordinate system is same as in textures and for example HTML. Top left is (0,0) and
/// X grows right, while Y grows down. Bottom of the window is Y=1 and right is X=1
#[derive(Debug, Clone, Copy, PartialEq, RawData, Encode, Decode)]
pub struct Position {
    pub x: f32,
    pub y: f32,
}

impl Position {
    pub const CENTER: Self = Position { x: 0.5, y: 0.5 };
    pub const TOP_LEFT: Self = Position { x: 0.0, y: 0.0 };
    pub const TOP_RIGHT: Self = Position { x: 0.5, y: 1.0 };
    pub const BOTTOM_LEFT: Self = Position { x: 1.0, y: 1.0 };
    pub const BOTTOM_RIGHT: Self = Position { x: 1.0, y: 1.0 };

    pub fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }

    pub fn from_physical_position(pos: PhysicalPosition<f64>, window_resolution: Resolution) -> Self {
        Self {
            x: pos.x as f32 / window_resolution.width as f32,
            y: pos.y as f32 / window_resolution.height as f32,
        }
    }

    pub fn update(self, x_inc: f32, y_inc: f32) -> Self {
        Position {
            x: self.x + x_inc,
            y: self.y + y_inc,
        }
    }

    /// Returns true if the position is within the frame of the window
    pub fn is_visible(&self) -> bool {
        self.x >= 0.0 && self.x <= 1.0 && self.y >= 0.0 && self.y <= 1.0
    }

    /// Converts the position to the clip space coordinates
    pub fn to_clip_pos(&self) -> [f32; 2] {
        [self.x * 2.0 - 1.0, 1.0 - self.y * 2.0]
    }
}

impl Add for Position {
    type Output = Position;

    fn add(self, rhs: Self) -> Self::Output {
        Position {
            x: self.x + rhs.x,
            y: self.y + rhs.y,
        }
    }
}

// ---------------------------------------------------------- //
// ------------------- Core Location Type-------------------- //
// ---------------------------------------------------------- //

/// Represents an in-world location. Does not have any specific bounds.
/// Y-Positive axis grows to top left, while X-Positive axis grows to top right.
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, RawData, Encode, Decode)]
pub struct Location {
    pub x: f32,
    pub y: f32,
}

impl Location {
    pub fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }

    #[inline]
    pub fn orig() -> Self {
        Self { x: 0.0, y: 0.0 }
    }

    #[inline]
    pub fn intersection(line_1_a: Self, line_1_b: Self, line_2_a: Self, line_2_b: Self) -> Option<Location> {
        let den = (line_1_a.x - line_1_b.x) * (line_2_a.y - line_2_b.y)
            - (line_1_a.y - line_1_b.y) * (line_2_a.x - line_2_b.x);
        if den == 0.0 {
            return None; // Lines are parallel or coincident
        }

        let t_num = (line_1_a.x - line_2_a.x) * (line_2_a.y - line_2_b.y)
            - (line_1_a.y - line_2_a.y) * (line_2_a.x - line_2_b.x);
        let u_num = (line_1_a.x - line_2_a.x) * (line_1_a.y - line_1_b.y)
            - (line_1_a.y - line_2_a.y) * (line_1_a.x - line_1_b.x);

        let t = t_num / den;
        let u = u_num / den;

        if (0.0..=1.0).contains(&t) && (0.0..=1.0).contains(&u) {
            // Intersection point
            let ix = line_1_a.x + t * (line_1_b.x - line_1_a.x);
            let iy = line_1_a.y + t * (line_1_b.y - line_1_a.y);
            return Some(Location { x: ix, y: iy });
        }

        None // No intersection within the segments
    }

    #[inline]
    pub fn midpoint(self, other: Location) -> Self {
        Location {
            x: (self.x + other.x) / 2.0,
            y: (self.y + other.y) / 2.0,
        }
    }

    #[inline]
    pub fn update(self, x_inc: f32, y_inc: f32) -> Self {
        Location {
            x: self.x + x_inc,
            y: self.y + y_inc,
        }
    }

    /// Distance from other location
    #[inline]
    pub fn dist(self, other: Location) -> f32 {
        ((self.x - other.x).powf(2.0) + (self.y - other.y).powf(2.0)).sqrt()
    }

    /// Squared distance from other location.
    /// Works for comparison, but not actual value. Cheaper to compute than the real distance
    #[inline]
    pub fn dist_sq(self, other: Location) -> f32 {
        (self.x - other.x).powf(2.0) + (self.y - other.y).powf(2.0)
    }

    #[inline]
    pub fn towards_loc(self, to: Location, distance: f32) -> Location {
        let travel_percent = (distance / self.dist(to)).min(1.0);
        Location {
            x: self.x + (to.x - self.x) * travel_percent,
            y: self.y + (to.y - self.y) * travel_percent,
        }
    }

    #[inline]
    pub fn towards_dir(self, to: Direction, distance: f32) -> Location {
        match to {
            Direction::N => Location {
                x: self.x,
                y: self.y + distance,
            },
            Direction::E => Location {
                x: self.x + distance,
                y: self.y,
            },
            Direction::S => Location {
                x: self.x,
                y: self.y - distance,
            },
            Direction::W => Location {
                x: self.x - distance,
                y: self.y,
            },
            Direction::NE => Location {
                x: self.x + distance / std::f32::consts::SQRT_2,
                y: self.y + distance / std::f32::consts::SQRT_2,
            },
            Direction::SE => Location {
                x: self.x + distance / std::f32::consts::SQRT_2,
                y: self.y - distance / std::f32::consts::SQRT_2,
            },
            Direction::SW => Location {
                x: self.x - distance / std::f32::consts::SQRT_2,
                y: self.y - distance / std::f32::consts::SQRT_2,
            },
            Direction::NW => Location {
                x: self.x - distance / std::f32::consts::SQRT_2,
                y: self.y + distance / std::f32::consts::SQRT_2,
            },
            Direction::Deg(angle_degrees) => {
                // Convert to radians and adjust for coordinate system
                // 0 degrees = North, clockwise rotation
                let angle_rad = std::f32::consts::FRAC_PI_2 - (angle_degrees as f32).to_radians();
                let (sin, cos) = angle_rad.sin_cos();
                Location {
                    x: self.x + distance * cos,
                    y: self.y + distance * sin,
                }
            }
        }
    }

    /// Returns all tiles that a line crosses using the Grid Traversal Algorithm (Amanatides-Woo).
    pub fn tiles_on_line(&self, end: Location) -> Vec<TileLocation> {
        let start = *self;
        let mut tiles = vec![TileLocation::from(*self)];

        let start_tile = TileLocation::from(start);
        let end_tile = TileLocation::from(end);
        let dx = end.x - start.x;
        let dy = end.y - start.y;

        let mut x = start_tile.x;
        let mut y = start_tile.y;

        let step_x = if dx > 0.0 { 1 } else { -1 };
        let step_y = if dy > 0.0 { 1 } else { -1 };

        // How far along the ray we must travel to cross one grid cell
        let t_delta_x = if dx == 0.0 { f32::INFINITY } else { (step_x as f32) / dx };
        let t_delta_y = if dy == 0.0 { f32::INFINITY } else { (step_y as f32) / dy };

        // Calculate initial tMax values - how far to travel to reach next grid line
        let mut t_max_x = if dx == 0.0 {
            f32::INFINITY
        } else {
            let next_x_boundary = if step_x > 0 { (x + 1) as f32 } else { x as f32 };
            (next_x_boundary - start.x) / dx
        };

        let mut t_max_y = if dy == 0.0 {
            f32::INFINITY
        } else {
            let next_y_boundary = if step_y > 0 { (y + 1) as f32 } else { y as f32 };
            (next_y_boundary - start.y) / dy
        };

        // Traverse the line until we reach the end tile
        while x != end_tile.x || y != end_tile.y {
            if t_max_x < t_max_y {
                // Step in X direction
                t_max_x += t_delta_x;
                x += step_x;
            } else {
                // Step in Y direction
                t_max_y += t_delta_y;
                y += step_y;
            }

            tiles.push(TileLocation { x, y });
        }

        tiles
    }
}

impl Add for Location {
    type Output = Location;

    #[inline]
    fn add(self, rhs: Self) -> Self::Output {
        Location {
            x: self.x + rhs.x,
            y: self.y + rhs.y,
        }
    }
}

impl Default for Location {
    fn default() -> Self {
        Self::orig()
    }
}

// ---------------------------------------------------------- //
// --------------------- Tile Location ---------------------- //
// ---------------------------------------------------------- //

#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Eq, Ord, Hash, RawData, Encode, Decode)]
pub struct TileLocation {
    pub x: i16,
    pub y: i16,
}

impl TileLocation {
    /// Returns all 8 neighbors of the tile, starting from North and going counter-clockwise
    #[rustfmt::skip]
    pub fn tiles_neighboring(self) -> [TileLocation; 8] {
        [
            TileLocation { x: self.x, y: self.y + 1, },
            TileLocation { x: self.x - 1, y: self.y + 1, },
            TileLocation { x: self.x - 1, y: self.y, },
            TileLocation { x: self.x - 1, y: self.y - 1, },
            TileLocation { x: self.x, y: self.y - 1, },
            TileLocation { x: self.x + 1, y: self.y - 1, },
            TileLocation { x: self.x + 1, y: self.y, },
            TileLocation { x: self.x + 1, y: self.y + 1, },
        ]
    }

    #[inline]
    pub fn update(self, x_inc: i16, y_inc: i16) -> TileLocation {
        TileLocation {
            x: self.x + x_inc,
            y: self.y + y_inc,
        }
    }

    #[inline]
    pub fn midpoint(self, other: TileLocation) -> TileLocation {
        TileLocation {
            x: (self.x + other.x) / 2,
            y: (self.y + other.y) / 2,
        }
    }

    #[inline]
    pub fn center(self) -> Location {
        Location {
            x: self.x as f32 + 0.5,
            y: self.y as f32 + 0.5,
        }
    }

    #[inline]
    pub fn dist(self, other: TileLocation) -> f32 {
        ((self.x as f32 - other.x as f32).powf(2.0) + (self.y as f32 - other.y as f32).powf(2.0)).sqrt()
    }

    #[inline]
    pub fn dist_sq(self, other: TileLocation) -> i16 {
        (self.x - other.x).pow(2) + (self.y - other.y).pow(2)
    }

    #[inline]
    pub fn dist_manhattan(self, other: TileLocation) -> i16 {
        (self.x - other.x).abs() + (self.y - other.y).abs()
    }

    #[inline]
    pub fn tile_index(self) -> usize {
        let local_x = self.x.rem_euclid(CHUNK_SIZE);
        let local_y = self.y.rem_euclid(CHUNK_SIZE);
        (local_x as usize * CHUNK_SIZE as usize) + local_y as usize
    }

    #[inline]
    pub fn tile_index_fast(self) -> usize {
        let local_x = self.x & (CHUNK_SIZE - 1); // Faster than rem_euclid for power of 2
        let local_y = self.y & (CHUNK_SIZE - 1);
        (local_x as usize * CHUNK_SIZE as usize) + local_y as usize
    }
}

impl Add for TileLocation {
    type Output = TileLocation;

    #[inline]
    fn add(self, rhs: Self) -> Self::Output {
        TileLocation {
            x: self.x + rhs.x,
            y: self.y + rhs.y,
        }
    }
}

impl Default for TileLocation {
    fn default() -> Self {
        Self { x: 0, y: 0 }
    }
}

impl From<Location> for TileLocation {
    #[inline]
    fn from(loc: Location) -> Self {
        Self {
            x: loc.x.floor() as i16,
            y: loc.y.floor() as i16,
        }
    }
}

impl From<TileLocation> for Location {
    #[inline]
    fn from(loc: TileLocation) -> Self {
        Self {
            x: loc.x as f32,
            y: loc.y as f32,
        }
    }
}

// ---------------------------------------------------------- //
// --------------------- Chunk Location --------------------- //
// ---------------------------------------------------------- //

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, RawData, Encode, Decode)]
pub struct ChunkLocation {
    pub x: i16,
    pub y: i16,
}

impl ChunkLocation {
    #[inline]
    pub fn to_u32(self) -> u32 {
        //unsafe { std::mem::transmute(self) } // Faster?
        ((self.x as u32) << 16) | ((self.y as u32) & 0xFFFF)
    }

    #[inline]
    pub fn orig() -> Self {
        Self { x: 0, y: 0 }
    }

    /// Returns all 8 neighbors of the chunk
    /// First 4 are adjacent, last 4 are diagonal
    #[rustfmt::skip]
    #[inline]
    pub fn neighbors(self) -> [ChunkLocation; 8] {
        [
            ChunkLocation { x: self.x + 1, y: self.y, },
            ChunkLocation { x: self.x, y: self.y - 1, },
            ChunkLocation { x: self.x - 1, y: self.y, },
            ChunkLocation { x: self.x, y: self.y + 1, },
            ChunkLocation { x: self.x + 1, y: self.y + 1, },
            ChunkLocation { x: self.x + 1, y: self.y - 1, },
            ChunkLocation { x: self.x - 1, y: self.y - 1, },
            ChunkLocation { x: self.x - 1, y: self.y + 1, },
        ]
    }

    /// Returns the 4 adjacent neighbors of the chunk
    #[rustfmt::skip]
    #[inline]
    pub fn neighbors_adjacent(self) -> [ChunkLocation; 4] {
        [
            ChunkLocation { x: self.x + 1, y: self.y, },
            ChunkLocation { x: self.x, y: self.y - 1, },
            ChunkLocation { x: self.x - 1, y: self.y, },
            ChunkLocation { x: self.x, y: self.y + 1, },
        ]
    }

    /// Returns the 4 diagonal neighbors of the chunk
    #[rustfmt::skip]
    #[inline]
    pub fn neighbors_diagonal(self) -> [ChunkLocation; 4] {
        [
            ChunkLocation { x: self.x + 1, y: self.y + 1, },
            ChunkLocation { x: self.x + 1, y: self.y - 1, },
            ChunkLocation { x: self.x - 1, y: self.y - 1, },
            ChunkLocation { x: self.x - 1, y: self.y + 1, },
        ]
    }

    pub fn chunks_around(self, range: u32) -> Vec<ChunkLocation> {
        let mut id_vec: Vec<ChunkLocation> = Vec::with_capacity((range as usize * 2 + 1).pow(2) - 1);
        for x in (self.x - range as i16)..=(self.x + range as i16) {
            for y in (self.y - range as i16)..=(self.y + range as i16) {
                id_vec.push(ChunkLocation { x, y })
            }
        }
        id_vec
    }
}

impl From<Location> for ChunkLocation {
    #[inline]
    fn from(loc: Location) -> Self {
        Self {
            x: (loc.x / CHUNK_SIZE as f32).floor() as i16,
            y: (loc.y / CHUNK_SIZE as f32).floor() as i16,
        }
    }
}

impl From<TileLocation> for ChunkLocation {
    #[inline]
    fn from(loc: TileLocation) -> Self {
        Self {
            x: (loc.x as usize / CHUNK_SIZE as usize) as i16,
            y: (loc.y as usize / CHUNK_SIZE as usize) as i16,
        }
    }
}

impl From<ChunkLocation> for Location {
    #[inline]
    fn from(loc: ChunkLocation) -> Self {
        Self {
            x: loc.x as f32 * CHUNK_SIZE as f32,
            y: loc.y as f32 * CHUNK_SIZE as f32,
        }
    }
}

impl From<ChunkLocation> for TileLocation {
    #[inline]
    fn from(loc: ChunkLocation) -> Self {
        TileLocation {
            x: loc.x as i16 * CHUNK_SIZE,
            y: loc.y as i16 * CHUNK_SIZE,
        }
    }
}

// ---------------------------------------------------------- //
// ----------------------- Direction ------------------------ //
// ---------------------------------------------------------- //

/// Represents a direction in the game world. There are eight possible directions.
/// Since the world is in 45-degree angle, the north is facing top-left, i.e. along positive y-axel
/// The direction is either a cardinal direction or a custom angle. Custom angle is in degrees,
/// ranging from 0 to 360, and represents the "compass angle" of the direction.
#[repr(u16)]
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Eq, Ord, Hash, Encode, Decode)]
pub enum Direction {
    N,
    E,
    S,
    W,
    NE,
    SE,
    SW,
    NW,
    Deg(u16),
}

impl Direction {
    pub fn new(from: Location, to: Location) -> Self {
        let dx = to.x - from.x;
        let dy = to.y - from.y;
        let angle_degrees = -dy.atan2(dx).to_degrees() + 90.0;
        let normalized_degrees = (angle_degrees + 360.0) % 360.0;
        Direction::Deg(normalized_degrees as u16)
    }

    #[inline]
    pub fn opposite(self) -> Self {
        match self {
            Direction::N => Direction::S,
            Direction::E => Direction::W,
            Direction::S => Direction::N,
            Direction::W => Direction::E,
            Direction::NE => Direction::SW,
            Direction::SE => Direction::NW,
            Direction::SW => Direction::NE,
            Direction::NW => Direction::SE,
            Direction::Deg(angle) => Direction::Deg((angle + 180) % 360),
        }
    }

    pub fn rotate_cw_45(self) -> Self {
        match self {
            Direction::N => Direction::NE,
            Direction::E => Direction::SE,
            Direction::S => Direction::SW,
            Direction::W => Direction::NW,
            Direction::NE => Direction::E,
            Direction::SE => Direction::S,
            Direction::SW => Direction::W,
            Direction::NW => Direction::N,
            Direction::Deg(angle) => {
                let new_angle = (angle + 45) % 360;
                Direction::Deg(new_angle)
            }
        }
    }

    pub fn rotate_ccw_45(self) -> Self {
        match self {
            Direction::N => Direction::NW,
            Direction::E => Direction::NE,
            Direction::S => Direction::SE,
            Direction::W => Direction::SW,
            Direction::NE => Direction::N,
            Direction::SE => Direction::E,
            Direction::SW => Direction::S,
            Direction::NW => Direction::W,
            Direction::Deg(angle) => {
                let new_angle = (angle + 315) % 360;
                Direction::Deg(new_angle)
            }
        }
    }

    pub fn tex_index(self) -> u32 {
        match self {
            Direction::N => 0,
            Direction::NW => 1,
            Direction::W => 2,
            Direction::SW => 3,
            Direction::S => 4,
            Direction::SE => 5,
            Direction::E => 6,
            Direction::NE => 7,
            Direction::Deg(angle_degrees) => {
                // Define the angles for the cardinal directions in degrees
                let cardinal_angles = [
                    (0, 0.0),   // N
                    (1, 315.0), // NW
                    (2, 270.0), // W
                    (3, 225.0), // SW
                    (4, 180.0), // S
                    (5, 135.0), // SE
                    (6, 90.0),  // E
                    (7, 45.0),  // NE
                ];

                // Normalize the Custom angle to the range [0, 360)
                let angle = angle_degrees as f32 % 360.0;

                // Find the closest cardinal direction by minimizing the angular difference
                cardinal_angles
                    .iter()
                    .min_by(|(_, a1), (_, a2)| {
                        let diff1 = (angle - a1).abs().min(360.0 - (angle - a1).abs());
                        let diff2 = (angle - a2).abs().min(360.0 - (angle - a2).abs());
                        diff1.partial_cmp(&diff2).unwrap()
                    })
                    .map(|(index, _)| *index)
                    .unwrap()
            }
        }
    }

    pub fn as_deg(&self) -> u16 {
        match self {
            Direction::N => 0,
            Direction::NE => 45,
            Direction::E => 90,
            Direction::SE => 135,
            Direction::S => 180,
            Direction::SW => 225,
            Direction::W => 270,
            Direction::NW => 315,
            Direction::Deg(deg) => *deg,
        }
    }

    pub fn as_vec2(&self) -> (f32, f32) {
        match self {
            Direction::N => (0.0, 1.0),
            Direction::NE => (0.7071, 0.7071),
            Direction::E => (1.0, 0.0),
            Direction::SE => (0.7071, -0.7071),
            Direction::S => (0.0, -1.0),
            Direction::SW => (-0.7071, -0.7071),
            Direction::W => (-1.0, 0.0),
            Direction::NW => (-0.7071, 0.7071),
            Direction::Deg(deg) => {
                let angle_rad = (*deg as f32).to_radians();
                // For custom angles, 0 = North, clockwise rotation
                // So we need to shift by 90 degrees (π/2) and negate for clockwise
                let adjusted_angle = std::f32::consts::FRAC_PI_2 - angle_rad;
                (adjusted_angle.cos(), adjusted_angle.sin())
            }
        }
    }

    /// Rotates the direction by the given number of degrees clockwise.
    /// Positive degrees rotate clockwise, negative degrees rotate counterclockwise.
    pub fn rotate_deg(self, degrees: i16) -> Self {
        // Convert any direction to degrees first
        let current_deg = match self {
            Direction::N => 0,
            Direction::NE => 45,
            Direction::E => 90,
            Direction::SE => 135,
            Direction::S => 180,
            Direction::SW => 225,
            Direction::W => 270,
            Direction::NW => 315,
            Direction::Deg(deg) => deg as i16,
        };

        // Convert back to Direction, preferring named directions when exact
        match (current_deg + degrees % 360 + 360) % 360 {
            0 | 360 => Direction::N,
            45 => Direction::NE,
            90 => Direction::E,
            135 => Direction::SE,
            180 => Direction::S,
            225 => Direction::SW,
            270 => Direction::W,
            315 => Direction::NW,
            deg => Direction::Deg(deg as u16),
        }
    }
}

impl Default for Direction {
    fn default() -> Self {
        Direction::NE
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    mod location {
        use super::*;

        #[test]
        fn test_intersection() {
            // Test parallel lines
            assert_eq!(
                Location::intersection(
                    Location::new(0.0, 0.0),
                    Location::new(1.0, 0.0),
                    Location::new(0.0, 1.0),
                    Location::new(1.0, 1.0),
                ),
                None
            );

            // Test intersecting lines
            let intersection = Location::intersection(
                Location::new(0.0, 0.0),
                Location::new(1.0, 1.0),
                Location::new(0.0, 1.0),
                Location::new(1.0, 0.0),
            );
            assert_eq!(intersection, Some(Location::new(0.5, 0.5)));

            // Test non-intersecting line segments
            assert_eq!(
                Location::intersection(
                    Location::new(0.0, 0.0),
                    Location::new(1.0, 1.0),
                    Location::new(2.0, 2.0),
                    Location::new(3.0, 3.0),
                ),
                None
            );
        }

        #[test]
        fn test_distance_calculations() {
            let loc1 = Location::new(0.0, 0.0);
            let loc2 = Location::new(3.0, 4.0);

            assert_eq!(loc1.dist(loc2), 5.0); // 3-4-5 triangle
            assert_eq!(loc1.dist_sq(loc2), 25.0);
        }

        #[test]
        fn test_towards_location() {
            let start = Location::new(0.0, 0.0);
            let end = Location::new(10.0, 0.0);

            // Test partial distance
            let mid = start.towards_loc(end, 5.0);
            assert_eq!(mid, Location::new(5.0, 0.0));

            // Test distance larger than total
            let beyond = start.towards_loc(end, 20.0);
            assert_eq!(beyond, end);
        }

        #[test]
        fn test_towards_direction() {
            let start = Location::new(0.0, 0.0);

            // Test cardinal directions
            assert_eq!(start.towards_dir(Direction::N, 1.0), Location::new(0.0, 1.0));
            assert_eq!(start.towards_dir(Direction::E, 1.0), Location::new(1.0, 0.0));
            assert_eq!(start.towards_dir(Direction::S, 1.0), Location::new(0.0, -1.0));
            assert_eq!(start.towards_dir(Direction::W, 1.0), Location::new(-1.0, 0.0));

            // Test diagonal directions (should move by 1/√2 in each component)
            let diagonal_dist = 1.0 / std::f32::consts::SQRT_2;
            assert_eq!(
                start.towards_dir(Direction::NE, 1.0),
                Location::new(diagonal_dist, diagonal_dist)
            );
        }
    }

    mod tile_location {
        use super::*;

        #[test]
        fn test_neighboring_tiles() {
            let tile = TileLocation { x: 5, y: 5 };
            let neighbors = tile.tiles_neighboring();

            // Test all 8 neighbors are present and correct
            assert!(neighbors.contains(&TileLocation { x: 6, y: 6 }));
            assert!(neighbors.contains(&TileLocation { x: 6, y: 5 }));
            assert!(neighbors.contains(&TileLocation { x: 6, y: 4 }));
            assert!(neighbors.contains(&TileLocation { x: 5, y: 4 }));
            assert!(neighbors.contains(&TileLocation { x: 4, y: 4 }));
            assert!(neighbors.contains(&TileLocation { x: 4, y: 5 }));
            assert!(neighbors.contains(&TileLocation { x: 4, y: 6 }));
            assert!(neighbors.contains(&TileLocation { x: 5, y: 6 }));
        }

        #[test]
        fn test_distance_calculations() {
            let tile1 = TileLocation { x: 0, y: 0 };
            let tile2 = TileLocation { x: 3, y: 4 };

            assert_eq!(tile1.dist(tile2), 5.0); // 3-4-5 triangle
            assert_eq!(tile1.dist_sq(tile2), 25);
            assert_eq!(tile1.dist_manhattan(tile2), 7); // |3| + |4| = 7
        }

        #[test]
        fn test_conversions() {
            let tile = TileLocation { x: 10, y: 20 };
            let loc: Location = tile.into();
            assert_eq!(loc, Location::new(10.0, 20.0));

            let tile_back: TileLocation = loc.into();
            assert_eq!(tile_back, tile);
        }

        #[test]
        fn test_negative_conversions() {
            // Test whole number negative coordinates
            let loc = Location::new(5.0, -11.0);
            let tile: TileLocation = loc.into();
            assert_eq!(tile, TileLocation { x: 5, y: -11 });

            // Test fractional negative coordinates
            let loc = Location::new(-5.7, -11.3);
            let tile: TileLocation = loc.into();
            assert_eq!(tile, TileLocation { x: -6, y: -12 });

            // Test mixed positive/negative coordinates
            let loc = Location::new(-3.0, 7.0);
            let tile: TileLocation = loc.into();
            assert_eq!(tile, TileLocation { x: -3, y: 7 });

            // Test edge cases near zero
            let loc = Location::new(-0.1, 0.9);
            let tile: TileLocation = loc.into();
            assert_eq!(tile, TileLocation { x: -1, y: 0 });
        }
    }

    mod chunk_location {
        use super::*;

        #[test]
        fn test_chunk_neighbors() {
            let chunk = ChunkLocation { x: 0, y: 0 };

            // Test adjacent neighbors
            let adjacent = chunk.neighbors_adjacent();
            assert!(adjacent.contains(&ChunkLocation { x: 1, y: 0 }));
            assert!(adjacent.contains(&ChunkLocation { x: 0, y: -1 }));
            assert!(adjacent.contains(&ChunkLocation { x: -1, y: 0 }));
            assert!(adjacent.contains(&ChunkLocation { x: 0, y: 1 }));

            // Test diagonal neighbors
            let diagonal = chunk.neighbors_diagonal();
            assert!(diagonal.contains(&ChunkLocation { x: 1, y: 1 }));
            assert!(diagonal.contains(&ChunkLocation { x: 1, y: -1 }));
            assert!(diagonal.contains(&ChunkLocation { x: -1, y: -1 }));
            assert!(diagonal.contains(&ChunkLocation { x: -1, y: 1 }));
        }

        #[test]
        fn test_conversions() {
            // Test conversion from tile location
            let tile = TileLocation {
                x: CHUNK_SIZE * 2,
                y: CHUNK_SIZE * 3,
            };
            let chunk: ChunkLocation = tile.into();
            assert_eq!(chunk, ChunkLocation { x: 2, y: 3 });

            // Test conversion to tile location
            let tile_back: TileLocation = chunk.into();
            assert_eq!(
                tile_back,
                TileLocation {
                    x: CHUNK_SIZE * 2,
                    y: CHUNK_SIZE * 3
                }
            );
        }

        #[test]
        fn test_negative_location_to_chunk_conversions() {
            // Test whole number negative coordinates
            let loc = Location::new(5.0 * CHUNK_SIZE as f32, -11.0 * CHUNK_SIZE as f32);
            let chunk: ChunkLocation = loc.into();
            assert_eq!(chunk, ChunkLocation { x: 5, y: -11 });

            // Test fractional negative coordinates within chunk
            let loc = Location::new(-5.7 * CHUNK_SIZE as f32, -11.3 * CHUNK_SIZE as f32);
            let chunk: ChunkLocation = loc.into();
            assert_eq!(chunk, ChunkLocation { x: -6, y: -12 });

            // Test mixed positive/negative coordinates
            let loc = Location::new(-3.0 * CHUNK_SIZE as f32, 7.0 * CHUNK_SIZE as f32);
            let chunk: ChunkLocation = loc.into();
            assert_eq!(chunk, ChunkLocation { x: -3, y: 7 });

            // Test edge cases near zero
            let loc = Location::new(-0.1, 0.9 * CHUNK_SIZE as f32);
            let chunk: ChunkLocation = loc.into();
            assert_eq!(chunk, ChunkLocation { x: -1, y: 0 });
        }
    }

    mod direction {
        use super::*;

        #[test]
        fn test_direction_opposite() {
            // Test cardinal directions
            assert_eq!(Direction::N.opposite(), Direction::S);
            assert_eq!(Direction::E.opposite(), Direction::W);
            assert_eq!(Direction::S.opposite(), Direction::N);
            assert_eq!(Direction::W.opposite(), Direction::E);

            // Test ordinal directions
            assert_eq!(Direction::NE.opposite(), Direction::SW);
            assert_eq!(Direction::SE.opposite(), Direction::NW);
            assert_eq!(Direction::SW.opposite(), Direction::NE);
            assert_eq!(Direction::NW.opposite(), Direction::SE);

            // Test custom angles
            assert_eq!(Direction::Deg(0).opposite(), Direction::Deg(180));
            assert_eq!(Direction::Deg(90).opposite(), Direction::Deg(270));
            assert_eq!(Direction::Deg(180).opposite(), Direction::Deg(0));
            assert_eq!(Direction::Deg(270).opposite(), Direction::Deg(90));
            assert_eq!(Direction::Deg(45).opposite(), Direction::Deg(225));
            assert_eq!(Direction::Deg(135).opposite(), Direction::Deg(315));
        }

        #[test]
        fn test_direction_from_locations() {
            // Test cardinal directions
            let center = Location::new(0.0, 0.0);

            // North (0 degrees)
            let dir = Direction::new(center, Location::new(0.0, 1.0));
            if let Direction::Deg(angle) = dir {
                assert_eq!(angle, 0);
            } else {
                panic!("Expected Custom direction");
            }

            // East (90 degrees)
            let dir = Direction::new(center, Location::new(1.0, 0.0));
            if let Direction::Deg(angle) = dir {
                assert_eq!(angle, 90);
            }

            // South (180 degrees)
            let dir = Direction::new(center, Location::new(0.0, -1.0));
            if let Direction::Deg(angle) = dir {
                assert_eq!(angle, 180);
            }

            // West (270 degrees)
            let dir = Direction::new(center, Location::new(-1.0, 0.0));
            if let Direction::Deg(angle) = dir {
                assert_eq!(angle, 270);
            }
        }

        #[test]
        fn test_direction_rotation() {
            // Test cardinal and ordinal directions
            assert_eq!(Direction::N.rotate_cw_45(), Direction::NE);
            assert_eq!(Direction::NE.rotate_cw_45(), Direction::E);
            assert_eq!(Direction::E.rotate_cw_45(), Direction::SE);

            assert_eq!(Direction::N.rotate_ccw_45(), Direction::NW);
            assert_eq!(Direction::NW.rotate_ccw_45(), Direction::W);
            assert_eq!(Direction::W.rotate_ccw_45(), Direction::SW);

            // Test custom angles
            assert_eq!(Direction::Deg(0).rotate_cw_45(), Direction::Deg(45));
            assert_eq!(Direction::Deg(45).rotate_cw_45(), Direction::Deg(90));
            assert_eq!(Direction::Deg(315).rotate_cw_45(), Direction::Deg(0));

            assert_eq!(Direction::Deg(45).rotate_ccw_45(), Direction::Deg(0));
            assert_eq!(Direction::Deg(0).rotate_ccw_45(), Direction::Deg(315));
            assert_eq!(Direction::Deg(315).rotate_ccw_45(), Direction::Deg(270));
        }

        #[test]
        fn test_texture_index_mapping() {
            // Test cardinal directions
            assert_eq!(Direction::N.tex_index(), 0);
            assert_eq!(Direction::W.tex_index(), 2);
            assert_eq!(Direction::S.tex_index(), 4);
            assert_eq!(Direction::E.tex_index(), 6);

            // Test custom angles map to nearest cardinal direction
            if let Direction::Deg(0) = Direction::Deg(0) {
                assert_eq!(Direction::Deg(0).tex_index(), 0); // North
                assert_eq!(Direction::Deg(90).tex_index(), 6); // East
                assert_eq!(Direction::Deg(180).tex_index(), 4); // South
                assert_eq!(Direction::Deg(270).tex_index(), 2); // West
            }
        }

        #[test]
        fn test_direction_to_vector() {
            // Test cardinal directions
            assert_eq!(Direction::N.as_vec2(), (0.0, 1.0));
            assert_eq!(Direction::E.as_vec2(), (1.0, 0.0));
            assert_eq!(Direction::S.as_vec2(), (0.0, -1.0));
            assert_eq!(Direction::W.as_vec2(), (-1.0, 0.0));

            // Test diagonal directions (should be normalized)
            let diagonal = 0.7071; // Approximately 1/√2
            assert!((Direction::NE.as_vec2().0 - diagonal).abs() < 0.0001);
            assert!((Direction::NE.as_vec2().1 - diagonal).abs() < 0.0001);
        }

        #[test]
        fn test_towards_dir_custom_angles() {
            let start = Location::new(0.0, 0.0);
            let distance = 1.0;

            // Test cardinal angles
            let north = start.towards_dir(Direction::Deg(0), distance);
            assert!((north.x - 0.0).abs() < 0.0001);
            assert!((north.y - 1.0).abs() < 0.0001);

            let east = start.towards_dir(Direction::Deg(90), distance);
            assert!((east.x - 1.0).abs() < 0.0001);
            assert!((east.y - 0.0).abs() < 0.0001);

            let south = start.towards_dir(Direction::Deg(180), distance);
            assert!((south.x - 0.0).abs() < 0.0001);
            assert!((south.y - (-1.0)).abs() < 0.0001);

            let west = start.towards_dir(Direction::Deg(270), distance);
            assert!((west.x - (-1.0)).abs() < 0.0001);
            assert!((west.y - 0.0).abs() < 0.0001);

            // Test diagonal angles
            let diagonal = 1.0 / std::f32::consts::SQRT_2;
            let northeast = start.towards_dir(Direction::Deg(45), distance);
            assert!((northeast.x - diagonal).abs() < 0.0001);
            assert!((northeast.y - diagonal).abs() < 0.0001);
        }

        #[test]
        fn test_direction_rotate_deg() {
            // Test cardinal to cardinal
            assert_eq!(Direction::N.rotate_deg(90), Direction::E);
            assert_eq!(Direction::E.rotate_deg(90), Direction::S);
            assert_eq!(Direction::S.rotate_deg(90), Direction::W);
            assert_eq!(Direction::W.rotate_deg(90), Direction::N);

            // Test negative rotation
            assert_eq!(Direction::N.rotate_deg(-90), Direction::W);
            assert_eq!(Direction::E.rotate_deg(-90), Direction::N);

            // Test ordinal to ordinal
            assert_eq!(Direction::NE.rotate_deg(90), Direction::SE);
            assert_eq!(Direction::SE.rotate_deg(90), Direction::SW);

            // Test wrapping around
            assert_eq!(Direction::N.rotate_deg(360), Direction::N);
            assert_eq!(Direction::E.rotate_deg(720), Direction::E);
            assert_eq!(Direction::S.rotate_deg(-180), Direction::N);

            // Test degree to cardinal
            assert_eq!(Direction::Deg(45).rotate_deg(45), Direction::E);
            assert_eq!(Direction::Deg(0).rotate_deg(180), Direction::S);

            // Test to arbitrary degree
            assert_eq!(Direction::N.rotate_deg(30), Direction::Deg(30));
            assert_eq!(Direction::Deg(45).rotate_deg(30), Direction::Deg(75));

            // Test negative wrapping
            assert_eq!(Direction::N.rotate_deg(-90), Direction::W);
            assert_eq!(Direction::Deg(45).rotate_deg(-90), Direction::NW);
        }
    }
}
