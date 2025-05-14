use crate::math::hash::FastHash;
use std::collections::{HashMap, HashSet};
use std::hash::BuildHasherDefault;

pub use bincode;

pub use js_sys;
pub use wasm_bindgen;
pub use web_sys;

pub use util::log::*;
pub use util::time::{DateTime, Instant};

pub mod math;
pub mod net;
pub mod util;

// ---------------------------------------------------------- //
// ---------------- Player and Server ids ------------------- //
// ---------------------------------------------------------- //

pub type PlayerId = u32;
pub type ServerId = u32;

// ---------------------------------------------------------- //
// ------------- Fast hash-based collections ---------------- //
// ---------------------------------------------------------- //

/// A hash map with a fast hash function. See [`FastHash`] for more details.
pub type Map<K, V> = HashMap<K, V, BuildHasherDefault<FastHash>>;

/// A hash set with a fast hash function. See [`FastHash`] for more details.
pub type Set<K> = HashSet<K, BuildHasherDefault<FastHash>>;
