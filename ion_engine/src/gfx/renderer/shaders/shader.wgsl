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
    @builtin(position) clip_position: vec4<f32>,
    @location(0) world_location: vec2<f32>,
    @location(1) tex_coords: vec2<f32>,
    @location(2) camera_direction: vec3<f32>,
}

@group(0) @binding(0)
var<uniform> globals: GlobalsUniform;
@group(1) @binding(0)
var<uniform> camera: CameraUniform;

const sqrt_2: f32 = 1.4142135623730951;

@vertex
fn vs_main(@builtin(vertex_index) vertex_index: u32,) -> VertexOutput {
    var out: VertexOutput;

    switch (i32(vertex_index)) {
        case 0 : {
            out.clip_position = vec4<f32>(1.0, 1.0, 1.0, 1.0);
            out.tex_coords = vec2<f32>(1.0, 0.0);
        }
        case 1 : {
            out.clip_position = vec4<f32>(-1.0, 1.0, 1.0, 1.0);
            out.tex_coords = vec2<f32>(0.0, 0.0);
        }
        case 2 : {
            out.clip_position = vec4<f32>(-1.0, -1.0, 1.0, 1.0);
            out.tex_coords = vec2<f32>(0.0, 1.0);
        }
        case 3 : {
            out.clip_position = vec4<f32>(1.0, 1.0, 1.0, 1.0);
            out.tex_coords = vec2<f32>(1.0, 0.0);
        }
        case 4 : {
            out.clip_position = vec4<f32>(-1.0, -1.0, 1.0, 1.0);
            out.tex_coords = vec2<f32>(0.0, 1.0);
        }
        case 5 : {
            out.clip_position = vec4<f32>(1.0, -1.0, 1.0, 1.0);
            out.tex_coords = vec2<f32>(1.0, 1.0);
        }
        default : {
            out.clip_position = vec4<f32>(0.0, 0.0, 1.0, 1.0);
            out.tex_coords = vec2<f32>(0.0, 0.0);
        }
    }

    let clip_vec = vec4<f32>(out.clip_position.x, out.clip_position.y, 0.0, 1.0);
    out.world_location = (camera.vp_mat_inv * clip_vec).xy;
    out.camera_direction = normalize(vec3<f32>(camera.angle_sin / sqrt_2, camera.angle_sin / sqrt_2, camera.angle_cos * -1.0));
    
    return out;
}


// Water shader constants
const DRAG_MULT: f32 = 0.38;
const WAVE_HEIGHT: f32 = 1.2;  // Controls wave amplitude
const WATER_DEPTH: f32 = 2.0;  // Maximum depth of water
const ITERATIONS_NORMAL: i32 = 16;
const NORMAL_SAMPLE_DISTANCE: f32 = 0.05;

// Water color constants
const DEEP_WATER_COLOR: vec3<f32> = vec3<f32>(0.03, 0.06, 0.08);  // Current dark ocean blue
const SHALLOW_WATER_COLOR: vec3<f32> = vec3<f32>(0.09, 0.2, 0.25);  // Slightly lighter, more turquoise
const SHALLOW_DEPTH: f32 = 1.8;  // Depth at which we consider water "shallow"

// Wave generation functions
fn wavedx(location: vec2<f32>, direction: vec2<f32>, frequency: f32, timeshift: f32) -> vec2<f32> {
    let x = dot(direction, location) * frequency + timeshift;
    let wave = exp(sin(x) - 1.0);
    let dx = wave * cos(x);
    return vec2<f32>(wave, -dx);
}

// Calculates wave normal and height
fn get_wave_data(location: vec2<f32>, iterations: i32) -> vec4<f32> {
    let wave_phase_shift = length(location) * 0.1;
    var iter = 0.0;
    var frequency = 1.0;
    var time_multiplier = 1.0;
    var weight = 1.0;
    
    var center_pos = location;
    var right_pos = location + vec2<f32>(NORMAL_SAMPLE_DISTANCE, 0.0);
    var up_pos = location + vec2<f32>(0.0, NORMAL_SAMPLE_DISTANCE);
    
    var center_height = 0.0;
    var right_height = 0.0;
    var up_height = 0.0;
    var total_weight = 0.0;

    for(var i = 0; i < iterations; i++) {
        let p = vec2<f32>(sin(iter), cos(iter));
        let time = f32(globals.frame) * 0.016667 * time_multiplier + wave_phase_shift;
        
        // Calculate heights at all three points
        let res_center = wavedx(center_pos, p, frequency, time);
        let res_right = wavedx(right_pos, p, frequency, time);
        let res_up = wavedx(up_pos, p, frequency, time);
        
        // Accumulate heights
        center_height += res_center.x * weight;
        right_height += res_right.x * weight;
        up_height += res_up.x * weight;
        total_weight += weight;
        
        // Update locations with drag effect
        center_pos += p * res_center.y * weight * DRAG_MULT;
        right_pos += p * res_right.y * weight * DRAG_MULT;
        up_pos += p * res_up.y * weight * DRAG_MULT;

        weight = mix(weight, 0.0, 0.2);
        frequency *= 1.18;
        time_multiplier *= 1.07;
        iter += 1232.399963;
    }

    // Normalize heights
    center_height /= total_weight;
    right_height /= total_weight;
    up_height /= total_weight;

    // Scale heights to wave height range (-WAVE_HEIGHT to 0)
    center_height = center_height * WAVE_HEIGHT - WAVE_HEIGHT;
    right_height = right_height * WAVE_HEIGHT - WAVE_HEIGHT;
    up_height = up_height * WAVE_HEIGHT - WAVE_HEIGHT;

    // Calculate normal using height differences
    let center = vec3<f32>(location.x, location.y, center_height * 2.0);
    let right = vec3<f32>(location.x + NORMAL_SAMPLE_DISTANCE, location.y, right_height * 2.0);
    let up = vec3<f32>(location.x, location.y + NORMAL_SAMPLE_DISTANCE, up_height * 2.0);

    // Calculate normal using cross product - fix orientation by swapping cross product order
    let tangent_x = right - center;
    let tangent_y = up - center;
    let normal = normalize(cross(tangent_x, tangent_y));  // Swapped from cross(tangent_y, tangent_x)

    return vec4<f32>(normal.xyz, center_height);
}

fn get_sky_color(direction: vec3<f32>) -> vec3<f32> {
    let sun_direction = normalize(vec3<f32>(0.3, 0.3, 0.8)); // TODO: Match the sun in assets
    let suncolor = vec3<f32>(0.98, 0.95, 0.91);

    let zenith_factor = 1.0 / (direction.z + 0.1);
    let sun_effect = pow(abs(dot(sun_direction, direction)), 2.0);

    let sky_color_1 = vec3<f32>(5.5, 13.0, 22.4) / 22.4 * suncolor;
    let sky_color_2 = max(vec3<f32>(0.0), sky_color_1 - vec3<f32>(5.5, 13.0, 22.4) * 0.002 * (zenith_factor + -6.0 * sun_direction.z * sun_direction.z));
    let final_sky = sky_color_2 * zenith_factor * (0.24 + sun_effect * 0.24);
    let atmosphere = final_sky * (1.0 + 1.0 * pow(1.0 - direction.z, 3.0));
    
    // Add sun disk
    let sun_intensity = 50.0;
    let sun_radius = 0.003;
    let sun_dot = dot(direction, sun_direction);
    let sun = smoothstep(1.0 - sun_radius, 1.0, sun_dot) * sun_intensity;
    
    return atmosphere + suncolor * sun;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let screen_y_coord = in.clip_position.y / f32(globals.frame_res_y);
    let camera_height = camera.z_edges.x + (camera.z_edges.y - camera.z_edges.x) * (1.0 - screen_y_coord);
    let camera_x_y_offset = camera_height * camera.angle_tan / 1.414;
    let ray_dir = in.camera_direction;

    //let height_id = textureSample(t_height_id, linear_sampler, in.tex_coords);
    let ground_height = -1.0; //(height_id.x - globals.height_scaled_zero) * globals.height_units_total;

    // Get wave height and normal in one pass
    let wave_data = get_wave_data(in.world_location.xy, ITERATIONS_NORMAL);

    if (wave_data.w < ground_height) {
        return vec4<f32>(0.0);
    }

    // Calculate water depth
    let water_depth = wave_data.w - ground_height;
    
    // Calculate depth-based color blend
    let depth_blend = smoothstep(0.0, SHALLOW_DEPTH, water_depth);
    let base_water_color = mix(SHALLOW_WATER_COLOR, DEEP_WATER_COLOR, depth_blend);

    // Calculate normal
    let smoothed_N = mix(wave_data.xyz, vec3<f32>(0.0, 0.0, 1.0), 0.8 * min(1.0, sqrt(camera_height*0.01) * 1.1));

    // Calculate fresnel
    let fresnel = 0.02 + (1.0 - 0.02) * pow(1.0 - max(0.0, dot(-smoothed_N, ray_dir)), 3.0);

    // Calculate reflection
    let reflect_dir = reflect(ray_dir, smoothed_N);
    let reflection = get_sky_color(vec3<f32>(reflect_dir.x, reflect_dir.y, abs(reflect_dir.z)));
    
    // Calculate scattering with depth-aware factor
    let depth_factor = mix(0.5, 1.0, depth_blend);  // Less scattering in shallow water
    let scattering = base_water_color * depth_factor;

    // Combine colors with depth-aware fresnel
    let shallow_fresnel = mix(fresnel * 0.7, fresnel, depth_blend);  // Less reflective in shallow water
    let final_color = mix(scattering, reflection, shallow_fresnel);

    // Calculate opacity based on depth and viewing angle
    let depth_opacity = mix(0.6, 0.85, depth_blend);  // More transparent in shallow water
    let view_opacity = 1.0 - shallow_fresnel * 0.5;  // More transparent at glancing angles
    let final_opacity = depth_opacity * view_opacity * 0.8;

    return vec4<f32>(final_color, 0.1);
}

