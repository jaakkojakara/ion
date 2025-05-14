use core::convert::TryInto;
use core::default::Default;
use core::ops::BitXor;
use std::cell::Cell;
use std::hash::Hasher;

const HASH_MULTIPLIER: u64 = 0x517cc1b727220a95;

/// A fast, non-cryptographic hash implementation optimized for integer keys.
///
/// This hash implementation is inspired by the ones used in Firefox and the Rust compiler.
/// It provides extremely fast hashing performance, particularly for integer keys, but does not
/// offer any protection against hash collision attacks (DOS resistance).
///
/// ## Performance Characteristics
/// - Optimized for integer keys
/// - Very fast hashing speed
/// - No DOS resistance
/// - Deterministic across all platforms (32-bit, 64-bit, WASM)
#[derive(Debug, Default)]
pub struct FastHash {
    hash: Cell<u64>,
}

impl FastHash {
    #[inline]
    pub fn hash(bytes: &[u8]) -> u64 {
        let hasher = Self::default();
        hasher.add_bytes(bytes);
        hasher.get()
    }

    #[inline]
    pub fn get(&self) -> u64 {
        self.hash.get()
    }

    #[inline]
    fn add_to_hash(&self, i: u64) {
        self.hash
            .set(self.hash.take().rotate_left(5).bitxor(i).wrapping_mul(HASH_MULTIPLIER));
    }

    #[inline]
    pub fn add_bytes(&self, mut bytes: &[u8]) {
        let read_u64 = |bytes: &[u8]| u64::from_ne_bytes(bytes[..8].try_into().unwrap());
        let hash = FastHash {
            hash: self.hash.clone(),
        };
        while bytes.len() >= 8 {
            hash.add_to_hash(read_u64(bytes));
            bytes = &bytes[8..];
        }
        if bytes.len() >= 4 {
            hash.add_to_hash(u32::from_ne_bytes(bytes[..4].try_into().unwrap()) as u64);
            bytes = &bytes[4..];
        }
        if bytes.len() >= 2 {
            hash.add_to_hash(u16::from_ne_bytes(bytes[..2].try_into().unwrap()) as u64);
            bytes = &bytes[2..];
        }
        if !bytes.is_empty() {
            hash.add_to_hash(bytes[0] as u64);
        }
        self.hash.swap(&hash.hash);
    }

    #[inline]
    pub fn add_u8(&self, i: u8) {
        self.add_to_hash(i as u64);
    }

    #[inline]
    pub fn add_u16(&self, i: u16) {
        self.add_to_hash(i as u64);
    }

    #[inline]
    pub fn add_u32(&self, i: u32) {
        self.add_to_hash(i as u64);
    }

    #[inline]
    pub fn add_u64(&self, i: u64) {
        self.add_to_hash(i);
    }

    #[inline]
    pub fn add_f32(&self, f: f32) {
        self.add_bytes(&f.to_ne_bytes())
    }

    #[inline]
    pub fn add_f64(&self, f: f64) {
        self.add_bytes(&f.to_ne_bytes())
    }

    #[inline]
    pub fn add_usize(&self, i: usize) {
        self.add_to_hash(i as u64);
    }
}

impl Hasher for FastHash {
    #[inline]
    fn finish(&self) -> u64 {
        self.get()
    }

    #[inline]
    fn write(&mut self, bytes: &[u8]) {
        self.add_bytes(bytes);
    }

    #[inline]
    fn write_u8(&mut self, i: u8) {
        self.add_u8(i);
    }

    #[inline]
    fn write_u16(&mut self, i: u16) {
        self.add_u16(i);
    }

    #[inline]
    fn write_u32(&mut self, i: u32) {
        self.add_u32(i);
    }

    #[inline]
    fn write_u64(&mut self, i: u64) {
        self.add_u64(i);
    }

    #[inline]
    fn write_usize(&mut self, i: usize) {
        self.add_usize(i);
    }
}

// ---------------------------------------------------------- //
// ------------------------- Tests -------------------------- //
// ---------------------------------------------------------- //

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::time::{Duration, Instant};

    use crate::Map;
    use crate::math::rand::Rng;

    use super::*;

    #[test]
    fn hashing_works() {
        let hash = FastHash::default();

        hash.add_f32(0.455435);
        hash.add_u32(782783);
        hash.add_bytes(&[2, 4, 6, 4, 2, 0]);

        hash.get();
    }

    #[test]
    fn hashing_is_stable() {
        let hash1 = FastHash::default();
        let hash2 = FastHash::default();

        hash1.add_f32(0.455435);
        hash1.add_u32(782783);
        hash1.add_bytes(&[2, 4, 6, 4, 2, 0]);

        hash2.add_f32(0.455435);
        hash2.add_u32(782783);
        hash2.add_bytes(&[2, 4, 6, 4, 2, 0]);

        assert_eq!(hash1.get(), hash2.get())
    }

    #[test]
    fn fast_hash_maps_are_faster_than_rust_std_maps() {
        let mut rng = Rng::new(None);

        let mut hash_map: HashMap<u64, u64> = HashMap::default();
        let mut fast_map: Map<u64, u64> = Map::default();

        let mut hp_time_spent = Duration::default();
        let mut fp_time_spent = Duration::default();

        for _ in 0..10000 {
            let val1 = rng.gen_u64();
            let val2 = rng.gen_u64();

            let t1 = Instant::now();
            hash_map.insert(val1, val2);
            let t2 = Instant::now();

            hp_time_spent += t2 - t1;

            let t3 = Instant::now();
            fast_map.insert(val1, val2);
            let t4 = Instant::now();

            fp_time_spent += t4 - t3;
        }

        assert!(hp_time_spent > fp_time_spent);
    }
}
