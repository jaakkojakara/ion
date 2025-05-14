use std::cell::RefCell;
use std::mem::size_of;

use crate::build_shader;
use crate::gfx::GfxDebugData;
use crate::gfx::renderer::gpu_data_types::{LineVertex, SHADER_DEBUG, SHADER_SCALE};
use crate::gfx::renderer::render_camera::RenderCamera;
use crate::gfx::renderer::render_globals::RenderGlobals;
use wgpu::PolygonMode::Line;
use wgpu::util::DeviceExt;
use wgpu::{Buffer, BufferDescriptor, CommandEncoder, SurfaceConfiguration, TextureView};

use crate::gfx::renderer::render_helpers::{build_render_pipeline, build_tex_bind_group_layout};
use crate::util::casting::slice_as_bytes;

use super::RenderGraph;

pub(super) struct RenderPassFinal {
    render_pipeline_final: wgpu::RenderPipeline,
    render_pipeline_debug: Option<wgpu::RenderPipeline>,

    debug_line_buffer: Buffer,

    source_tex_bind_group_layout: wgpu::BindGroupLayout,
    source_tex_bind_group: RefCell<Option<wgpu::BindGroup>>,
}

impl RenderPassFinal {
    pub(super) fn new(
        device: &wgpu::Device,
        surface_config: &SurfaceConfiguration,
        render_globals: &RenderGlobals,
        render_camera: &RenderCamera,
    ) -> Self {
        let source_tex_bind_group_layout =
            build_tex_bind_group_layout(device, 1, false, "source_tex_bind_group_layout_final");

        let render_pipeline_layout_final = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("render_pipeline_layout_final"),
            bind_group_layouts: &[&source_tex_bind_group_layout],
            push_constant_ranges: &[],
        });

        let render_pipeline_layout_debug = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("render_pipeline_layout_primitive"),
            bind_group_layouts: &[&render_globals.globals_bind_group_layout, &render_camera.camera_bind_group_layout],
            push_constant_ranges: &[],
        });

        let shader_final = build_shader!(device, SHADER_SCALE);
        let shader_debug = build_shader!(device, SHADER_DEBUG);

        let render_pipeline_final = build_render_pipeline(
            device,
            &render_pipeline_layout_final,
            &shader_final,
            &[],
            &[Some(wgpu::ColorTargetState {
                format: surface_config.format,
                blend: None,
                write_mask: wgpu::ColorWrites::ALL,
            })],
            None,
            "render_pipeline_final",
        );

        let render_pipeline_debug = if cfg!(not(target_arch = "wasm32")) {
            Some(device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("render_pipeline_debug"),
                layout: Some(&render_pipeline_layout_debug),
                vertex: wgpu::VertexState {
                    module: &shader_debug,
                    entry_point: Some("vs_main"),
                    compilation_options: Default::default(),
                    buffers: &[LineVertex::buffer_layout()],
                },
                fragment: Some(wgpu::FragmentState {
                    module: &shader_debug,
                    entry_point: Some("fs_main"),
                    compilation_options: Default::default(),
                    targets: &[Some(wgpu::ColorTargetState {
                        format: surface_config.format,
                        blend: Some(wgpu::BlendState {
                            color: wgpu::BlendComponent::REPLACE,
                            alpha: wgpu::BlendComponent::REPLACE,
                        }),
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                }),
                primitive: wgpu::PrimitiveState {
                    topology: wgpu::PrimitiveTopology::LineList,
                    strip_index_format: None,
                    front_face: wgpu::FrontFace::Ccw,
                    cull_mode: None,
                    polygon_mode: Line,
                    unclipped_depth: false,
                    conservative: false,
                },
                depth_stencil: None,
                multisample: wgpu::MultisampleState {
                    count: 1,
                    mask: !0,
                    alpha_to_coverage_enabled: false,
                },
                multiview: None,
                cache: None,
            }))
        } else {
            None
        };

        let debug_line_buffer = device.create_buffer(&BufferDescriptor {
            label: Some("line_buffer"),
            size: 1024 * size_of::<LineVertex>() as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        Self {
            render_pipeline_final,
            render_pipeline_debug,
            debug_line_buffer,
            source_tex_bind_group_layout,
            source_tex_bind_group: RefCell::new(None),
        }
    }

    pub fn set_render_graph(&self, device: &wgpu::Device, render_graph: &RenderGraph) {
        *self.source_tex_bind_group.borrow_mut() = Some(device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &self.source_tex_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(
                        &render_graph.target_post_2.as_ref().unwrap().texture_view,
                    ),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&render_graph.linear_sampler),
                },
            ],
            label: Some("render_targets_bind_group_final"),
        }));
    }

    #[allow(clippy::too_many_arguments)]
    pub(super) fn render(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        encoder: &mut CommandEncoder,
        surface_view: &TextureView,
        render_camera: &RenderCamera,
        render_globals: &RenderGlobals,
        debug_data: &GfxDebugData,
    ) {
        let render_sources = self.source_tex_bind_group.borrow();
        let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("render_pass_final"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: surface_view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    store: wgpu::StoreOp::Store,
                },
            })],
            ..Default::default()
        });

        render_pass.set_bind_group(0, render_sources.as_ref().unwrap(), &[]);
        render_pass.set_pipeline(&self.render_pipeline_final);
        render_pass.draw(0..6, 0..1);

        drop(render_pass);

        if self.render_pipeline_debug.is_some() && !debug_data.debug_shapes.is_empty() {
            let vertices: Vec<_> = debug_data
                .debug_shapes
                .iter()
                .flat_map(|shape| shape.as_line_vertices())
                .collect();

            if self.debug_line_buffer.size() < (vertices.len() * size_of::<LineVertex>()) as u64 {
                self.debug_line_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("line_buffer"),
                    contents: slice_as_bytes(vertices.as_slice()),
                    usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                });
            } else {
                queue.write_buffer(&self.debug_line_buffer, 0, slice_as_bytes(vertices.as_slice()));
            }

            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("render_pass_primitive"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: surface_view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    },
                })],
                ..Default::default()
            });

            render_pass.set_bind_group(0, &render_globals.globals_bind_group, &[]);
            render_pass.set_bind_group(1, &render_camera.camera_bind_group, &[]);
            render_pass.set_vertex_buffer(0, self.debug_line_buffer.slice(..));

            render_pass.set_pipeline(self.render_pipeline_debug.as_ref().unwrap());
            render_pass.draw(0..(vertices.len() as u32), 0..1);
        }
    }
}
