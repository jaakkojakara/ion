use std::cell::RefCell;

use crate::gfx::renderer::gpu_data_types::SHADER_POST_1;
use crate::gfx::renderer::render_camera::RenderCamera;
use crate::gfx::renderer::render_globals::RenderGlobals;
use crate::{build_shader, gfx::WASM_COMPATIBLE_RENDERING};
use wgpu::CommandEncoder;

use crate::gfx::renderer::render_helpers::{build_render_pipeline, build_tex_bind_group_layout};

use super::RenderGraph;

pub(super) struct RenderPassPost1 {
    render_pipeline_post_1: wgpu::RenderPipeline,

    source_tex_bind_group_layout: wgpu::BindGroupLayout,
    source_tex_bind_group: RefCell<Option<wgpu::BindGroup>>,
}

impl RenderPassPost1 {
    pub(super) fn new(device: &wgpu::Device, render_globals: &RenderGlobals, render_camera: &RenderCamera) -> Self {
        let source_tex_bind_group_layout =
            build_tex_bind_group_layout(device, 5, true, "source_tex_bind_group_layout_post_1");

        let render_pipeline_layout_post_1 = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("render_pipeline_layout_post_1"),
            bind_group_layouts: &[
                &render_globals.globals_bind_group_layout,
                &render_camera.camera_bind_group_layout,
                &source_tex_bind_group_layout,
            ],
            push_constant_ranges: &[],
        });

        let shader_post_1 = build_shader!(device, SHADER_POST_1);

        let target_format = if WASM_COMPATIBLE_RENDERING {
            wgpu::TextureFormat::Rgba16Float
        } else {
            wgpu::TextureFormat::Rg11b10Ufloat
        };

        let render_pipeline_post_1 = build_render_pipeline(
            device,
            &render_pipeline_layout_post_1,
            &shader_post_1,
            &[],
            &[Some(wgpu::ColorTargetState {
                format: target_format,
                blend: None,
                write_mask: wgpu::ColorWrites::ALL,
            })],
            None,
            "render_pipeline_post_1",
        );

        Self {
            render_pipeline_post_1,

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
                        &render_graph.target_color.as_ref().unwrap().texture_view,
                    ),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(
                        &render_graph.target_normal.as_ref().unwrap().texture_view,
                    ),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::TextureView(
                        &render_graph.target_height_id.as_ref().unwrap().texture_view,
                    ),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: wgpu::BindingResource::TextureView(
                        &render_graph.target_light_shadow.as_ref().unwrap().texture_view,
                    ),
                },
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: wgpu::BindingResource::TextureView(
                        &render_graph.target_ssao.as_ref().unwrap().texture_view,
                    ),
                },
                wgpu::BindGroupEntry {
                    binding: 5,
                    resource: wgpu::BindingResource::Sampler(&render_graph.linear_sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 6,
                    resource: wgpu::BindingResource::TextureView(
                        &render_graph.target_depth.as_ref().unwrap().texture_view,
                    ),
                },
                wgpu::BindGroupEntry {
                    binding: 7,
                    resource: wgpu::BindingResource::Sampler(&render_graph.nearest_sampler),
                },
            ],
            label: Some("render_targets_bind_group_post_1"),
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
            label: Some("render_pass_post_1"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &render_graph.target_post_1.as_ref().unwrap().texture_view,
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
        render_pass.set_pipeline(&self.render_pipeline_post_1);
        render_pass.draw(0..6, 0..1);
    }
}
