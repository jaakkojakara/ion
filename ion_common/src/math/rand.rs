use std::time::{SystemTime, UNIX_EPOCH};

/// A Pcg64Mcg-based PRNG (Pseudo-Random Number Generator) implementation.
///
/// This is a fast, high-quality random number generator based on the PCG (Permuted Congruential Generator)
/// family, specifically using the MCG (Multiplicative Congruential Generator) variant with 128-bit state.
/// While it provides good statistical properties and is suitable for most general-purpose random number
/// generation needs, it is NOT cryptographically secure.
///
/// # Features
/// * Fast generation of random numbers
/// * Good statistical properties
/// * Support for various number types (u32, u64, f32, f64)
/// * Range-based random number generation
/// * Byte array filling
///
/// # Security Note
/// This RNG is not suitable for cryptographic purposes. For cryptographic applications,
/// use a cryptographically secure RNG like `rand::rngs::OsRng`.
pub struct Rng {
    state: u128,
    mult: u128,
}

impl Rng {
    pub fn new(seed: Option<u128>) -> Self {
        let seed = seed.unwrap_or_else(|| {
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        });
        let seed_div: [u64; 2] = [seed as u64, (seed >> 64) as u64];
        Self {
            state: u128::from(seed_div[0]) | u128::from(seed_div[1]) << 64,
            mult: 0x2360_ED05_1FC6_5DA4_4385_DF64_9FCC_F645,
        }
    }

    pub fn gen_u64(&mut self) -> u64 {
        self.state = self.state.wrapping_mul(self.mult);

        const XSHIFT: u32 = 64;
        const ROTATE: u32 = 122;

        let rot = (self.state >> ROTATE) as u32;
        let xsl = ((self.state >> XSHIFT) as u64) ^ (self.state as u64);
        xsl.rotate_right(rot)
    }

    pub fn gen_range_u64(&mut self, min: u64, max: u64) -> u64 {
        self.gen_u64() % (max - min) + min
    }

    pub fn gen_u32(&mut self) -> u32 {
        self.gen_u64() as u32
    }

    pub fn gen_range_u32(&mut self, min: u32, max: u32) -> u32 {
        self.gen_u32() % (max - min) + min
    }

    pub fn gen_f64(&mut self) -> f64 {
        let r_u64 = self.gen_u64();
        r_u64 as f64 / u64::MAX as f64
    }

    pub fn gen_range_f64(&mut self, min: f64, max: f64) -> f64 {
        self.gen_f64() * (max - min) + min
    }

    pub fn gen_f32(&mut self) -> f32 {
        let r_u32 = self.gen_u32();
        r_u32 as f32 / u32::MAX as f32
    }

    pub fn gen_range_f32(&mut self, min: f32, max: f32) -> f32 {
        self.gen_f32() * (max - min) + min
    }

    pub fn fill_random_bytes(&mut self, target: &mut [u8]) {
        for chunk in target.chunks_mut(8) {
            let random = self.gen_u64();
            for (i, byte) in chunk.iter_mut().enumerate() {
                *byte = ((random >> (i * 8)) & 0xFF) as u8;
            }
        }
    }
}

// ---------------------------------------------------------- //
// ------------------------- Tests -------------------------- //
// ---------------------------------------------------------- //

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rng_generates_random_numbers() {
        let mut rng = Rng::new(None);

        for _ in 0..10 {
            rng.gen_u64();
            rng.gen_u32();
        }
    }

    #[test]
    fn rng_float_generation_works() {
        let mut rng = Rng::new(None);
        let mut avg = 0.0;
        for _ in 0..1000000 {
            avg += rng.gen_f64();
        }

        avg /= 1000000.0;

        assert!(avg < 0.501 && avg > 0.499);
    }

    #[test]
    fn rng_in_range_works() {
        let mut rng = Rng::new(None);
        for _ in 0..10000 {
            let r_f32 = rng.gen_range_f32(-34.0, 786.0);
            assert!((-34.0..786.0).contains(&r_f32));
            let r_u32 = rng.gen_range_u32(2, 23);
            assert!((2..23).contains(&r_u32));
        }
    }

    #[test]
    fn rng_seed_generates_deterministic_sequence() {
        let mut rng_1 = Rng::new(Some(66733));
        let mut rng_2 = Rng::new(Some(66733));

        for _ in 0..100 {
            let r1 = rng_1.gen_u64();
            let r2 = rng_2.gen_u64();
            assert_eq!(r1, r2);
        }
    }

    #[test]
    fn rng_fill_random_bytes_works() {
        let mut rng = Rng::new(None);
        let mut target = [0; 10];

        assert_eq!(target.iter().map(|v| *v as u32).sum::<u32>(), 0);

        rng.fill_random_bytes(&mut target);

        assert_ne!(target.iter().map(|v| *v as u32).sum::<u32>(), 0);
    }
}
