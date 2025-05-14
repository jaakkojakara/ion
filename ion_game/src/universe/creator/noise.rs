use bincode::{Decode, Encode};

use ion_engine::core::coordinates::TileLocation;

const STRETCH: f32 = -0.211_324_87;
// (1 / sqrt(2 + 1) - 1) / 2
const SQUISH: f32 = 0.366_025_42;
// (sqrt(2 + 1) - 1) / 2
const STRETCH_POINT: [f32; 2] = [STRETCH, STRETCH];
const SQUISH_POINT: [f32; 2] = [SQUISH, SQUISH];

const NORMALIZING_SCALAR: f32 = 47.0;

const GRADIENTS: [[f32; 2]; 8] = [
    [5.0, 2.0],
    [2.0, 5.0],
    [-5.0, 2.0],
    [-2.0, 5.0],
    [5.0, -2.0],
    [2.0, -5.0],
    [-5.0, -2.0],
    [-2.0, -5.0],
];

pub const PSIZE: i64 = 2048;

type PermTable = [i64; PSIZE as usize];

#[derive(Debug, Encode, Decode)]
pub struct Noise {
    seed: u64,
    scale: f32,
    octaves: u32,
    persistence: f32,
    perm_table: PermTable,
}

impl Noise {
    #[allow(dead_code)]
    /// Creates a new noise generator.
    ///
    /// # Arguments
    ///
    /// * `seed` - The seed for the noise generator.
    /// * `scale` - The scale of the noise.
    /// * `octaves` - The number of different noise sized noise layers to sum.
    /// * `persistence` - How much each successive noise layer contributes to the total noise. 2.0 means each layer contributes twice as much as the previous layer.
    pub fn new(seed: u64, scale: f32, octaves: u32, persistence: f32) -> Self {
        let mut perm_table: PermTable = [0; PSIZE as usize];
        let mut source: Vec<i64> = (0..PSIZE).collect();
        let seed2: i128 = (seed as i128 * 6_364_136_223_846_793_005) + 1_442_695_040_888_963_407;
        for i in (0..PSIZE).rev() {
            let mut r = ((seed2 + 31) % (i as i128 + 1)) as i64;
            if r < 0 {
                r += i + 1;
            }
            perm_table[i as usize] = source[r as usize];
            source[r as usize] = source[i as usize];
        }

        Self {
            seed,
            scale,
            octaves,
            persistence,
            perm_table,
        }
    }

    #[allow(dead_code)]
    /// Returns the seed used to generate the noise.
    pub fn seed(&self) -> u64 {
        self.seed
    }

    #[allow(dead_code)]
    /// Calculates the noise value at the given location.
    /// The noise value is normalized to the range [0, 1].
    pub fn at(&self, location: TileLocation) -> f32 {
        let mut total: f32 = 0.0;
        let mut frequency: f32 = 1.0;
        let mut amplitude: f32 = 1.0;
        let mut max_value: f32 = 0.0;

        for _ in 0..self.octaves {
            total += calc_open_simplex(location, self.scale * frequency, &self.perm_table) * amplitude;
            max_value += amplitude;
            amplitude *= self.persistence;
            frequency *= 2.0;
        }

        (total / max_value + 1.0) / 2.0
    }
}

// Helper functions for array operations
fn dot(a: [f32; 2], b: [f32; 2]) -> f32 {
    a[0] * b[0] + a[1] * b[1]
}

fn add(a: [f32; 2], b: [f32; 2]) -> [f32; 2] {
    [a[0] + b[0], a[1] + b[1]]
}

fn sub(a: [f32; 2], b: [f32; 2]) -> [f32; 2] {
    [a[0] - b[0], a[1] - b[1]]
}

fn mul_scalar(a: [f32; 2], scalar: f32) -> [f32; 2] {
    [a[0] * scalar, a[1] * scalar]
}

fn floor(a: [f32; 2]) -> [f32; 2] {
    [a[0].floor(), a[1].floor()]
}

fn calc_open_simplex(coordinates: TileLocation, scale: f32, perm_table: &PermTable) -> f32 {
    fn contribute(delta: [f32; 2], origin: [f32; 2], grid: [f32; 2], perm: &PermTable) -> f32 {
        let shifted: [f32; 2] = sub(origin, add(delta, mul_scalar(SQUISH_POINT, delta[0] + delta[1])));
        let attn: f32 = 2.0 - dot(shifted, shifted);
        if attn > 0.0 {
            let delta_grid = add(grid, delta);
            let index0 = ((perm[(delta_grid[0] as i64 & 0xFF) as usize] + delta_grid[1] as i64) & 0xFF) as usize;
            let grad_index = ((perm[index0] & 0x0E) >> 1) as usize;
            let gradient_dot = dot(GRADIENTS[grad_index], shifted);
            attn.powi(4) * gradient_dot
        } else {
            0.0
        }
    }

    fn evaluate_inside_triangle(ins: [f32; 2], contribute: impl Fn(f32, f32) -> f32) -> f32 {
        let in_sum = ins[0] + ins[1];
        let factor_point = match in_sum {
            x if x <= 1.0 => [0.0, 0.0],
            _ => [1.0, 1.0],
        };
        let zins = 1.0 + factor_point[0] - in_sum;
        let point = if zins > ins[0] || zins > ins[1] {
            // (0, 0) is one of the closest two triangular vertices
            if ins[0] > ins[1] {
                [1.0 + factor_point[0], -1.0 + factor_point[1]]
            } else {
                [-1.0 + factor_point[0], 1.0 + factor_point[1]]
            }
        } else {
            // (1, 0) and (0, 1) are the closest two vertices.
            [1.0 - factor_point[0], 1.0 - factor_point[1]]
        };
        contribute(0.0 + factor_point[0], 0.0 + factor_point[1]) + contribute(point[0], point[1])
    }

    let normalized_x = (coordinates.x as f32 + 24768.0) / 32.0 / scale; // Add large value to offset [0,0] coordinates where noise produces artefacts
    let normalized_y = (coordinates.y as f32 + 24768.0) / 32.0 / scale; // Add large value to offset [0,0] coordinates where noise produces artefacts
    let normalized_input = [normalized_x, normalized_y];
    let stretch: [f32; 2] = add(
        normalized_input,
        mul_scalar(STRETCH_POINT, normalized_input[0] + normalized_input[1]),
    );
    let grid = floor(stretch);
    let squashed: [f32; 2] = add(grid, mul_scalar(SQUISH_POINT, grid[0] + grid[1]));
    let ins = sub(stretch, grid);
    let origin = sub(normalized_input, squashed);

    let contribute = |x, y| -> f32 { contribute([x, y], origin, grid, perm_table) };

    let value = contribute(1.0, 0.0) + contribute(0.0, 1.0) + evaluate_inside_triangle(ins, contribute);

    value / NORMALIZING_SCALAR
}

#[allow(dead_code)]
pub fn noise_to_color_slot(noise_val: f32) -> u32 {
    (((noise_val + 1.0) / 2.0) * 255.0) as u32
}

#[cfg(test)]
mod tests {
    use std::fs;

    use image::ImageBuffer;

    use super::*;

    #[test]
    fn can_generate_noise_for_positive_and_negative_coords() {
        let noise = Noise::new(123, 0.5, 5, 2.2);
        for x in -64..64 {
            for y in -64..64 {
                noise.at(TileLocation { x, y });
            }
        }
    }

    #[test]
    fn noise_changes_smoothly() {
        let noise = Noise::new(123, 1.0, 1, 1.0);
        let mut noise_vals: Vec<f32> = Vec::new();

        for x in -64..64 {
            for y in -64..64 {
                noise_vals.push(noise.at(TileLocation { x, y }));
            }
        }

        for x in 1..127 {
            for y in 1..127 {
                let i = x * 128 + y;
                let i_val = noise_vals[i];
                assert!((noise_vals[i - 1] - i_val).abs() < 0.25);
                assert!((noise_vals[i + 1] - i_val).abs() < 0.25);
                assert!((noise_vals[i - 128] - i_val).abs() < 0.25);
                assert!((noise_vals[i + 128] - i_val).abs() < 0.25);
            }
        }
    }

    #[test]
    #[ignore]
    /// This test writes an image of the noise pattern to "target/test/noise.png"
    fn noise_output_looks_correct() {
        let noise = Noise::new(123, 1.0, 5, 1.6);
        let noise_img = ImageBuffer::from_fn(1024, 1024, |x, y| {
            let x_u = x as i16 - 64;
            let y_u = y as i16 - 64;
            let noise_val = noise.at(TileLocation { x: x_u, y: y_u });

            let pixel_value: u8 = (((noise_val + 1.0) / 2.0) * 255.0) as u8;
            image::Luma([pixel_value])
        });

        fs::create_dir_all("target/test").unwrap();
        noise_img.save("target/test/noise.png").unwrap();
    }
}
