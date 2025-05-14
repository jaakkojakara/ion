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
var tex_post: texture_2d<f32>;
@group(2) @binding(1)
var tex_bloom: texture_2d<f32>;
@group(2) @binding(2)
var linear_sampler: sampler;

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let exposure = 1.0;
    let bloom = textureSampleLevel(tex_bloom, linear_sampler, in.tex_coords, 0.0);
    let color = textureSampleLevel(tex_post, linear_sampler, in.tex_coords, 0.0);

    var to_be_mapped: vec3<f32>;
    var do_mapping: bool;
    switch globals.frame_mode {
        case 0 : {
            to_be_mapped = mix(color, bloom, globals.post_bloom).xyz;
            do_mapping = true;
        }
        case 8 : {
            to_be_mapped = bloom.xyz;
            do_mapping = true;
        }
        default : {
            to_be_mapped = color.xyz;
            do_mapping = false;
        }
    }

    
    var tonemapped: vec3<f32>;
    if (do_mapping) {
        tonemapped = vec3<f32>(1.0, 1.0, 1.0) - exp(- to_be_mapped.xyz * exposure);
    } else {
        tonemapped = to_be_mapped.xyz;
    }

    return vec4<f32>(tonemapped, 1.0);
}



