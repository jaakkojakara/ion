use std::{
    ops::{Add, Mul},
    time::Duration,
};

use bincode::{Decode, Encode};
use derive_engine::RawData;
use ion_common::Map;
use renderer::gpu_data_types::{InstanceLight, InstanceSprite, LineVertex};
use textures::{TextureId, TextureLayout};

use crate::{
    core::{
        FrameId,
        coordinates::{ChunkLocation, Direction, Location},
    },
    gfx::renderer::render_camera::RenderCamera,
    util::casting::RawData,
};

pub mod gfx_config;
pub mod renderer;
pub mod textures;

// ---------------------------------------------------------- //
// ------------------------ Consts -------------------------- //
// ---------------------------------------------------------- //

/// Wheter the rendering system uses a wasm-compatible rendering architecture.
/// End result is identical to the native one (with some caveats), but the performance is likely to be worse.
/// Native targets also support wasm rendering for debugging purposes.
#[cfg(not(target_arch = "wasm32"))]
pub(crate) const WASM_COMPATIBLE_RENDERING: bool = false;

#[cfg(target_arch = "wasm32")]
pub(crate) const WASM_COMPATIBLE_RENDERING: bool = true;

// ---------------------------------------------------------- //
// ------------------- GFX Data Holders --------------------- //
// ---------------------------------------------------------- //

#[derive(Debug)]
pub struct GfxFrameData {
    pub global_data: GfxGlobalData,
    pub timing_data: GfxTimingData,
    pub sprite_data: GfxSpriteData,
    pub debug_data: GfxDebugData,
}

/// Global data for the current frame
#[derive(Debug)]
pub struct GfxGlobalData {
    pub frame: FrameId,
    pub camera_loc: Location,
    pub camera_scale: f32,

    pub lighting_ambient: f32,
    pub lighting_sun: f32,

    pub post_bloom: f32,
}

/// Data for timing measurements
/// This is used for frame pacing and syncing universe and render threads.
/// Completely handeled by the engine module.
#[derive(Debug)]
pub struct GfxTimingData {
    pub universe_frame_duration: Duration,
    pub render_frame_duration: Duration,
    pub render_frame_offset: f32,
    pub render_data_use_count: u32,
}

/// Contains all the objects that need to be rendered
///
/// Chunked sprites contain per-chunk sprites that are cached between frames.
/// Useful for static sprites that don't move, like ground, trees, buildings, etc.
/// If `None` is provided, the rendering system will use cached data for that chunk.
/// If the chunk is not cached, chunked sprites must be Some(...)
///
/// Dynamic sprites are sprites that move or change very frequently.
/// These are not cached are more expensive to render.
#[derive(Debug)]
pub struct GfxSpriteData {
    pub chunked_gfx: Map<ChunkLocation, Option<Vec<GfxRef>>>,
    pub dynamic_gfx: Vec<GfxRef>,
}

impl GfxSpriteData {
    pub fn chunks_to_vec(&self) -> Vec<ChunkLocation> {
        self.chunked_gfx.keys().map(|k| *k).collect()
    }
}

/// Data for debug rendering
/// Frame mode allows to rendering different intermediate render steps on screen.
/// Debug shapes are line primitives rendered on top of the final render.
/// Debug shapes are NOT rendered on wasm32 target.
#[derive(Debug)]
pub struct GfxDebugData {
    pub debug_shapes: Vec<Box<dyn DebugShape>>,
    pub debug_labels: Vec<(String, Location)>,
}

// ---------------------------------------------------------- //
// ------------------------- Color -------------------------- //
// ---------------------------------------------------------- //

#[repr(C)]
#[derive(Debug, Copy, Clone, Eq, PartialEq, RawData)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

impl Color {
    pub const WHITE: Color = Color { r: 255, g: 255, b: 255 };
    pub const GRAY: Color = Color { r: 100, g: 100, b: 100 };
    pub const BLACK: Color = Color { r: 0, g: 0, b: 0 };
    pub const RED: Color = Color { r: 255, g: 0, b: 0 };
    pub const GREEN: Color = Color { r: 0, g: 255, b: 0 };
    pub const BLUE: Color = Color { r: 0, g: 0, b: 255 };
    pub const YELLOW: Color = Color { r: 255, g: 255, b: 0 };
    pub const MAGENTA: Color = Color { r: 255, g: 0, b: 255 };
    pub const CYAN: Color = Color { r: 0, g: 255, b: 255 };
    pub const ORANGE: Color = Color { r: 255, g: 165, b: 0 };
    pub const PURPLE: Color = Color { r: 128, g: 0, b: 128 };
    pub const BROWN: Color = Color { r: 165, g: 42, b: 42 };
    pub const PINK: Color = Color { r: 255, g: 192, b: 203 };
    pub const LIGHT_GRAY: Color = Color { r: 211, g: 211, b: 211 };
    pub const DARK_GRAY: Color = Color { r: 64, g: 64, b: 64 };
    pub const LIME: Color = Color { r: 50, g: 205, b: 50 };
    pub const NAVY: Color = Color { r: 0, g: 0, b: 128 };
    pub const TEAL: Color = Color { r: 0, g: 128, b: 128 };
    pub const OLIVE: Color = Color { r: 128, g: 128, b: 0 };
    pub const MAROON: Color = Color { r: 128, g: 0, b: 0 };

    pub fn from_hex(hex_color: &str) -> Self {
        let stripped = hex_color.trim_start_matches('#');

        let r = u8::from_str_radix(&stripped[0..2], 16)
            .unwrap_or_else(|_| panic!("String {} is not a valid hex color", hex_color));
        let g = u8::from_str_radix(&stripped[2..4], 16)
            .unwrap_or_else(|_| panic!("String {} is not a valid hex color", hex_color));
        let b = u8::from_str_radix(&stripped[4..6], 16)
            .unwrap_or_else(|_| panic!("String {} is not a valid hex color", hex_color));

        Self { r, g, b }
    }

    pub(crate) fn to_f32_array(self) -> [f32; 3] {
        [self.r as f32 / 255.0, self.g as f32 / 255.0, self.b as f32 / 255.0]
    }
}

impl Add for Color {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        Self {
            r: self.r.saturating_add(rhs.r),
            g: self.g.saturating_add(rhs.g),
            b: self.b.saturating_add(rhs.b),
        }
    }
}

impl Mul<f32> for Color {
    type Output = Self;
    fn mul(self, rhs: f32) -> Self::Output {
        Self {
            r: ((self.r as f32) * rhs).clamp(0.0, 255.0) as u8,
            g: ((self.g as f32) * rhs).clamp(0.0, 255.0) as u8,
            b: ((self.b as f32) * rhs).clamp(0.0, 255.0) as u8,
        }
    }
}

// ---------------------------------------------------------- //
// --------------------- GFX Primitives --------------------- //
// ---------------------------------------------------------- //

/// A lightweight reference to a renderable gfx bundle
///
/// Contains the necessary information to render sprites, shadows, and lights that form a single graphical "object" like a tree, npc or a ground tile:
/// - `id`: Id of the sprite bundle. Received from `TextureAssets::include_sprite_bundle`
/// - `loc`: Location in the world
/// - `anim_start`: Optional animation start frame. If Some(..) the sprite will play the animation, as if it was started on the given frame. This allows randomizing or setting the animation start time exactly. All animations will loop, unless specifically stopped.
#[derive(Debug, Clone)]
pub struct GfxRef {
    pub id: u32,
    pub loc: Location,
    pub anim_start: Option<u32>,
}

impl GfxRef {
    pub fn new(id: u32, loc: Location) -> Self {
        Self {
            id,
            loc,
            anim_start: None,
        }
    }

    pub fn new_anim(id: u32, loc: Location, anim_start: u32) -> Self {
        Self {
            id,
            loc,
            anim_start: Some(anim_start),
        }
    }
}

/// A bundle of sprites, shadows and lights that form a single graphical "object" like a tree, npc or a ground tile.
/// These should be defined once and included in the [`TextureAssets`] struct before loading the assets.
/// When the game is running, the [`GfxRef`] structs are used to reference the bundles and render them.
pub struct GfxBundle {
    sprites: Vec<Sprite>,
    shadows: Vec<Sprite>,
    lights: Vec<GfxLight>,
}

impl GfxBundle {
    pub fn new(sprites: Vec<Sprite>) -> Self {
        Self {
            sprites,
            shadows: Vec::new(),
            lights: Vec::new(),
        }
    }

    pub fn new_with_shadows(sprites: Vec<Sprite>, shadows: Vec<Sprite>) -> Self {
        Self {
            sprites,
            shadows,
            lights: Vec::new(),
        }
    }

    pub fn new_with_lights(sprites: Vec<Sprite>, lights: Vec<GfxLight>) -> Self {
        Self {
            sprites,
            shadows: Vec::new(),
            lights,
        }
    }

    pub fn new_with_shadows_and_lights(sprites: Vec<Sprite>, shadows: Vec<Sprite>, lights: Vec<GfxLight>) -> Self {
        Self {
            sprites,
            shadows,
            lights,
        }
    }

    fn extract_for_render(
        &self,
        render_camera: Option<&RenderCamera>,
        gfx_ref: &GfxRef,
        sprites: &mut Vec<(InstanceSprite, TextureLayout, u8)>,
        shadows: &mut Vec<(InstanceSprite, TextureLayout)>,
        lights: &mut Vec<(InstanceLight, TextureLayout)>,
    ) {
        for sprite in &self.sprites {
            sprites.push((
                sprite.as_instance(render_camera, gfx_ref),
                sprite.texture_id.map(|id| id.layout).unwrap_or(TextureLayout::Square),
                sprite.layer,
            ));
        }

        for shadow in &self.shadows {
            shadows.push((
                shadow.as_instance(render_camera, gfx_ref),
                shadow.texture_id.map(|id| id.layout).unwrap_or(TextureLayout::Square),
            ));
        }

        for light in &self.lights {
            lights.push((
                light.as_instance_light(gfx_ref.loc.x, gfx_ref.loc.y),
                light
                    .sprite
                    .texture_id
                    .map(|id| id.layout)
                    .unwrap_or(TextureLayout::Square),
            ));
        }
    }
}

#[derive(Debug, Clone)]
pub struct GfxLight {
    light_source: Location,
    strength: f32,
    sprite: Sprite,
}

impl GfxLight {
    pub fn new(light_source: Location, strength: f32, sprite: Sprite) -> Self {
        Self {
            light_source,
            strength,
            sprite,
        }
    }

    pub(crate) fn as_instance_light(&self, x_offset: f32, y_offset: f32) -> InstanceLight {
        InstanceLight {
            loc_sprite: [self.sprite.loc.x + x_offset, self.sprite.loc.y + y_offset],
            loc_light: [self.light_source.x + x_offset, self.light_source.y + y_offset, 1.0],
            rot: self.sprite.rot,
            scale: self.sprite.scale,
            strength: self.strength,
            tex_coords: self.sprite.texture_id.unwrap().tex_coords,
            tex_coords_sizes: self.sprite.texture_id.unwrap().tex_coords_sizes,
            tex_sheet_index: self.sprite.texture_id.unwrap().tex_sheet_indices[0],
        }
    }
}

#[repr(u8)]
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum SpriteTypeId {
    Missing = 0,
    Normal = 10,
    Shadow = 20,
    Light = 30,
}

impl SpriteTypeId {
    fn as_normalized_f32(self) -> f32 {
        self as u8 as f32 / 255.0
    }
}

/// A single renderable sprite texture
#[derive(Debug, Clone)]
pub struct Sprite {
    type_id: SpriteTypeId,
    texture: String,
    texture_mask: Option<String>,
    texture_id: Option<TextureId>,
    texture_mask_id: Option<TextureId>,

    loc: Location,
    rot: f32,
    scale: f32,
    layer: u8,
    anim_fps: u32,
    camera_follow: bool,
}

impl Sprite {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        type_id: SpriteTypeId,
        texture: String,
        texture_mask: Option<String>,
        loc: Location,
        rot: f32,
        scale: f32,
        layer: u8, // Only affects color sprites, not shadows
        anim_fps: Option<u32>,
        camera_follow: bool,
    ) -> Self {
        Self {
            type_id,
            texture,
            texture_mask,
            texture_id: None,
            texture_mask_id: None,
            loc,
            rot,
            scale,
            layer,
            anim_fps: anim_fps.unwrap_or(0),
            camera_follow,
        }
    }

    fn as_instance(&self, render_camera: Option<&RenderCamera>, gfx_ref: &GfxRef) -> InstanceSprite {
        debug_assert!(self.texture_id.is_some(), "Sprite has no texture id");

        let anim_on: u32 = if gfx_ref.anim_start.is_some() { 1 } else { 0 };
        let anim_frame_rate: u32 = self.anim_fps;
        let anim_frame_start: u32 = gfx_ref.anim_start.unwrap_or(0);
        let anim_frame_count: u32 = self.texture_id.map(|id| id.frame_count as u32).unwrap_or(1);
        let anim_data: u32 = anim_on | (anim_frame_rate << 8) | (anim_frame_start << 16) | (anim_frame_count << 24);

        let offset_x = if self.camera_follow && render_camera.is_some() {
            render_camera.unwrap().interpolation_x()
        } else {
            0.0
        };

        let offset_y = if self.camera_follow && render_camera.is_some() {
            render_camera.unwrap().interpolation_y()
        } else {
            0.0
        };

        InstanceSprite {
            loc: [self.loc.x + gfx_ref.loc.x + offset_x, self.loc.y + gfx_ref.loc.y + offset_y],
            rot: self.rot,
            scale: self.scale,
            anim_data,
            tex_coords: self.texture_id.map(|id| id.tex_coords).unwrap_or([0.0, 0.0]),
            tex_coords_mask: self
                .texture_mask_id
                .map(|mask_id| mask_id.tex_coords)
                .unwrap_or([0.0, 0.0]),
            tex_coords_sizes: self.texture_id.map(|id| id.tex_coords_sizes).unwrap_or([1.0, 1.0]),
            tex_sheet_indices: self
                .texture_id
                .map(|id| id.tex_sheet_indices[0] | id.tex_sheet_indices[1] << 8)
                .unwrap_or(0)
                | self
                    .texture_mask_id
                    .map(|mask_id| mask_id.tex_sheet_indices[0])
                    .unwrap_or(255)
                    << 16
                | 0 << 24,
            type_id: self.type_id.as_normalized_f32(),
        }
    }
}

// ---------------------------------------------------------- //
// -------------------- Debug definitions ------------------- //
// ---------------------------------------------------------- //

#[repr(u32)]
#[derive(Debug, Clone, Copy, Eq, PartialEq, Encode, Decode)]
pub enum GfxFrameMode {
    Normal = 0,
    RawColorPass = 1,
    RawNormalPass = 2,
    RawHeightIdPass = 3,
    RawLightPass = 4,
    RawShadowPass = 5,
    SsaoPass = 6,
    LightPass = 7,
    BlurPass = 8,
    OverDraw = 9,
    DepthTexture = 10,
}

impl Default for GfxFrameMode {
    fn default() -> Self {
        Self::Normal
    }
}

pub trait DebugShape: std::fmt::Debug + Send + Sync {
    fn as_line_vertices(&self) -> Vec<LineVertex>;
}

#[derive(Debug)]
pub struct DebugLine {
    pub start: Location,
    pub end: Location,
    pub color: Color,
}

impl DebugShape for DebugLine {
    fn as_line_vertices(&self) -> Vec<LineVertex> {
        vec![
            LineVertex {
                loc: [self.start.x, self.start.y, 0.0],
                color: self.color.to_f32_array(),
            },
            LineVertex {
                loc: [self.end.x, self.end.y, 0.0],
                color: self.color.to_f32_array(),
            },
        ]
    }
}

#[derive(Debug)]
pub struct DebugCross {
    pub loc: Location,
    pub color: Color,
}

impl DebugShape for DebugCross {
    fn as_line_vertices(&self) -> Vec<LineVertex> {
        vec![
            LineVertex {
                loc: [self.loc.x - 0.4, self.loc.y, 0.0],
                color: self.color.to_f32_array(),
            },
            LineVertex {
                loc: [self.loc.x + 0.4, self.loc.y, 0.0],
                color: self.color.to_f32_array(),
            },
            LineVertex {
                loc: [self.loc.x, self.loc.y - 0.4, 0.0],
                color: self.color.to_f32_array(),
            },
            LineVertex {
                loc: [self.loc.x, self.loc.y + 0.4, 0.0],
                color: self.color.to_f32_array(),
            },
        ]
    }
}

#[derive(Debug)]
pub struct DebugArrow {
    pub loc: Location,
    pub dir: Direction,
    pub size: f32,
    pub color: Color,
}

impl DebugShape for DebugArrow {
    fn as_line_vertices(&self) -> Vec<LineVertex> {
        let end_point = self.loc.towards_dir(self.dir, self.size);

        // Create main stem line
        let mut vertices = vec![
            LineVertex {
                loc: [self.loc.x, self.loc.y, 0.0],
                color: self.color.to_f32_array(),
            },
            LineVertex {
                loc: [end_point.x, end_point.y, 0.0],
                color: self.color.to_f32_array(),
            },
        ];

        // Create arrowhead lines (30% of arrow size)
        let arrowhead_size = self.size * 0.3;
        let left_arrowhead = end_point.towards_dir(self.dir.rotate_deg(135), arrowhead_size);
        let right_arrowhead = end_point.towards_dir(self.dir.rotate_deg(-135), arrowhead_size);

        // Add left arrowhead line
        vertices.extend([
            LineVertex {
                loc: [end_point.x, end_point.y, 0.0],
                color: self.color.to_f32_array(),
            },
            LineVertex {
                loc: [left_arrowhead.x, left_arrowhead.y, 0.0],
                color: self.color.to_f32_array(),
            },
        ]);

        // Add right arrowhead line
        vertices.extend([
            LineVertex {
                loc: [end_point.x, end_point.y, 0.0],
                color: self.color.to_f32_array(),
            },
            LineVertex {
                loc: [right_arrowhead.x, right_arrowhead.y, 0.0],
                color: self.color.to_f32_array(),
            },
        ]);

        vertices
    }
}
