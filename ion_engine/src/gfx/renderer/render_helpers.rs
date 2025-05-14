use wgpu::{VertexBufferLayout, util::DeviceExt};

use crate::util::casting::{RawData, slice_as_bytes};

/// Writes a slice of instances to a buffer.
/// If the buffer is too small, a new buffer will be created.
/// If the buffer is already the large enough, it will be written to.
/// The buffer will nevers shrink in size.
pub(super) fn write_to_buffer<T: RawData>(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    buffer: &mut wgpu::Buffer,
    instances: &[T],
) {
    if buffer.size() < (instances.len() * size_of::<T>()) as u64 {
        *buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: None,
            contents: slice_as_bytes(&instances),
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
        });
    } else {
        queue.write_buffer(buffer, 0, slice_as_bytes(instances));
    }
}

/// Builds a shader module from a given shader constant.
/// The first argument is a wgpu::Device, the second is a string literal of the shader.
#[macro_export]
macro_rules! build_shader {
    ($device:expr, $shader_const:ident) => {{
        let label = stringify!($shader_const).to_lowercase();
        $device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some(&label),
            source: wgpu::ShaderSource::Wgsl($shader_const.into()),
        })
    }};
}

/// Builds a bind group layout for a given number of texture bindings.
/// If bind_depth is true, the layout will include a depth texture binding at the end.
pub(super) fn build_tex_bind_group_layout(
    device: &wgpu::Device,
    texture_bindings: u32,
    bind_depth: bool,
    label: &str,
) -> wgpu::BindGroupLayout {
    let mut bindings: Vec<_> = (0..texture_bindings)
        .map(|binding| wgpu::BindGroupLayoutEntry {
            binding,
            visibility: wgpu::ShaderStages::FRAGMENT,
            ty: wgpu::BindingType::Texture {
                multisampled: false,
                view_dimension: wgpu::TextureViewDimension::D2,
                sample_type: wgpu::TextureSampleType::Float { filterable: true },
            },
            count: None,
        })
        .collect();

    bindings.push(wgpu::BindGroupLayoutEntry {
        binding: texture_bindings,
        visibility: wgpu::ShaderStages::FRAGMENT,
        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
        count: None,
    });

    if bind_depth {
        bindings.push(wgpu::BindGroupLayoutEntry {
            binding: texture_bindings + 1,
            visibility: wgpu::ShaderStages::FRAGMENT,
            ty: wgpu::BindingType::Texture {
                multisampled: false,
                view_dimension: wgpu::TextureViewDimension::D2,
                sample_type: wgpu::TextureSampleType::Depth,
            },
            count: None,
        });

        bindings.push(wgpu::BindGroupLayoutEntry {
            binding: texture_bindings + 2,
            visibility: wgpu::ShaderStages::FRAGMENT,
            ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::NonFiltering),
            count: None,
        });
    }

    device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        entries: &bindings,
        label: Some(label),
    })
}

/// Builds a render pipeline from given parameters.
/// Uses a standard pipeline layout. Any custom pipeline should be built manually.
pub(super) fn build_render_pipeline(
    device: &wgpu::Device,
    layout: &wgpu::PipelineLayout,
    shader: &wgpu::ShaderModule,
    sources: &[VertexBufferLayout],
    targets: &[Option<wgpu::ColorTargetState>],
    depth_stencil: Option<wgpu::DepthStencilState>,
    label: &str,
) -> wgpu::RenderPipeline {
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some(label),
        layout: Some(layout),
        vertex: wgpu::VertexState {
            module: shader,
            entry_point: Some("vs_main"),
            compilation_options: Default::default(),
            buffers: sources,
        },
        fragment: Some(wgpu::FragmentState {
            module: shader,
            entry_point: Some("fs_main"),
            compilation_options: Default::default(),
            targets,
        }),
        primitive: wgpu::PrimitiveState {
            topology: wgpu::PrimitiveTopology::TriangleList,
            strip_index_format: None,
            front_face: wgpu::FrontFace::Ccw,
            cull_mode: None,
            polygon_mode: wgpu::PolygonMode::Fill,
            unclipped_depth: false,
            conservative: false,
        },
        depth_stencil,
        multisample: wgpu::MultisampleState {
            count: 1,
            mask: !0,
            alpha_to_coverage_enabled: false,
        },
        multiview: None,
        cache: None,
    })
}
