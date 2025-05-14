use ion_engine::{
    core::coordinates::{Direction, Location},
    gfx::GfxRef,
};

use crate::{
    assets,
    universe::entities::{
        mobs::{MobClone, MobType},
        structures::{StructureClone, StructureSize, StructureType},
    },
};

pub fn create_player(loc: Location) -> MobClone {
    MobClone {
        mob_type: MobType::Player,
        loc,
        dir: Direction::N,
        speed: 0.0,
    }
}

#[allow(dead_code)]
pub fn create_torus(loc: Location) -> (StructureClone, GfxRef) {
    (
        StructureClone {
            structure_type: StructureType::Torus,
            loc,
            size: StructureSize { width: 4, height: 4 },
        },
        GfxRef::new(assets::gfx_ref("torus"), loc),
    )
}

#[allow(dead_code)]
pub fn create_lamp(loc: Location) -> (StructureClone, GfxRef) {
    (
        StructureClone {
            structure_type: StructureType::Lamp,
            loc,
            size: StructureSize { width: 1, height: 1 },
        },
        GfxRef::new(assets::gfx_ref("lamp"), loc),
    )
}

#[allow(dead_code)]
pub fn create_bollard(loc: Location) -> (StructureClone, GfxRef) {
    (
        StructureClone {
            structure_type: StructureType::Bollard,
            loc,
            size: StructureSize { width: 1, height: 1 },
        },
        GfxRef::new(assets::gfx_ref("bollard"), loc),
    )
}

#[allow(dead_code)]
pub fn create_tree(loc: Location) -> (StructureClone, GfxRef) {
    (
        StructureClone {
            structure_type: StructureType::Tree,
            loc,
            size: StructureSize { width: 1, height: 1 },
        },
        GfxRef::new(assets::gfx_ref("tree_big"), loc),
    )
}
