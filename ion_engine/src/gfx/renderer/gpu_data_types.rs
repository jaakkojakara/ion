use std::mem;

use derive_engine::RawData;
use wgpu::vertex_attr_array;

use crate::{gfx::textures::TextureLayout, util::casting::RawData};

// ---------------------------------------------------------- //
// ----------------------- GPU shaders ---------------------- //
// ---------------------------------------------------------- //

pub const SHADER_SCALE: &str = include_str!("shaders/shader_scale.wgsl");
pub const SHADER_DEBUG: &str = include_str!("shaders/shader_line.wgsl");
pub const SHADER_GBUF: &str = include_str!("shaders/shader_gbuf.wgsl");
pub const SHADER_GBUF_WASM: &str = include_str!("shaders/shader_gbuf_wasm.wgsl");
pub const SHADER_LIGHT: &str = include_str!("shaders/shader_light.wgsl");
pub const SHADER_LIGHT_WASM: &str = include_str!("shaders/shader_light_wasm.wgsl");
pub const SHADER_SHADOW: &str = include_str!("shaders/shader_shadow.wgsl");
pub const SHADER_SHADOW_WASM: &str = include_str!("shaders/shader_shadow_wasm.wgsl");
pub const SHADER_POST_1: &str = include_str!("shaders/shader_post_1.wgsl");
pub const SHADER_POST_2: &str = include_str!("shaders/shader_post_2.wgsl");
pub const SHADER_BLOOM_DS: &str = include_str!("shaders/shader_bloom_ds.wgsl");
pub const SHADER_BLOOM_US: &str = include_str!("shaders/shader_bloom_us.wgsl");
pub const SHADER_SSAO_RAW: &str = include_str!("shaders/shader_ssao_raw.wgsl");
pub const SHADER_SSAO_BLUR: &str = include_str!("shaders/shader_ssao_blur.wgsl");

// ---------------------------------------------------------- //
// --------------- GPU-supported data models ---------------- //
// ---------------------------------------------------------- //

// These types use stable c-presentation, and can be copied over to GPU.
#[repr(C)]
#[derive(Debug, Clone, Copy, RawData)]
pub(crate) struct Vertex {
    /// Position of the vertex in the sprite before transformations
    pub position: [f32; 3],
    /// Offsets in range of 0.0 to 1.0 for the for a texture that exactly covers this polygon
    pub tex_coord_offset: [f32; 2],
    /// Tells the shader how to handle position offsets for this vertex
    /// x = layout where 1 = Square/Overlay, 2 = Iso, 3 = IsoHex bottom, 4 = IsoHex top
    /// y = flag whether vertex should be affected by y-scaling of the sprite
    pub tex_layout_flags: [u32; 2],
}

const VERTEX_ATTRIBUTES: [wgpu::VertexAttribute; 3] = vertex_attr_array![
    0 => Float32x3,
    1 => Float32x2,
    2 => Uint32x2,
];

impl Vertex {
    pub fn buffer_layout<'a>() -> wgpu::VertexBufferLayout<'a> {
        wgpu::VertexBufferLayout {
            array_stride: mem::size_of::<Vertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &VERTEX_ATTRIBUTES,
        }
    }
}

#[repr(C)]
#[derive(Debug, Clone, Copy, RawData)]
pub struct LineVertex {
    pub(crate) loc: [f32; 3],
    pub(crate) color: [f32; 3],
}

const LINE_VERTEX_ATTRIBUTES: [wgpu::VertexAttribute; 2] = vertex_attr_array![
    0 => Float32x3,
    1 => Float32x3,
];

impl LineVertex {
    pub fn buffer_layout<'a>() -> wgpu::VertexBufferLayout<'a> {
        wgpu::VertexBufferLayout {
            array_stride: mem::size_of::<LineVertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &LINE_VERTEX_ATTRIBUTES,
        }
    }
}

#[repr(C)]
#[derive(Debug, Clone, Copy, RawData)]
pub(crate) struct InstanceSprite {
    /// Location of the sprite in world coordinates
    pub loc: [f32; 2],
    /// Rotation of the sprite in radians
    pub rot: f32,
    /// Scale multiplier for the sprite
    pub scale: f32,
    /// Animation data
    /// Contains four u8 values packed into a u32
    pub anim_data: u32,
    /// Texture coordinates of the sprite
    pub tex_coords: [f32; 2],
    /// Texture coordinates of the optional sprite mask
    pub tex_coords_mask: [f32; 2],
    /// Size of the texture in fraction of the texture sheet size
    pub tex_coords_sizes: [f32; 2],
    /// Texture sheet index
    /// Contains three u8 values + one u8 padding packed into a u32
    pub tex_sheet_indices: u32,
    /// Type id of the sprite
    /// Id between 0.0 and 1.0, so that it can be stored as color
    pub type_id: f32,
}

const INSTANCE_SPRITE_ATTRIBUTES: [wgpu::VertexAttribute; 9] = vertex_attr_array![
    6 => Float32x2,
    7 => Float32,
    8 => Float32,
    9 => Uint32,
    10 => Float32x2,
    11 => Float32x2,
    12 => Float32x2,
    13 => Uint32,
    14 => Float32,
];

impl InstanceSprite {
    pub fn buffer_layout<'a>() -> wgpu::VertexBufferLayout<'a> {
        wgpu::VertexBufferLayout {
            array_stride: mem::size_of::<InstanceSprite>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Instance,
            attributes: &INSTANCE_SPRITE_ATTRIBUTES,
        }
    }
}

#[repr(C)]
#[derive(Copy, Clone, Debug, RawData)]
pub(crate) struct InstanceLight {
    /// Location of the light sprite in world coordinates
    pub loc_sprite: [f32; 2],
    /// Location of the light (not sprite) in world coordinates.
    /// This is the real source of the light for the directional lighting effects.
    /// Note that this actually contains the height-coordinate of the light, which affects the result.
    pub loc_light: [f32; 3],
    /// Rotation of the light sprite in radians
    pub rot: f32,
    /// Scale multiplier for the light sprite
    pub scale: f32,
    /// Strength of the light
    pub strength: f32,
    /// Texture coordinates of the light sprite
    pub tex_coords: [f32; 2],
    /// Size of the texture in fraction of the texture sheet size
    pub tex_coords_sizes: [f32; 2],
    /// Texture sheet index
    /// Contains three u8 values + one u8 padding packed into a u32
    pub tex_sheet_index: u32,
}

const INSTANCE_LIGHT_ATTRIBUTES: [wgpu::VertexAttribute; 8] = vertex_attr_array![
    6 => Float32x2,
    7 => Float32x3,
    8 => Float32,
    9 => Float32,
    10 => Float32,
    11 => Float32x2,
    12 => Float32x2,
    13 => Uint32,
];

impl InstanceLight {
    pub fn buffer_layout<'a>() -> wgpu::VertexBufferLayout<'a> {
        wgpu::VertexBufferLayout {
            array_stride: mem::size_of::<InstanceLight>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Instance,
            attributes: &INSTANCE_LIGHT_ATTRIBUTES,
        }
    }
}

// ---------------------------------------------------------- //
// --------------- Vertex geometry generators --------------- //
// ---------------------------------------------------------- //

pub(crate) fn build_vertex_vec(camera_cos: f32) -> Vec<Vertex> {
    vec![square_vertices(camera_cos), isometric_vertices(), isometric_hex_vertices(), overlay_vertices()]
        .into_iter()
        .flatten()
        .collect()
}

pub(crate) fn build_index_vec() -> Vec<u16> {
    vec![
        TextureLayout::Square.build_indices(0),
        TextureLayout::Isometric.build_indices(4),
        TextureLayout::IsometricHex.build_indices(8),
        TextureLayout::Overlay.build_indices(14),
    ]
    .into_iter()
    .flatten()
    .collect()
}

fn square_vertices(camera_cos: f32) -> Vec<Vertex> {
    let x_y_scale = camera_cos;
    let sqrt_2 = std::f32::consts::SQRT_2;
    let cos_mult = std::f32::consts::FRAC_1_SQRT_2;
    let cos_mult_half = cos_mult / 2.0;
    vec![
        Vertex {
            position: [
                (cos_mult_half + cos_mult / x_y_scale) * sqrt_2,
                (-cos_mult_half + cos_mult / x_y_scale) * sqrt_2,
                0.,
            ],
            tex_coord_offset: [1.0, 0.0],
            tex_layout_flags: [1, 1],
        },
        Vertex {
            position: [
                (-cos_mult_half + cos_mult / x_y_scale) * sqrt_2,
                (cos_mult_half + cos_mult / x_y_scale) * sqrt_2,
                0.,
            ],
            tex_coord_offset: [0.0, 0.0],
            tex_layout_flags: [1, 1],
        },
        Vertex {
            position: [-sqrt_2 * cos_mult_half, sqrt_2 * cos_mult_half, 0.],
            tex_coord_offset: [0.0, 1.0],
            tex_layout_flags: [1, 0],
        },
        Vertex {
            position: [sqrt_2 * cos_mult_half, -sqrt_2 * cos_mult_half, 0.],
            tex_coord_offset: [1.0, 1.0],
            tex_layout_flags: [1, 0],
        },
    ]
}

fn isometric_vertices() -> Vec<Vertex> {
    vec![
        Vertex {
            position: [1.0, 1.0, 0.],
            tex_coord_offset: [0.5, 0.0],
            tex_layout_flags: [2, 0],
        },
        Vertex {
            position: [0., 1.0, 0.],
            tex_coord_offset: [0.0, 0.5],
            tex_layout_flags: [2, 0],
        },
        Vertex {
            position: [0., 0., 0.],
            tex_coord_offset: [0.5, 1.0],
            tex_layout_flags: [2, 0],
        },
        Vertex {
            position: [1.0, 0., 0.],
            tex_coord_offset: [1.0, 0.5],
            tex_layout_flags: [2, 0],
        },
    ]
}

fn isometric_hex_vertices() -> Vec<Vertex> {
    vec![
        Vertex {
            position: [1.0, 1.0, 0.],
            tex_coord_offset: [0.5, 0.0],
            tex_layout_flags: [3, 0],
        },
        Vertex {
            position: [0., 1.0, 0.],
            tex_coord_offset: [0.0, 0.0],
            tex_layout_flags: [3, 1],
        },
        Vertex {
            position: [1.0, 0., 0.],
            tex_coord_offset: [1.0, 0.0],
            tex_layout_flags: [3, 1],
        },
        Vertex {
            position: [0., 1.0, 0.],
            tex_coord_offset: [0.0, 0.0],
            tex_layout_flags: [4, 1],
        },
        Vertex {
            position: [0., 0., 0.],
            tex_coord_offset: [0.5, 1.0],
            tex_layout_flags: [4, 0],
        },
        Vertex {
            position: [1.0, 0., 0.],
            tex_coord_offset: [1.0, 0.0],
            tex_layout_flags: [4, 1],
        },
    ]
}

fn overlay_vertices() -> Vec<Vertex> {
    vec![
        Vertex {
            position: [1.0, 0.0, 0.],
            tex_coord_offset: [1.0, 0.0],
            tex_layout_flags: [1, 1],
        },
        Vertex {
            position: [0.0, 0.0, 0.],
            tex_coord_offset: [0.0, 0.0],
            tex_layout_flags: [1, 1],
        },
        Vertex {
            position: [0.0, -1.0, 0.],
            tex_coord_offset: [0.0, 1.0],
            tex_layout_flags: [1, 0],
        },
        Vertex {
            position: [1.0, -1.0, 0.],
            tex_coord_offset: [1.0, 1.0],
            tex_layout_flags: [1, 0],
        },
    ]
}
