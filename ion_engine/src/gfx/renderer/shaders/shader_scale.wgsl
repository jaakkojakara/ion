struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) tex_coords: vec2<f32>,
}


@vertex
fn vs_main(
    @builtin(vertex_index) vertex_index: u32,
) -> VertexOutput {
    var out: VertexOutput;

    switch (i32(vertex_index)) {
        case 0: {
            out.position = vec4<f32>(1.0, 1.0, 1.0, 1.0);
            out.tex_coords = vec2<f32>(1.0, 0.0);
        }
        case 1: {
            out.position = vec4<f32>(-1.0, 1.0, 1.0, 1.0);
            out.tex_coords = vec2<f32>(0.0, 0.0);
        }
        case 2: {
            out.position = vec4<f32>(-1.0, -1.0, 1.0, 1.0);
            out.tex_coords = vec2<f32>(0.0, 1.0);
        }
        case 3: {
            out.position = vec4<f32>(1.0, 1.0, 1.0, 1.0);
            out.tex_coords = vec2<f32>(1.0, 0.0);
        }
        case 4: {
            out.position = vec4<f32>(-1.0, -1.0, 1.0, 1.0);
            out.tex_coords = vec2<f32>(0.0, 1.0);
        }
        case 5: {
            out.position = vec4<f32>(1.0, -1.0, 1.0, 1.0);
            out.tex_coords = vec2<f32>(1.0, 1.0);
        }
        default: {}
    }

    return out;
}

@group(0) @binding(0)
var t_post: texture_2d<f32>;
@group(0) @binding(1)
var s_linear: sampler;

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    return textureSample(t_post, s_linear, in.tex_coords);
}




