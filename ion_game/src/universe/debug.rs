use ion_engine::{
    core::{CHUNK_SIZE, coordinates::Location},
    gfx::{Color, DebugLine, DebugShape, GfxDebugData},
};

use crate::universe::world::World;

#[derive(Debug, Default)]
pub struct DebugConfig {
    pub debug_sys_enabled: bool,

    pub draw_chunk_borders: bool,
    pub draw_tile_borders: bool,

    pub debug_cursor_enabled: bool,
    pub debug_cursor_loc: Location,

    pub draw_nav_ports: bool,
    pub draw_nav_port_targets: bool,
    pub draw_nav_port_connections: bool,
    pub draw_nav_tiles: bool,
    pub draw_flow_field: bool,
    pub show_noise: bool,
}

impl DebugConfig {
    #[allow(dead_code)]
    pub fn reset(&mut self) {
        self.draw_chunk_borders = false;
        self.draw_tile_borders = false;
        self.draw_flow_field = false;
        self.draw_nav_ports = false;
        self.draw_nav_port_targets = false;
        self.draw_nav_port_connections = false;
        self.draw_nav_tiles = false;
        self.show_noise = false;
        self.debug_cursor_loc = Location::default();
        self.debug_cursor_enabled = false;
    }
}

pub fn build_gfx_debug_data(world: &World) -> GfxDebugData {
    if !world.debug_config.debug_sys_enabled {
        return GfxDebugData {
            debug_shapes: vec![],
            debug_labels: vec![],
        };
    }

    let chunks_to_render = world.camera.chunks_visible(&world.chunks);
    let mut debug_shapes: Vec<Box<dyn DebugShape>> = vec![];
    //let mut debug_labels: Vec<(String, Location)> = vec![];

    if world.debug_config.draw_tile_borders {
        for chunk_loc in &chunks_to_render {
            let loc: Location = (*chunk_loc).into();
            for i in 1..32 {
                debug_shapes.push(Box::new(DebugLine {
                    start: loc.update(i as f32, 0.0),
                    end: loc.update(i as f32, CHUNK_SIZE as f32),
                    color: Color { r: 0, g: 100, b: 100 },
                }));
                debug_shapes.push(Box::new(DebugLine {
                    start: loc.update(0.0, i as f32),
                    end: loc.update(CHUNK_SIZE as f32, i as f32),
                    color: Color { r: 0, g: 100, b: 100 },
                }));
            }
        }
    }

    if world.debug_config.draw_chunk_borders {
        for chunk_loc in &chunks_to_render {
            let loc: Location = (*chunk_loc).into();
            debug_shapes.push(Box::new(DebugLine {
                start: loc,
                end: loc.update(CHUNK_SIZE as f32, 0.0),
                color: Color { r: 0, g: 255, b: 255 },
            }));
            debug_shapes.push(Box::new(DebugLine {
                start: loc,
                end: loc.update(0.0, CHUNK_SIZE as f32),
                color: Color { r: 0, g: 255, b: 255 },
            }));
        }
    }

    GfxDebugData {
        debug_shapes,
        debug_labels: vec![],
    }
}
