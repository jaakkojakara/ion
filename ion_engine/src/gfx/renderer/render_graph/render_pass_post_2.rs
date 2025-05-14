use std::cell::RefCell;

use crate::build_shader;
use crate::gfx::renderer::gpu_data_types::SHADER_POST_2;
use crate::gfx::renderer::render_camera::RenderCamera;
use crate::gfx::renderer::render_globals::RenderGlobals;
use wgpu::CommandEncoder;

use crate::gfx::renderer::render_helpers::{build_render_pipeline, build_tex_bind_group_layout};

use super::RenderGraph;

pub(super) struct RenderPassPost2 {
    render_pipeline_post_2: wgpu::RenderPipeline,

    source_tex_bind_group_layout: wgpu::BindGroupLayout,
    source_tex_bind_group: RefCell<Option<wgpu::BindGroup>>,
}

impl RenderPassPost2 {
    pub(super) fn new(device: &wgpu::Device, render_globals: &RenderGlobals, render_camera: &RenderCamera) -> Self {
        let source_tex_bind_group_layout =
            build_tex_bind_group_layout(device, 2, false, "source_tex_bind_group_layout_post_2");

        let render_pipeline_layout_post_2 = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("render_pipeline_layout_post_2"),
            bind_group_layouts: &[
                &render_globals.globals_bind_group_layout,
                &render_camera.camera_bind_group_layout,
                &source_tex_bind_group_layout,
            ],
            push_constant_ranges: &[],
        });

        let shader_post_2 = build_shader!(device, SHADER_POST_2);

        let ldr_target_format = if cfg!(target_arch = "wasm32") {
            wgpu::TextureFormat::Rgba8UnormSrgb
        } else {
            wgpu::TextureFormat::Bgra8UnormSrgb
        };

        let render_pipeline_post_2 = build_render_pipeline(
            device,
            &render_pipeline_layout_post_2,
            &shader_post_2,
            &[],
            &[Some(wgpu::ColorTargetState {
                format: ldr_target_format,
                blend: None,
                write_mask: wgpu::ColorWrites::ALL,
            })],
            None,
            "render_pipeline_post_2",
        );

        Self {
            render_pipeline_post_2,

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
                        &render_graph.target_post_1.as_ref().unwrap().texture_view,
                    ),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(
                        &render_graph.target_bloom.as_ref().unwrap().texture_view,
                    ),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::Sampler(&render_graph.linear_sampler),
                },
            ],
            label: Some("render_targets_bind_group_post_2"),
        }));
    }

    pub(super) fn render(
        &self,
        encoder: &mut CommandEncoder,
        render_camera: &RenderCamera,
        render_globals: &RenderGlobals,
        render_graph: &RenderGraph,
    ) {
        let render_sources = self.source_tex_bind_group.borrow();
        let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("render_pass_post_2"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &render_graph.target_post_2.as_ref().unwrap().texture_view,
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
        render_pass.set_bind_group(2, render_sources.as_ref().unwrap(), &[]);
        render_pass.set_pipeline(&self.render_pipeline_post_2);
        render_pass.draw(0..6, 0..1);
    }
}
