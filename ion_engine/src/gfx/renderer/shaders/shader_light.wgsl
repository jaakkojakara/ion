struct GlobalsUniform {
    frame: u32,
    frame_mode: u32,
    frame_res_x: u32,
    frame_res_y: u32,
    window_res_x: u32,
    window_res_y: u32,

    tex_sheet_size: f32,
    pixels_per_unit: f32,
    height_units_total: f32,
    height_scaled_zero: f32,

    lighting_ambient: f32,
    lighting_sun: f32,
    lighting_unused: f32,

    post_bloom: f32,

    _padding1: f32,
    _padding2: f32,
}

struct CameraUniform {
    vp_mat: mat4x4<f32>,
    vp_mat_inv: mat4x4<f32>,
    z_edges: vec2<f32>,
    loc: vec2<f32>,
    scale: f32,
    angle_cos: f32,
    angle_sin: f32,
    angle_tan: f32,
}

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) tex_coord_offset: vec2<f32>,
    @location(2) tex_layout_flags: vec2<u32>,
}

struct VertexOutput {
    @builtin(position) clip_pos: vec4<f32>,
    @location(0) world_loc: vec2<f32>,
    @location(1) light_loc: vec3<f32>,
    @location(2) light_strength: f32,
    @location(3) tex_coords: vec2<f32>,
    @location(4) tex_sheet_index: u32,
}

struct InstanceInput {
    @location(6) loc_sprite: vec2<f32>,
    @location(7) loc_light: vec3<f32>,
    @location(8) rot: f32,
    @location(9) scale: f32,
    @location(10) strength: f32,
    @location(11) tex_coords: vec2<f32>,
    @location(12) tex_coords_sizes: vec2<f32>,
    @location(13) tex_sheet_index: u32,
};

@group(0) @binding(0)
var<uniform> globals: GlobalsUniform;
@group(1) @binding(0)
var<uniform> camera: CameraUniform;

@group(2) @binding(0)
var tex: binding_array<texture_2d<f32>>;
@group(2) @binding(1)
var tex_sampler: sampler;

@group(3) @binding(0)
var tex_normal: texture_2d<f32>;
@group(3) @binding(1)
var tex_height: texture_2d<f32>;
@group(3) @binding(2)
var tex_source_sampler: sampler;


fn unpack4u8u32(packed: u32) -> vec4<u32> {
    let a: u32 = packed & 0xFFu;
    let b: u32 = (packed >> 8u) & 0xFFu;
    let c: u32 = (packed >> 16u) & 0xFFu;
    let d: u32 = (packed >> 24u) & 0xFFu;
    return vec4<u32>(a, b, c, d);
}

@vertex
fn vs_main(
    vertex: VertexInput,
    instance: InstanceInput,
    @builtin(vertex_index) vertex_index: u32,
) -> VertexOutput {
    let scale_factor = instance.scale * (instance.tex_coords_sizes.x / (globals.pixels_per_unit / globals.tex_sheet_size));

    // ---------------------------------------------------------- //
    // ------------ Vertex and tex coord offsets ---------------- //
    // ---------------------------------------------------------- //
    
    var position_vertical_fix: f32;
    var text_coords_vertical_fix: f32 = 0.0;

    if vertex.tex_layout_flags.x == u32(1) && vertex.tex_layout_flags.y == u32(1) {
        // Ortographic layouts, top 2 vertices
        position_vertical_fix = (instance.tex_coords_sizes.y / instance.tex_coords_sizes.x - 1.0) * scale_factor;
    } else if vertex.tex_layout_flags.x == u32(3) {
        // IsometricHex layouts, Top 3 vertices
        position_vertical_fix = (instance.tex_coords_sizes.y / camera.angle_cos / instance.tex_coords_sizes.x - 1.0) * scale_factor;
        if vertex_index == u32(9) || vertex_index == u32(10) {
            text_coords_vertical_fix = camera.angle_cos / 2.0 * instance.tex_coords_sizes.x;
        }
    } else if vertex.tex_layout_flags.x == u32(4) {
        // IsometricHex layouts, Bottom 3 vertices
        position_vertical_fix = 0.0;
        if vertex_index == u32(11) || vertex_index == u32(13) {
            text_coords_vertical_fix = instance.tex_coords_sizes.y - camera.angle_cos / 2.0 * instance.tex_coords_sizes.x;
        }
    } else {
        position_vertical_fix = 0.0;
    }

    // ---------------------------------------------------------- //
    // -------------------- Vertex position --------------------- //
    // ---------------------------------------------------------- //

    var position = vertex.position;
    position.x = vertex.position.x * scale_factor + position_vertical_fix + instance.loc_sprite.x;
    position.y = vertex.position.y * scale_factor + position_vertical_fix + instance.loc_sprite.y;

    // ---------------------------------------------------------- //
    // ------------------ Texture coordinates ------------------- //
    // ---------------------------------------------------------- //

    
    var tex_coords: vec2<f32>;
    tex_coords.x = instance.tex_coords.x + instance.tex_coords_sizes.x * vertex.tex_coord_offset.x;
    tex_coords.y = instance.tex_coords.y + instance.tex_coords_sizes.y * vertex.tex_coord_offset.y + text_coords_vertical_fix;


    // ---------------------------------------------------------- //
    // -------------- Output for fragment shader ---------------- //
    // ---------------------------------------------------------- //

    var out: VertexOutput;

    out.clip_pos = camera.vp_mat * vec4<f32>(position, 1.0);
    out.world_loc = vec2<f32>(position.x, position.y);
    out.light_loc = instance.loc_light;
    out.light_strength = instance.strength;
    out.tex_coords = tex_coords;
    out.tex_sheet_index = instance.tex_sheet_index;
    return out;
}

const TYPE_NORMAL: f32 = 10.0 / 255.0;
const TYPE_SHADOW: f32 = 20.0 / 255.0;
const TYPE_LIGHT: f32 = 30.0 / 255.0;
const TYPE_OCEAN: f32 = 40.0 / 255.0;

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let screen_x_coord = in.clip_pos.x / f32(globals.frame_res_x);
    let screen_y_coord = in.clip_pos.y / f32(globals.frame_res_y);

    let light_color = textureSample(tex[in.tex_sheet_index], tex_sampler, in.tex_coords);
    let normal = textureSample(tex_normal, tex_source_sampler, vec2<f32>(screen_x_coord, screen_y_coord));
    let height_id = textureSample(tex_height, tex_source_sampler, vec2<f32>(screen_x_coord, screen_y_coord));
    let frag_height = (height_id.x - globals.height_scaled_zero) * globals.height_units_total;
    let frag_x_y_offset = frag_height * camera.angle_tan / 1.414;
    let frag_location = vec3<f32>(in.world_loc.x - frag_x_y_offset, in.world_loc.y - frag_x_y_offset, frag_height);

    let light_dir = normalize(in.light_loc.xyz - frag_location);
    let surface_dir = vec3<f32>(normal.x * -1.0, normal.y * -1.0, normal.z);

    let diffuse_strength = max(dot(surface_dir, light_dir), 0.0);
    var final_strength: f32;
    if abs(height_id.z - TYPE_LIGHT) < 0.01 {
        final_strength = in.light_strength;
    } else {
        final_strength = diffuse_strength * in.light_strength * light_color.a;
    }

    return vec4<f32>(light_color.xyz * final_strength, 1.0);
}