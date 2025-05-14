use std::collections::VecDeque;

use ion_common::Map;
use render_pass_final::RenderPassFinal;
use render_pass_gbuf::RenderPassGBuf;
use render_pass_shadow::RenderPassShadow;
use wgpu::util::DeviceExt;

use crate::{
    WASM_COMPATIBLE_RENDERING,
    core::{GfxConstants, coordinates::ChunkLocation},
    gfx::{
        GfxFrameData, GfxSpriteData,
        gfx_config::{GfxConfig, Resolution},
        renderer::{
            gpu_data_types::{InstanceLight, InstanceSprite},
            render_graph::{
                render_pass_bloom::RenderPassBloom, render_pass_light::RenderPassLight,
                render_pass_post_1::RenderPassPost1, render_pass_post_2::RenderPassPost2,
                render_pass_ssao::RenderPassSsao,
            },
            render_helpers::write_to_buffer,
        },
        textures::{
            Texture,
            texture_assets::{DrawCall, DrawCallWasm, TextureAssets},
        },
    },
    util::casting::slice_as_bytes,
};

use super::{
    gpu_data_types::{build_index_vec, build_vertex_vec},
    render_camera::RenderCamera,
    render_globals::RenderGlobals,
};

mod render_pass_bloom;
mod render_pass_final;
mod render_pass_gbuf;
mod render_pass_light;
mod render_pass_post_1;
mod render_pass_post_2;
mod render_pass_shadow;
mod render_pass_ssao;

pub(super) struct RenderGraph {
    vertex_buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,

    free_buffers: VecDeque<Buffers>,
    chunk_buffers: Map<ChunkLocation, Buffers>,
    dynamic_buffers: Buffers,

    render_pass_gbuf: Option<RenderPassGBuf>,
    render_pass_final: Option<RenderPassFinal>,
    render_pass_light: Option<RenderPassLight>,
    render_pass_shadow: Option<RenderPassShadow>,
    render_pass_ssao: Option<RenderPassSsao>,
    render_pass_post_1: Option<RenderPassPost1>,
    render_pass_post_2: Option<RenderPassPost2>,
    render_pass_bloom: Option<RenderPassBloom>,

    target_color: Option<Texture>,
    target_normal: Option<Texture>,
    target_height_id: Option<Texture>,
    target_depth: Option<Texture>,
    target_light_shadow: Option<Texture>,
    target_ssao: Option<Texture>,
    target_bloom: Option<Texture>,
    target_post_1: Option<Texture>,
    target_post_2: Option<Texture>,

    linear_sampler: wgpu::Sampler,
    nearest_sampler: wgpu::Sampler,
    mirror_sampler: wgpu::Sampler,
}

impl RenderGraph {
    pub fn new(constants: &GfxConstants, device: &wgpu::Device) -> Self {
        let vertices: Vec<_> = build_vertex_vec(RenderCamera::calc_camera_x_y_ratio(constants));
        let indices: Vec<_> = build_index_vec();

        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("vertex_buffer"),
            contents: slice_as_bytes(&vertices),
            usage: wgpu::BufferUsages::VERTEX,
        });
        let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("index_buffer"),
            contents: slice_as_bytes(&indices),
            usage: wgpu::BufferUsages::INDEX,
        });

        let linear_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });

        let nearest_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Nearest,
            min_filter: wgpu::FilterMode::Nearest,
            mipmap_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });

        let mirror_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            address_mode_u: wgpu::AddressMode::MirrorRepeat,
            address_mode_v: wgpu::AddressMode::MirrorRepeat,
            address_mode_w: wgpu::AddressMode::MirrorRepeat,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });

        let dynamic_buffers = Buffers {
            color_buf: device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("dynamic_color_buffer"),
                size: 128 * size_of::<InstanceSprite>() as u64,
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            }),
            shadow_buf: device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("dynamic_shadow_buffer"),
                size: 128 * size_of::<InstanceSprite>() as u64,
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            }),
            light_buf: device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("dynamic_light_buffer"),
                size: 128 * size_of::<InstanceLight>() as u64,
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            }),
            color_draw_calls: Vec::new(),
            shadow_draw_calls: Vec::new(),
            light_draw_calls: Vec::new(),
            color_draw_calls_wasm: Vec::new(),
            shadow_draw_calls_wasm: Vec::new(),
            light_draw_calls_wasm: Vec::new(),
        };

        Self {
            vertex_buffer,
            index_buffer,
            render_pass_gbuf: None,
            render_pass_final: None,
            render_pass_light: None,
            render_pass_shadow: None,
            render_pass_ssao: None,
            render_pass_post_1: None,
            render_pass_post_2: None,
            render_pass_bloom: None,

            free_buffers: VecDeque::new(),
            chunk_buffers: Map::default(),
            dynamic_buffers,

            target_color: None,
            target_normal: None,
            target_height_id: None,
            target_depth: None,
            target_light_shadow: None,
            target_ssao: None,
            target_bloom: None,
            target_post_1: None,
            target_post_2: None,

            linear_sampler,
            nearest_sampler,
            mirror_sampler,
        }
    }

    pub fn create_render_passes(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        render_camera: &RenderCamera,
        render_globals: &RenderGlobals,
        texture_assets: &TextureAssets,
        gfx_config: &GfxConfig,
        surface_config: &wgpu::SurfaceConfiguration,
    ) {
        self.render_pass_gbuf = Some(RenderPassGBuf::new(
            device,
            render_camera,
            render_globals,
            &texture_assets.bind_group_layout(),
        ));

        self.render_pass_light = Some(RenderPassLight::new(
            device,
            render_camera,
            render_globals,
            &texture_assets.bind_group_layout(),
        ));

        self.render_pass_shadow = Some(RenderPassShadow::new(
            device,
            render_camera,
            render_globals,
            &texture_assets.bind_group_layout(),
        ));

        self.render_pass_ssao = Some(RenderPassSsao::new(
            device,
            queue,
            render_camera,
            render_globals,
            gfx_config,
        ));

        self.render_pass_post_1 = Some(RenderPassPost1::new(device, render_globals, render_camera));
        self.render_pass_post_2 = Some(RenderPassPost2::new(device, render_globals, render_camera));
        self.render_pass_bloom = Some(RenderPassBloom::new(device, render_camera, render_globals));

        self.render_pass_final = Some(RenderPassFinal::new(
            device,
            surface_config,
            render_globals,
            render_camera,
        ));
    }

    pub fn create_render_targets(&mut self, device: &wgpu::Device, render_resolution: Resolution) {
        let ldr_target_format = if cfg!(target_arch = "wasm32") {
            wgpu::TextureFormat::Rgba8UnormSrgb
        } else {
            wgpu::TextureFormat::Bgra8UnormSrgb
        };

        let post_target_format = if WASM_COMPATIBLE_RENDERING {
            wgpu::TextureFormat::Rgba16Float
        } else {
            wgpu::TextureFormat::Rg11b10Ufloat
        };

        self.target_color = Some(Texture::new_from_empty(
            device,
            render_resolution,
            ldr_target_format,
            1,
            "color_target",
        ));
        self.target_normal = Some(Texture::new_from_empty(
            device,
            render_resolution,
            ldr_target_format,
            1,
            "normal_target",
        ));
        self.target_height_id = Some(Texture::new_from_empty(
            device,
            render_resolution,
            ldr_target_format,
            1,
            "height_id_target",
        ));
        self.target_depth = Some(Texture::new_from_empty(
            device,
            render_resolution,
            wgpu::TextureFormat::Depth32Float,
            1,
            "depth_target",
        ));
        self.target_light_shadow = Some(Texture::new_from_empty(
            device,
            render_resolution,
            wgpu::TextureFormat::Rgba16Float,
            1,
            "light_shadow_target",
        ));
        self.target_ssao = Some(Texture::new_from_empty(
            device,
            render_resolution,
            ldr_target_format,
            1,
            "ssao_target",
        ));
        self.target_bloom = Some(Texture::new_from_empty(
            device,
            render_resolution,
            post_target_format,
            4, // For bloom upscaling
            "bloom_target",
        ));
        self.target_post_1 = Some(Texture::new_from_empty(
            device,
            render_resolution,
            post_target_format,
            5, // For bloom downscaling
            "post_1_target",
        ));
        self.target_post_2 = Some(Texture::new_from_empty(
            device,
            render_resolution,
            ldr_target_format,
            1,
            "post_2_target",
        ));

        if let Some(render_pass_final) = self.render_pass_final.as_ref() {
            render_pass_final.set_render_graph(device, &self);
        }

        if let Some(render_pass_light) = self.render_pass_light.as_ref() {
            render_pass_light.set_render_graph(device, &self);
        }

        if let Some(render_pass_shadow) = self.render_pass_shadow.as_ref() {
            render_pass_shadow.set_render_graph(device, &self);
        }

        if let Some(render_pass_ssao) = self.render_pass_ssao.as_ref() {
            render_pass_ssao.set_render_graph(device, &self, render_resolution);
        }

        if let Some(render_pass_post_1) = self.render_pass_post_1.as_ref() {
            render_pass_post_1.set_render_graph(device, &self);
        }

        if let Some(render_pass_post_2) = self.render_pass_post_2.as_ref() {
            render_pass_post_2.set_render_graph(device, &self);
        }

        if let Some(render_pass_bloom) = self.render_pass_bloom.as_ref() {
            render_pass_bloom.set_render_graph(device, &self);
        }
    }

    pub fn render_graph_ready(&self) -> bool {
        let passes_initialized = self.render_pass_gbuf.is_some();
        let targets_initialized = self.target_color.is_some();

        passes_initialized && targets_initialized
    }

    pub fn execute_render_graph(
        &mut self,
        encoder: &mut wgpu::CommandEncoder,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        surface_view: &wgpu::TextureView,
        render_camera: &RenderCamera,
        render_globals: &RenderGlobals,
        texture_assets: &TextureAssets,
        gfx_frame_data: &GfxFrameData,
    ) {
        // Update buffers
        if WASM_COMPATIBLE_RENDERING {
            self.update_buffers_wasm(device, queue, texture_assets, &gfx_frame_data.sprite_data);
        } else {
            self.update_buffers_native(
                device,
                queue,
                render_camera,
                texture_assets,
                &gfx_frame_data.sprite_data,
            );
        }

        // Run all the render passes
        self.render_pass_gbuf
            .as_ref()
            .unwrap()
            .render(encoder, render_camera, render_globals, &self, texture_assets);

        self.render_pass_light
            .as_ref()
            .unwrap()
            .render(encoder, render_camera, render_globals, &self, texture_assets);

        self.render_pass_shadow
            .as_ref()
            .unwrap()
            .render(encoder, render_camera, render_globals, &self, texture_assets);

        self.render_pass_post_1
            .as_ref()
            .unwrap()
            .render(encoder, render_camera, render_globals, &self);

        self.render_pass_ssao
            .as_ref()
            .unwrap()
            .render(encoder, render_camera, render_globals, &self);

        self.render_pass_bloom
            .as_ref()
            .unwrap()
            .render(encoder, render_camera, render_globals);

        self.render_pass_post_2
            .as_ref()
            .unwrap()
            .render(encoder, render_camera, render_globals, &self);

        self.render_pass_final.as_mut().unwrap().render(
            device,
            queue,
            encoder,
            surface_view,
            render_camera,
            render_globals,
            &gfx_frame_data.debug_data,
        );
    }

    fn update_buffers_native(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        render_camera: &RenderCamera,
        texture_assets: &TextureAssets,
        gfx_sprite_data: &GfxSpriteData,
    ) {
        // Remove chunks that are no longer being rendered
        let chunks_to_remove: Vec<_> = self
            .chunk_buffers
            .keys()
            .filter(|&chunk_location| !gfx_sprite_data.chunked_gfx.contains_key(chunk_location))
            .copied()
            .collect();

        for chunk_location in chunks_to_remove {
            self.free_buffers
                .push_back(self.chunk_buffers.remove(&chunk_location).unwrap());
        }

        // Update chunk buffers that received new data
        for (chunk_location, chunk_data) in &gfx_sprite_data.chunked_gfx {
            if let Some(chunk_data) = chunk_data {
                let (
                    (instances_color, draw_calls_color),
                    (instances_shadow, draw_calls_shadow),
                    (instances_light, draw_calls_light),
                ) = texture_assets.refs_to_draw_calls(&chunk_data, render_camera);

                let buffers = self.chunk_buffers.entry(*chunk_location).or_insert_with(|| {
                    self.free_buffers.pop_front().unwrap_or_else(|| Buffers {
                        color_buf: device.create_buffer(&wgpu::BufferDescriptor {
                            label: Some("dynamic_color_buffer"),
                            size: 128 * size_of::<InstanceSprite>() as u64,
                            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                            mapped_at_creation: false,
                        }),
                        shadow_buf: device.create_buffer(&wgpu::BufferDescriptor {
                            label: Some("dynamic_shadow_buffer"),
                            size: 128 * size_of::<InstanceSprite>() as u64,
                            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                            mapped_at_creation: false,
                        }),
                        light_buf: device.create_buffer(&wgpu::BufferDescriptor {
                            label: Some("dynamic_light_buffer"),
                            size: 128 * size_of::<InstanceLight>() as u64,
                            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                            mapped_at_creation: false,
                        }),
                        color_draw_calls: Vec::new(),
                        shadow_draw_calls: Vec::new(),
                        light_draw_calls: Vec::new(),
                        color_draw_calls_wasm: Vec::new(),
                        shadow_draw_calls_wasm: Vec::new(),
                        light_draw_calls_wasm: Vec::new(),
                    })
                });

                write_to_buffer(device, queue, &mut buffers.color_buf, &instances_color);
                write_to_buffer(device, queue, &mut buffers.shadow_buf, &instances_shadow);
                write_to_buffer(device, queue, &mut buffers.light_buf, &instances_light);

                buffers.color_draw_calls = draw_calls_color;
                buffers.shadow_draw_calls = draw_calls_shadow;
                buffers.light_draw_calls = draw_calls_light;
            }
        }

        // Update dynamic buffers
        let (
            (instances_color, draw_calls_color),
            (instances_shadow, draw_calls_shadow),
            (instances_light, draw_calls_light),
        ) = texture_assets.refs_to_draw_calls(&gfx_sprite_data.dynamic_gfx, render_camera);

        write_to_buffer(device, queue, &mut self.dynamic_buffers.color_buf, &instances_color);
        write_to_buffer(device, queue, &mut self.dynamic_buffers.shadow_buf, &instances_shadow);
        write_to_buffer(device, queue, &mut self.dynamic_buffers.light_buf, &instances_light);

        self.dynamic_buffers.color_draw_calls = draw_calls_color;
        self.dynamic_buffers.shadow_draw_calls = draw_calls_shadow;
        self.dynamic_buffers.light_draw_calls = draw_calls_light;
    }

    fn update_buffers_wasm(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        texture_assets: &TextureAssets,
        gfx_sprite_data: &GfxSpriteData,
    ) {
        // Remove chunks that are no longer being rendered
        let chunks_to_remove: Vec<_> = self
            .chunk_buffers
            .keys()
            .filter(|&chunk_location| !gfx_sprite_data.chunked_gfx.contains_key(chunk_location))
            .copied()
            .collect();

        for chunk_location in chunks_to_remove {
            self.free_buffers
                .push_back(self.chunk_buffers.remove(&chunk_location).unwrap());
        }

        // Update chunk buffers that received new data
        for (chunk_location, chunk_data) in &gfx_sprite_data.chunked_gfx {
            if let Some(chunk_data) = chunk_data {
                let (
                    (instances_color, draw_calls_color),
                    (instances_shadow, draw_calls_shadow),
                    (instances_light, draw_calls_light),
                ) = texture_assets.refs_to_draw_calls_wasm(&chunk_data);

                let buffers = self.chunk_buffers.entry(*chunk_location).or_insert_with(|| {
                    self.free_buffers.pop_front().unwrap_or_else(|| Buffers {
                        color_buf: device.create_buffer(&wgpu::BufferDescriptor {
                            label: Some("dynamic_color_buffer"),
                            size: 128 * size_of::<InstanceSprite>() as u64,
                            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                            mapped_at_creation: false,
                        }),
                        shadow_buf: device.create_buffer(&wgpu::BufferDescriptor {
                            label: Some("dynamic_shadow_buffer"),
                            size: 128 * size_of::<InstanceSprite>() as u64,
                            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                            mapped_at_creation: false,
                        }),
                        light_buf: device.create_buffer(&wgpu::BufferDescriptor {
                            label: Some("dynamic_light_buffer"),
                            size: 128 * size_of::<InstanceLight>() as u64,
                            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                            mapped_at_creation: false,
                        }),
                        color_draw_calls: Vec::new(),
                        shadow_draw_calls: Vec::new(),
                        light_draw_calls: Vec::new(),
                        color_draw_calls_wasm: Vec::new(),
                        shadow_draw_calls_wasm: Vec::new(),
                        light_draw_calls_wasm: Vec::new(),
                    })
                });

                write_to_buffer(device, queue, &mut buffers.color_buf, &instances_color);
                write_to_buffer(device, queue, &mut buffers.shadow_buf, &instances_shadow);
                write_to_buffer(device, queue, &mut buffers.light_buf, &instances_light);

                buffers.color_draw_calls_wasm = draw_calls_color;
                buffers.shadow_draw_calls_wasm = draw_calls_shadow;
                buffers.light_draw_calls_wasm = draw_calls_light;
            }
        }

        // Update dynamic buffers
        let (
            (instances_color, draw_calls_color),
            (instances_shadow, draw_calls_shadow),
            (instances_light, draw_calls_light),
        ) = texture_assets.refs_to_draw_calls_wasm(&gfx_sprite_data.dynamic_gfx);

        write_to_buffer(device, queue, &mut self.dynamic_buffers.color_buf, &instances_color);
        write_to_buffer(device, queue, &mut self.dynamic_buffers.shadow_buf, &instances_shadow);
        write_to_buffer(device, queue, &mut self.dynamic_buffers.light_buf, &instances_light);

        self.dynamic_buffers.color_draw_calls_wasm = draw_calls_color;
        self.dynamic_buffers.shadow_draw_calls_wasm = draw_calls_shadow;
        self.dynamic_buffers.light_draw_calls_wasm = draw_calls_light;
    }
}

struct Buffers {
    color_buf: wgpu::Buffer,
    shadow_buf: wgpu::Buffer,
    light_buf: wgpu::Buffer,

    color_draw_calls: Vec<DrawCall>,
    shadow_draw_calls: Vec<DrawCall>,
    light_draw_calls: Vec<DrawCall>,

    color_draw_calls_wasm: Vec<DrawCallWasm>,
    shadow_draw_calls_wasm: Vec<DrawCallWasm>,
    light_draw_calls_wasm: Vec<DrawCallWasm>,
}
