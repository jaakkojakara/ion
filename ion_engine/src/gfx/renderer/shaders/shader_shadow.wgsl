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
    @location(1) scale: f32,
    @location(2) tex_coords: vec2<f32>,
    @location(3) tex_coords_mask: vec2<f32>,
    @location(4) tex_sheet_indices: u32,
    @location(5) type_id: f32,
}

struct InstanceInput {
    @location(6) location: vec2<f32>,
    @location(7) rotation: f32,
    @location(8) scale: f32,
    @location(9) anim_data: u32,
    @location(10) tex_coords: vec2<f32>,
    @location(11) tex_coords_mask: vec2<f32>,
    @location(12) tex_coords_sizes: vec2<f32>,
    @location(13) tex_sheet_indices: u32,
    @location(14) type_id: f32,
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
var tex_height: texture_2d<f32>;
@group(3) @binding(1)
var tex_height_sampler: sampler;


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
    let anim_data = unpack4u8u32(instance.anim_data);
    let anim_on = anim_data.x;
    let anim_frame_rate = anim_data.y;
    let anim_frame_start = anim_data.z;
    let anim_frame_count = anim_data.w;

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
    position.x = vertex.position.x * scale_factor + position_vertical_fix + instance.location.x;
    position.y = vertex.position.y * scale_factor + position_vertical_fix + instance.location.y;

    // ---------------------------------------------------------- //
    // ------------------ Texture coordinates ------------------- //
    // ---------------------------------------------------------- //

    
    var anim_cur_frame = anim_on * ((globals.frame / anim_frame_rate + anim_frame_start) % anim_frame_count);
    let anim_frame_size_x = instance.tex_coords_sizes.x + 1.0 / globals.tex_sheet_size;
    let anim_frame_size_y = instance.tex_coords_sizes.y + 1.0 / globals.tex_sheet_size;

    let frames_in_first_row = floor((1.0 - instance.tex_coords.x) / anim_frame_size_x);
    let frames_in_subsequent_rows = floor(1.0 / anim_frame_size_x);
    
    // Calculate row and column in one unified calculation
    let is_on_first_row = f32(anim_cur_frame) < frames_in_first_row;
    let frame_row = select(
        floor((f32(anim_cur_frame) - frames_in_first_row) / frames_in_subsequent_rows) + 1.0,
        0.0,
        is_on_first_row
    );
    let frame_col = select(
        f32(anim_cur_frame) - frames_in_first_row - (frame_row - 1.0) * frames_in_subsequent_rows,
        f32(anim_cur_frame),
        is_on_first_row
    );

    let tex_coords_offset_x = instance.tex_coords_sizes.x * vertex.tex_coord_offset.x + frame_col * anim_frame_size_x;
    let tex_coords_offset_y = instance.tex_coords_sizes.y * vertex.tex_coord_offset.y + frame_row * anim_frame_size_y + text_coords_vertical_fix;
    
    let base_x = select(0.0, instance.tex_coords.x, frame_row == 0.0);
    let base_mask_x = select(0.0, instance.tex_coords_mask.x, frame_row == 0.0);
    
    let tex_coords = vec2<f32>(base_x + tex_coords_offset_x, instance.tex_coords.y + tex_coords_offset_y);
    let tex_coords_mask = vec2<f32>(base_mask_x + tex_coords_offset_x, instance.tex_coords_mask.y + tex_coords_offset_y);

    // ---------------------------------------------------------- //
    // -------------- Output for fragment shader ---------------- //
    // ---------------------------------------------------------- //

    var out: VertexOutput;

    out.clip_pos = camera.vp_mat * vec4<f32>(position, 1.0);
    out.world_loc = vec2<f32>(position.x, position.y);
    out.scale = instance.scale;
    out.tex_coords = tex_coords;
    out.tex_coords_mask = tex_coords_mask;
    out.tex_sheet_indices = instance.tex_sheet_indices;
    out.type_id = instance.type_id;
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let screen_x_coord = in.clip_pos.x / f32(globals.frame_res_x);
    let screen_y_coord = in.clip_pos.y / f32(globals.frame_res_y);
    let tex_sheet_indices = unpack4u8u32(in.tex_sheet_indices);

    let shadow_color = textureSample(tex[tex_sheet_indices.x], tex_sampler, in.tex_coords);
    let height_id = textureSample(tex_height, tex_height_sampler, vec2<f32>(screen_x_coord, screen_y_coord));
    let height = height_id.x;

    let height_distance = abs(height - globals.height_scaled_zero);
    let max_distance = 0.02; // 2% away from ground level
    let height_alpha = 1.0 - smoothstep(0.0, max_distance, height_distance);
    let sun_alpha = max(min(globals.lighting_sun / 2.0, 1.0), 0.5);

    return vec4<f32>(0.0, 0.0, 0.0, shadow_color.a * height_alpha * sun_alpha);
}