use std::cell::RefCell;

use image::{ImageBuffer, RgbaImage};
use wgpu::CommandEncoder;
use wgpu::util::DeviceExt;

use crate::build_shader;
use crate::gfx::gfx_config::{GfxConfig, Resolution};
use crate::gfx::renderer::RenderGraph;
use crate::gfx::renderer::gpu_data_types::{SHADER_SSAO_BLUR, SHADER_SSAO_RAW};
use crate::gfx::renderer::render_camera::RenderCamera;
use crate::gfx::renderer::render_globals::RenderGlobals;
use crate::gfx::renderer::render_helpers::{build_render_pipeline, build_tex_bind_group_layout};
use crate::gfx::textures::Texture;
use crate::util::casting::slice_as_bytes;

pub(super) struct RenderPassSsao {
    render_pipeline_raw: wgpu::RenderPipeline,
    render_pipeline_blur: wgpu::RenderPipeline,

    render_target_raw: RefCell<Texture>,
    render_target_blur: RefCell<Texture>,

    source_tex_bind_group_layout: wgpu::BindGroupLayout,
    source_tex_bind_group_raw: RefCell<Option<wgpu::BindGroup>>,
    source_tex_bind_group_blur_x: RefCell<Option<wgpu::BindGroup>>,
    source_tex_bind_group_blur_y: RefCell<Option<wgpu::BindGroup>>,

    ssao_noise_bind_group: wgpu::BindGroup,
    ssao_blur_bind_group_x: wgpu::BindGroup,
    ssao_blur_bind_group_y: wgpu::BindGroup,
}

impl RenderPassSsao {
    pub(super) fn new(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        render_camera: &RenderCamera,
        render_globals: &RenderGlobals,
        gfx_config: &GfxConfig,
    ) -> Self {
        let source_tex_bind_group_layout =
            build_tex_bind_group_layout(device, 1, false, "source_tex_bind_group_layout_ssao_raw");
        let ssao_noise_bind_group_layout =
            build_tex_bind_group_layout(device, 1, false, "source_tex_bind_group_layout_ssao_noise");

        let target_format = if cfg!(target_arch = "wasm32") {
            wgpu::TextureFormat::Rgba8UnormSrgb
        } else {
            wgpu::TextureFormat::Bgra8UnormSrgb
        };

        let ssao_noise = Texture::new_from_raw_data(
            device,
            queue,
            target_format,
            &create_ssao_noise_image(),
            (4, 4),
            1,
            "ssao_noise_texture",
        );

        let ssao_noise_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            address_mode_u: wgpu::AddressMode::Repeat,
            address_mode_v: wgpu::AddressMode::Repeat,
            address_mode_w: wgpu::AddressMode::Repeat,
            mag_filter: wgpu::FilterMode::Nearest,
            min_filter: wgpu::FilterMode::Nearest,
            mipmap_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });

        let ssao_noise_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &ssao_noise_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&ssao_noise.texture_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&ssao_noise_sampler),
                },
            ],
            label: Some("ssao_noise_bind_group"),
        });

        let ssao_blur_buffer_x = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("ssao_blur_buffer_x"),
            contents: slice_as_bytes(&[1.0f32, 0.0f32, 0.0f32, 0.0f32]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });
        let ssao_blur_buffer_y = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("ssao_blur_buffer_y"),
            contents: slice_as_bytes(&[0.0f32, 0.0f32, 0.0f32, 0.0f32]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });
        let ssao_blur_bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
            label: Some("ssao_blur_bind_group_layout"),
        });

        let ssao_blur_bind_group_x = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &ssao_blur_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: ssao_blur_buffer_x.as_entire_binding(),
            }],
            label: Some("ssao_blur_bind_group_x"),
        });

        let ssao_blur_bind_group_y = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &ssao_blur_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: ssao_blur_buffer_y.as_entire_binding(),
            }],
            label: Some("ssao_blur_bind_group_y"),
        });

        let render_pipeline_layout_raw = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("render_pipeline_layout_ssao_raw"),
            bind_group_layouts: &[
                &render_globals.globals_bind_group_layout,
                &render_camera.camera_bind_group_layout,
                &source_tex_bind_group_layout,
                &ssao_noise_bind_group_layout,
            ],
            push_constant_ranges: &[],
        });

        let render_pipeline_layout_blur = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("render_pipeline_layout_ssao_blur"),
            bind_group_layouts: &[
                &render_globals.globals_bind_group_layout,
                &render_camera.camera_bind_group_layout,
                &source_tex_bind_group_layout,
                &ssao_blur_bind_group_layout,
            ],
            push_constant_ranges: &[],
        });

        let shader_ssao_raw = build_shader!(device, SHADER_SSAO_RAW);
        let shader_ssao_blur = build_shader!(device, SHADER_SSAO_BLUR);

        let render_pipeline_raw = build_render_pipeline(
            device,
            &render_pipeline_layout_raw,
            &shader_ssao_raw,
            &[],
            &[Some(wgpu::ColorTargetState {
                format: target_format,
                blend: None,
                write_mask: wgpu::ColorWrites::ALL,
            })],
            None,
            "render_pipeline_ssao_raw",
        );

        let render_pipeline_blur = build_render_pipeline(
            device,
            &render_pipeline_layout_blur,
            &shader_ssao_blur,
            &[],
            &[Some(wgpu::ColorTargetState {
                format: target_format,
                blend: None,
                write_mask: wgpu::ColorWrites::ALL,
            })],
            None,
            "render_pipeline_ssao_blur",
        );

        let ssao_dims = gfx_config.frame_resolution;
        let render_tex_ssao_raw = Texture::new_from_empty(device, ssao_dims, target_format, 1, "render_tex_ssao_raw");
        let render_tex_ssao_blur = Texture::new_from_empty(device, ssao_dims, target_format, 1, "render_tex_ssao_blur");

        Self {
            render_pipeline_raw,
            render_pipeline_blur,

            render_target_raw: RefCell::new(render_tex_ssao_raw),
            render_target_blur: RefCell::new(render_tex_ssao_blur),

            source_tex_bind_group_layout,
            source_tex_bind_group_raw: RefCell::new(None),
            source_tex_bind_group_blur_x: RefCell::new(None),
            source_tex_bind_group_blur_y: RefCell::new(None),
            ssao_noise_bind_group,
            ssao_blur_bind_group_x,
            ssao_blur_bind_group_y,
        }
    }

    pub fn set_render_graph(&self, device: &wgpu::Device, render_graph: &RenderGraph, render_resolution: Resolution) {
        let target_format = if cfg!(target_arch = "wasm32") {
            wgpu::TextureFormat::Rgba8UnormSrgb
        } else {
            wgpu::TextureFormat::Bgra8UnormSrgb
        };

        let render_tex_ssao_raw =
            Texture::new_from_empty(device, render_resolution, target_format, 1, "render_tex_ssao_raw");
        let render_tex_ssao_blur =
            Texture::new_from_empty(device, render_resolution, target_format, 1, "render_tex_ssao_blur");

        *self.render_target_raw.borrow_mut() = render_tex_ssao_raw;
        *self.render_target_blur.borrow_mut() = render_tex_ssao_blur;

        *self.source_tex_bind_group_raw.borrow_mut() = Some(device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &self.source_tex_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(
                        &render_graph.target_height_id.as_ref().unwrap().texture_view,
                    ),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&render_graph.nearest_sampler),
                },
            ],
            label: Some("render_targets_bind_group_ssao_raw"),
        }));

        *self.source_tex_bind_group_blur_x.borrow_mut() = Some(device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &self.source_tex_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&self.render_target_raw.borrow().texture_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&render_graph.mirror_sampler),
                },
            ],
            label: Some("render_targets_bind_group_ssao_blur_1"),
        }));

        *self.source_tex_bind_group_blur_y.borrow_mut() = Some(device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &self.source_tex_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&self.render_target_blur.borrow().texture_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&render_graph.mirror_sampler),
                },
            ],
            label: Some("render_targets_bind_group_ssao_blur_2"),
        }));
    }

    #[allow(clippy::too_many_arguments)]
    pub(super) fn render(
        &self,
        encoder: &mut CommandEncoder,
        render_camera: &RenderCamera,
        render_globals: &RenderGlobals,
        render_graph: &RenderGraph,
    ) {
        let render_tex_ssao_raw = self.render_target_raw.borrow();
        let render_tex_ssao_blur = self.render_target_blur.borrow();
        let render_source_1 = self.source_tex_bind_group_raw.borrow();
        let render_source_2 = self.source_tex_bind_group_blur_x.borrow();
        let render_source_3 = self.source_tex_bind_group_blur_y.borrow();

        let mut render_pass_raw = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("render_pass_ssao_raw"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &render_tex_ssao_raw.texture_view,
                //view: &render_graph.target_ssao.as_ref().unwrap().texture_view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    store: wgpu::StoreOp::Store,
                },
            })],
            ..Default::default()
        });

        render_pass_raw.set_bind_group(0, &render_globals.globals_bind_group, &[]);
        render_pass_raw.set_bind_group(1, &render_camera.camera_bind_group, &[]);
        render_pass_raw.set_bind_group(2, render_source_1.as_ref().unwrap(), &[]);
        render_pass_raw.set_bind_group(3, &self.ssao_noise_bind_group, &[]);
        render_pass_raw.set_pipeline(&self.render_pipeline_raw);
        render_pass_raw.draw(0..6, 0..1);

        drop(render_pass_raw);

        let mut render_pass_blur_x = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("render_pass_ssao_blur_1"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &render_tex_ssao_blur.texture_view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    store: wgpu::StoreOp::Store,
                },
            })],
            ..Default::default()
        });

        render_pass_blur_x.set_bind_group(0, &render_globals.globals_bind_group, &[]);
        render_pass_blur_x.set_bind_group(1, &render_camera.camera_bind_group, &[]);
        render_pass_blur_x.set_bind_group(2, render_source_2.as_ref().unwrap(), &[]);
        render_pass_blur_x.set_bind_group(3, &self.ssao_blur_bind_group_x, &[]);
        render_pass_blur_x.set_pipeline(&self.render_pipeline_blur);
        render_pass_blur_x.draw(0..6, 0..1);

        drop(render_pass_blur_x);

        let mut render_pass_blur_y = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("render_pass_ssao_blur_2"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &render_graph.target_ssao.as_ref().unwrap().texture_view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    store: wgpu::StoreOp::Store,
                },
            })],
            ..Default::default()
        });

        render_pass_blur_y.set_bind_group(0, &render_globals.globals_bind_group, &[]);
        render_pass_blur_y.set_bind_group(1, &render_camera.camera_bind_group, &[]);
        render_pass_blur_y.set_bind_group(2, render_source_3.as_ref().unwrap(), &[]);
        render_pass_blur_y.set_bind_group(3, &self.ssao_blur_bind_group_y, &[]);
        render_pass_blur_y.set_pipeline(&self.render_pipeline_blur);
        render_pass_blur_y.draw(0..6, 0..1);
    }
}

fn create_ssao_noise_image() -> RgbaImage {
    let mut image: RgbaImage = ImageBuffer::new(4, 4);

    image[(0, 0)] = [58, 20, 0, 255].into();
    image[(1, 0)] = [0, 140, 0, 255].into();
    image[(2, 0)] = [55, 22, 0, 255].into();
    image[(3, 0)] = [234, 198, 0, 255].into();
    image[(0, 1)] = [2, 154, 0, 255].into();
    image[(1, 1)] = [50, 229, 0, 255].into();
    image[(2, 1)] = [214, 222, 0, 255].into();
    image[(3, 1)] = [69, 13, 0, 255].into();
    image[(0, 2)] = [69, 14, 0, 255].into();
    image[(1, 2)] = [25, 51, 0, 255].into();
    image[(2, 2)] = [161, 251, 0, 255].into();
    image[(3, 2)] = [66, 240, 0, 255].into();
    image[(0, 3)] = [42, 223, 0, 255].into();
    image[(1, 3)] = [239, 190, 0, 255].into();
    image[(2, 3)] = [248, 171, 0, 255].into();
    image[(3, 3)] = [154, 2, 0, 255].into();

    image
}
