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
var tex_height: texture_2d<f32>;
@group(2) @binding(1)
var tex_source_sampler: sampler;

@group(3) @binding(0)
var ssao_noise_texture: texture_2d<f32>;
@group(3) @binding(1)
var ssao_noise_sampler: sampler;

fn ssao_calc(sample_coords: vec2<f32>, ref_height: f32, ssao_radius: f32) -> f32 {
    let height_id = textureSample(tex_height, tex_source_sampler, sample_coords);
    let frag_height = (height_id.x - globals.height_scaled_zero) * globals.height_units_total;
    let radius_height = ssao_radius * 2.0;
    var ssao_contribution = max(min(frag_height - ref_height, radius_height), 0.0) * (1.0 - smoothstep(0.0, radius_height, frag_height));
    return ssao_contribution;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let ssao_resolution_fraction = 1.0;

    let height_id = textureSample(tex_height, tex_source_sampler, in.tex_coords);
    let frag_height = (height_id.x - globals.height_scaled_zero) * globals.height_units_total;

    let ssao_strength = 10.0;
    let ssao_radius = 0.15;
    let ssao_radius_scaled = ssao_radius / camera.scale;
    let ssao_noise = textureSample(ssao_noise_texture, ssao_noise_sampler, vec2<f32>(in.tex_coords.x, in.tex_coords.y) * vec2<f32>(f32(globals.frame_res_x) / (4.0 * ssao_resolution_fraction), f32(globals.frame_res_y) / (4.0 * ssao_resolution_fraction)));
    let noise_mat = mat2x2<f32>(vec2<f32>(ssao_noise.x, -ssao_noise.y), vec2<f32>(ssao_noise.y, ssao_noise.x));

    var occlusion = 0.0;
    var coords = in.tex_coords - noise_mat * vec2<f32>(0.2948544, 0.69643044) * ssao_radius_scaled;
    occlusion += ssao_calc(coords, frag_height, ssao_radius);
    coords = in.tex_coords - noise_mat * vec2<f32>(0.26781642, - 0.982173) * ssao_radius_scaled;
    occlusion += ssao_calc(coords, frag_height, ssao_radius);
    coords = in.tex_coords - noise_mat * vec2<f32>(0.5758351, - 0.5613427) * ssao_radius_scaled;
    occlusion += ssao_calc(coords, frag_height, ssao_radius);
    coords = in.tex_coords - noise_mat * vec2<f32>(0.1466015, 0.30463803) * ssao_radius_scaled;
    occlusion += ssao_calc(coords, frag_height, ssao_radius);
    coords = in.tex_coords - noise_mat * vec2<f32>(0.86359566, 0.7253686) * ssao_radius_scaled;
    occlusion += ssao_calc(coords, frag_height, ssao_radius);
    coords = in.tex_coords - noise_mat * vec2<f32>(- 0.115032256, 0.3288535) * ssao_radius_scaled;
    occlusion += ssao_calc(coords, frag_height, ssao_radius);
    coords = in.tex_coords - noise_mat * vec2<f32>(- 0.2590866, 0.23883444) * ssao_radius_scaled;
    occlusion += ssao_calc(coords, frag_height, ssao_radius);
    coords = in.tex_coords - noise_mat * vec2<f32>(- 0.82451904, - 0.07327837) * ssao_radius_scaled;
    occlusion += ssao_calc(coords, frag_height, ssao_radius);
    coords = in.tex_coords - noise_mat * vec2<f32>(- 0.13348156, - 0.35933006) * ssao_radius_scaled;
    occlusion += ssao_calc(coords, frag_height, ssao_radius);

    occlusion = occlusion * (1.0 - smoothstep(0.1, 0.5, frag_height));

    let ssao = 1.0 - occlusion * ssao_strength / 8.0;

    return vec4<f32>(ssao, ssao, ssao, 1.0);
}




