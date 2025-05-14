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
var tex_color: texture_2d<f32>;
@group(2) @binding(1)
var tex_normal: texture_2d<f32>;
@group(2) @binding(2)
var tex_height_id: texture_2d<f32>;
@group(2) @binding(3)
var tex_lights_shadows: texture_2d<f32>;
@group(2) @binding(4)
var tex_ssao: texture_2d<f32>;
@group(2) @binding(5)
var tex_source_sampler: sampler;
@group(2) @binding(6)
var tex_depth: texture_depth_2d;
@group(2) @binding(7)
var depth_sampler: sampler;

const TYPE_NORMAL: f32 = 10.0 / 255.0;
const TYPE_SHADOW: f32 = 20.0 / 255.0;
const TYPE_LIGHT: f32 = 30.0 / 255.0;
const TYPE_OCEAN: f32 = 40.0 / 255.0;

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let color = textureSample(tex_color, tex_source_sampler, in.tex_coords).xyz;
    let normal = textureSample(tex_normal, tex_source_sampler, in.tex_coords).xyz;
    let height_id = textureSample(tex_height_id, tex_source_sampler, in.tex_coords);
    let lights_shadows = textureSample(tex_lights_shadows, tex_source_sampler, in.tex_coords);
    let ssao_value = textureSample(tex_ssao, tex_source_sampler, in.tex_coords).x;

    let light = lights_shadows.xyz;
    let shadow = lights_shadows.a;
    let ssao = 1.0 - (1.0 - ssao_value) * 3.0;

    let frag_height = (height_id.x - globals.height_scaled_zero) * globals.height_units_total;

    // Sun lighting
    let surface_dir = vec3<f32>(normal.x * - 1.0, normal.y * - 1.0, normal.z);
    let sun_dir = normalize(vec3<f32>(- 5.0, 4.0, 15.0));
    let sun_color = vec3<f32>(1.0, 1.0, 1.0);
    let sun_light = max(dot(surface_dir, sun_dir), 0.0) * globals.lighting_sun * sun_color;
    let ambient_light = vec3<f32>(globals.lighting_ambient, globals.lighting_ambient, globals.lighting_ambient);

    // Final lighting composite
    let final_lighting = (light + sun_light + ambient_light) * ssao;

    let mix_shadow = mix(color.xyz, vec3<f32>(0.0, 0.0, 0.0), shadow);
    var mix_light = mix_shadow.xyz * final_lighting;

    // If this pixel is light source, use raw light
    if abs(height_id.x - TYPE_LIGHT) < 0.01 {
        mix_light = color.xyz * max(light.x, 1.0);
    }

    switch globals.frame_mode {
        case 1 : {
            return vec4<f32>(color.xyz, 1.0);
        }
        case 2 : {
            return vec4<f32>(normal.xyz, 1.0);
        }
        case 3 : {
            return vec4<f32>(height_id.rgb, 1.0);
        }
        case 4 : {
            let mapped_light = vec3<f32>(1.0, 1.0, 1.0) - exp(- light);
            return vec4<f32>(mapped_light, 1.0);
        }
        case 5 : {
            return vec4<f32>(shadow, shadow, shadow, 1.0);
        }
        case 6 : {
            return vec4<f32>(ssao, ssao, ssao, 1.0);
        }
        case 7 : {
            let mapped_light = vec3<f32>(1.0, 1.0, 1.0) - exp(- final_lighting);
            return vec4<f32>(mapped_light, 1.0);
        }
        case 9 : {
            return vec4<f32>(color.xyz, 1.0);
        }
        default : {
            return vec4<f32>(mix_light, 1.0);
        }
    }
}




