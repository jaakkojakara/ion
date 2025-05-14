use bincode::{Decode, Encode};
use std::fmt::Debug;
use std::hash::Hash;

pub mod entity_creators;
pub mod mobs;
pub mod structures;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Encode, Decode)]
pub struct Entity {
    index: u32,
    generation: u32,
}

impl Entity {
    pub fn index(&self) -> u32 {
        self.index
    }
}

#[derive(Debug, Clone, Encode, Decode)]
pub struct EntityHandler {
    free_indices: Vec<usize>,
    generations: Vec<u32>,
}

impl EntityHandler {
    pub fn new() -> Self {
        Self {
            free_indices: Vec::new(),
            generations: Vec::new(),
        }
    }

    fn next_id(&mut self) -> Entity {
        let index = self.free_indices.pop().unwrap_or_else(|| {
            let index = self.generations.len();
            self.generations.push(0);
            index
        });

        Entity {
            index: index as u32,
            generation: self.generations[index],
        }
    }

    fn delete_id(&mut self, id: Entity) -> bool {
        if self.is_valid(id) {
            self.generations[id.index() as usize] = self.generations[id.index() as usize].wrapping_add(1);
            self.free_indices.push(id.index() as usize);
            true
        } else {
            false
        }
    }

    fn is_valid(&self, id: Entity) -> bool {
        id.generation == *self.generations.get(id.index() as usize).unwrap_or(&u32::MAX)
    }

    fn iter_ids(&self) -> impl Iterator<Item = Entity> {
        self.generations.iter().enumerate().map(|(index, generation)| Entity {
            index: index as u32,
            generation: *generation,
        })
    }

    /// Iterates only over valid (non-deleted) entities
    pub fn iter_ids_valid(&self) -> impl Iterator<Item = Entity> {
        let free_set: std::collections::HashSet<usize> = self.free_indices.iter().cloned().collect();
        self.generations
            .iter()
            .enumerate()
            .filter_map(move |(index, generation)| {
                if free_set.contains(&index) {
                    None // This index is free (deleted entity)
                } else {
                    Some(Entity {
                        index: index as u32,
                        generation: *generation,
                    })
                }
            })
    }

    /// Iterates over all entities, including deleted ones. Faster than filtering out deleted ones
    #[allow(dead_code)]
    fn iter_ids_unchecked(&self) -> impl Iterator<Item = Entity> {
        self.generations.iter().enumerate().map(|(index, generation)| Entity {
            index: index as u32,
            generation: *generation,
        })
    }
}
