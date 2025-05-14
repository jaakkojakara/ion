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

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) tex_coords: vec2<f32>,
}

@vertex
fn vs_main(@builtin(vertex_index) vertex_index: u32,) -> VertexOutput {
    var out: VertexOutput;

    switch (i32(vertex_index)) {
        case 0 : {
            out.position = vec4<f32>(1.0, 1.0, 1.0, 1.0);
            out.tex_coords = vec2<f32>(1.0, 0.0);
        }
        case 1 : {
            out.position = vec4<f32>(- 1.0, 1.0, 1.0, 1.0);
            out.tex_coords = vec2<f32>(0.0, 0.0);
        }
        case 2 : {
            out.position = vec4<f32>(- 1.0, - 1.0, 1.0, 1.0);
            out.tex_coords = vec2<f32>(0.0, 1.0);
        }
        case 3 : {
            out.position = vec4<f32>(1.0, 1.0, 1.0, 1.0);
            out.tex_coords = vec2<f32>(1.0, 0.0);
        }
        case 4 : {
            out.position = vec4<f32>(- 1.0, - 1.0, 1.0, 1.0);
            out.tex_coords = vec2<f32>(0.0, 1.0);
        }
        case 5 : {
            out.position = vec4<f32>(1.0, - 1.0, 1.0, 1.0);
            out.tex_coords = vec2<f32>(1.0, 1.0);
        }
        default : { }
    }

    return out;
}

@group(0) @binding(0)
var<uniform> globals: GlobalsUniform;
@group(1) @binding(0)
var<uniform> camera: CameraUniform;

// Fragment shader
@group(2) @binding(0)
var t_bloom: texture_2d<f32>;
@group(2) @binding(1)
var linear_sampler: sampler;

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {

    let filter_radius = 0.0065;
    let x = filter_radius;
    let y = filter_radius;

    let a = textureSample(t_bloom, linear_sampler, vec2<f32>(in.tex_coords.x - x, in.tex_coords.y + y));
    let b = textureSample(t_bloom, linear_sampler, vec2<f32>(in.tex_coords.x, in.tex_coords.y + y));
    let c = textureSample(t_bloom, linear_sampler, vec2<f32>(in.tex_coords.x + x, in.tex_coords.y + y));
    let d = textureSample(t_bloom, linear_sampler, vec2<f32>(in.tex_coords.x - x, in.tex_coords.y));
    let e = textureSample(t_bloom, linear_sampler, vec2<f32>(in.tex_coords.x, in.tex_coords.y));
    let f = textureSample(t_bloom, linear_sampler, vec2<f32>(in.tex_coords.x + x, in.tex_coords.y));
    let g = textureSample(t_bloom, linear_sampler, vec2<f32>(in.tex_coords.x - x, in.tex_coords.y - y));
    let h = textureSample(t_bloom, linear_sampler, vec2<f32>(in.tex_coords.x, in.tex_coords.y - y));
    let i = textureSample(t_bloom, linear_sampler, vec2<f32>(in.tex_coords.x + x, in.tex_coords.y - y));

    var upsample = e * 4.0;
    upsample += (b + d + f + h) * 2.0;
    upsample += (a + c + g + i);
    upsample *= 1.0 / 16.0;

    return upsample;
}




