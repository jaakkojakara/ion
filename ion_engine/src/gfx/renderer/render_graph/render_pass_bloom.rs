use std::cell::RefCell;
use std::default::Default;

use wgpu::CommandEncoder;

use crate::build_shader;
use crate::gfx::WASM_COMPATIBLE_RENDERING;
use crate::gfx::renderer::RenderGraph;
use crate::gfx::renderer::gpu_data_types::{SHADER_BLOOM_DS, SHADER_BLOOM_US};
use crate::gfx::renderer::render_camera::RenderCamera;
use crate::gfx::renderer::render_globals::RenderGlobals;
use crate::gfx::renderer::render_helpers::{build_render_pipeline, build_tex_bind_group_layout};

pub(super) struct RenderPassBloom {
    source_tex_bind_group_layout: wgpu::BindGroupLayout,
    source_tex_bind_group: RefCell<Option<wgpu::BindGroup>>,

    mip_sampler: wgpu::Sampler,

    mip_ds_pipeline: wgpu::RenderPipeline,
    mip_ds_bind_groups: RefCell<Vec<wgpu::BindGroup>>,
    mip_ds_views: RefCell<Vec<wgpu::TextureView>>,

    mip_us_pipeline: wgpu::RenderPipeline,
    mip_us_bind_groups: RefCell<Vec<wgpu::BindGroup>>,
    mip_us_views: RefCell<Vec<wgpu::TextureView>>,
}

impl RenderPassBloom {
    pub(super) fn new(device: &wgpu::Device, render_camera: &RenderCamera, render_globals: &RenderGlobals) -> Self {
        let source_tex_bind_group_layout =
            build_tex_bind_group_layout(device, 1, false, "render_targets_bind_group_layout_bloom");

        let render_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("render_pipeline_layout_bloom"),
            bind_group_layouts: &[
                &render_globals.globals_bind_group_layout,
                &render_camera.camera_bind_group_layout,
                &source_tex_bind_group_layout,
            ],
            push_constant_ranges: &[],
        });

        let shader_ds = build_shader!(device, SHADER_BLOOM_DS);
        let shader_us = build_shader!(device, SHADER_BLOOM_US);

        let target_format = if WASM_COMPATIBLE_RENDERING {
            wgpu::TextureFormat::Rgba16Float
        } else {
            wgpu::TextureFormat::Rg11b10Ufloat
        };

        let mip_ds_pipeline = build_render_pipeline(
            device,
            &render_pipeline_layout,
            &shader_ds,
            &[],
            &[Some(wgpu::ColorTargetState {
                format: target_format,
                blend: None,
                write_mask: wgpu::ColorWrites::ALL,
            })],
            None,
            "render_pipeline_bloom_ds",
        );

        let mip_us_pipeline = build_render_pipeline(
            device,
            &render_pipeline_layout,
            &shader_us,
            &[],
            &[Some(wgpu::ColorTargetState {
                format: target_format,
                blend: None,
                write_mask: wgpu::ColorWrites::ALL,
            })],
            None,
            "render_pipeline_bloom_us",
        );

        let mip_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("mipmap_sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });

        Self {
            source_tex_bind_group_layout,
            source_tex_bind_group: RefCell::new(None),
            mip_sampler,

            mip_ds_pipeline,
            mip_ds_views: RefCell::new(Vec::new()),
            mip_ds_bind_groups: RefCell::new(Vec::new()),

            mip_us_pipeline,
            mip_us_views: RefCell::new(Vec::new()),
            mip_us_bind_groups: RefCell::new(Vec::new()),
        }
    }

    pub(crate) fn set_render_graph(&self, device: &wgpu::Device, render_graph: &RenderGraph) {
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
                    resource: wgpu::BindingResource::Sampler(&render_graph.linear_sampler),
                },
            ],
            label: Some("render_targets_bind_group_bloom"),
        }));

        *self.mip_ds_views.borrow_mut() = (0..5)
            .map(|mip| {
                render_graph
                    .target_post_1
                    .as_ref()
                    .unwrap()
                    .texture
                    .create_view(&wgpu::TextureViewDescriptor {
                        base_mip_level: mip,
                        mip_level_count: Some(1),
                        ..wgpu::TextureViewDescriptor::default()
                    })
            })
            .collect::<Vec<_>>();

        *self.mip_us_views.borrow_mut() = (0..4)
            .map(|mip| {
                render_graph
                    .target_bloom
                    .as_ref()
                    .unwrap()
                    .texture
                    .create_view(&wgpu::TextureViewDescriptor {
                        base_mip_level: mip,
                        mip_level_count: Some(1),
                        ..wgpu::TextureViewDescriptor::default()
                    })
            })
            .collect::<Vec<_>>();

        *self.mip_ds_bind_groups.borrow_mut() = (0..5)
            .map(|mip| {
                device.create_bind_group(&wgpu::BindGroupDescriptor {
                    layout: &self.source_tex_bind_group_layout,
                    entries: &[
                        wgpu::BindGroupEntry {
                            binding: 0,
                            resource: wgpu::BindingResource::TextureView(&self.mip_ds_views.borrow()[mip]),
                        },
                        wgpu::BindGroupEntry {
                            binding: 1,
                            resource: wgpu::BindingResource::Sampler(&self.mip_sampler),
                        },
                    ],
                    label: None,
                })
            })
            .collect::<Vec<_>>();

        *self.mip_us_bind_groups.borrow_mut() = (1..4)
            .map(|mip| {
                device.create_bind_group(&wgpu::BindGroupDescriptor {
                    layout: &self.source_tex_bind_group_layout,
                    entries: &[
                        wgpu::BindGroupEntry {
                            binding: 0,
                            resource: wgpu::BindingResource::TextureView(&self.mip_us_views.borrow()[mip]),
                        },
                        wgpu::BindGroupEntry {
                            binding: 1,
                            resource: wgpu::BindingResource::Sampler(&self.mip_sampler),
                        },
                    ],
                    label: None,
                })
            })
            .collect::<Vec<_>>();
    }

    pub(super) fn render(
        &self,
        encoder: &mut CommandEncoder,
        render_camera: &RenderCamera,
        render_globals: &RenderGlobals,
    ) {
        let mip_ds_bind_groups = self.mip_ds_bind_groups.borrow();
        let mip_ds_views = self.mip_ds_views.borrow();

        let mip_us_bind_groups = self.mip_us_bind_groups.borrow();
        let mip_us_views = self.mip_us_views.borrow();

        for target_mip in 1..5 {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("render_pass_bloom_ds"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &mip_ds_views[target_mip],
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    },
                })],
                ..Default::default()
            });

            render_pass.set_pipeline(&self.mip_ds_pipeline);
            render_pass.set_bind_group(0, &render_globals.globals_bind_group, &[]);
            render_pass.set_bind_group(1, &render_camera.camera_bind_group, &[]);
            render_pass.set_bind_group(2, &mip_ds_bind_groups[target_mip - 1], &[]);
            render_pass.draw(0..6, 0..1);
        }

        for target_mip in (1..5).rev() {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("render_pass_bloom_us"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &mip_us_views[target_mip - 1],
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    },
                })],
                ..Default::default()
            });

            let source_binding = if target_mip == 4 {
                &mip_ds_bind_groups[target_mip]
            } else {
                &mip_us_bind_groups[target_mip - 1]
            };

            render_pass.set_pipeline(&self.mip_us_pipeline);
            render_pass.set_bind_group(0, &render_globals.globals_bind_group, &[]);
            render_pass.set_bind_group(1, &render_camera.camera_bind_group, &[]);
            render_pass.set_bind_group(2, source_binding, &[]);
            render_pass.draw(0..6, 0..1);
        }
    }
}
