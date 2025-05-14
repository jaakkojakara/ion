use wgpu::util::DeviceExt;
use winit::dpi::PhysicalSize;

use derive_engine::RawData;

use crate::core::GfxConstants;
use crate::gfx::gfx_config::{GfxConfig, Resolution};
use crate::gfx::{GfxFrameMode, GfxGlobalData};
use crate::util::casting::{RawData, any_as_bytes, slice_as_bytes};

pub(crate) struct RenderGlobals {
    pub(crate) globals_bind_group_layout: wgpu::BindGroupLayout,
    pub(crate) globals_bind_group: wgpu::BindGroup,
    pub(crate) globals_buffer: wgpu::Buffer,

    frame: u32,
    frame_mode: GfxFrameMode,
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

    post_bloom: f32,
}

impl RenderGlobals {
    pub(crate) fn new(
        constants: &GfxConstants,
        device: &wgpu::Device,
        window_size: PhysicalSize<u32>,
        tex_sheet_size: u32,
    ) -> Self {
        let globals_bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
            label: Some("globals_bind_group_layout"),
        });
        let globals_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("globals_buffer"),
            contents: slice_as_bytes(&[0; 16]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });
        let globals_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &globals_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: globals_buffer.as_entire_binding(),
            }],
            label: Some("globals_bind_group"),
        });

        let default_config = GfxConfig::default();

        Self {
            globals_bind_group_layout,
            globals_bind_group,
            globals_buffer,

            frame: 0,
            frame_mode: GfxFrameMode::Normal,
            frame_res_x: default_config.frame_resolution.width,
            frame_res_y: default_config.frame_resolution.height,
            window_res_x: window_size.width,
            window_res_y: window_size.height,

            tex_sheet_size: tex_sheet_size as f32,
            pixels_per_unit: constants.pixels_per_unit,
            height_units_total: constants.height_units_total,
            height_scaled_zero: constants.height_scaled_zero,

            lighting_ambient: 0.5,
            lighting_sun: 0.2,

            post_bloom: 0.1,
        }
    }

    #[allow(dead_code)]
    pub(crate) fn frame_pixel_count(&self) -> u32 {
        self.frame_res_x * self.frame_res_y
    }

    #[allow(dead_code)]
    pub(crate) fn frame_resolution(&self) -> Resolution {
        Resolution {
            width: self.frame_res_x,
            height: self.frame_res_y,
        }
    }

    pub(crate) fn resize_window(&mut self, new_res: Resolution) {
        self.window_res_x = new_res.width;
        self.window_res_y = new_res.height;
    }

    pub(crate) fn resize_renderer(&mut self, new_render_size: Resolution) {
        self.frame_res_x = new_render_size.width;
        self.frame_res_y = new_render_size.height;
    }

    pub(crate) fn update_globals(&mut self, global_data: &GfxGlobalData) {
        self.frame = (global_data.frame % u32::MAX as u64) as u32;

        self.lighting_ambient = global_data.lighting_ambient;
        self.lighting_sun = global_data.lighting_sun;
        self.post_bloom = global_data.post_bloom;
    }

    pub(crate) fn set_frame_mode(&mut self, frame_mode: GfxFrameMode) {
        self.frame_mode = frame_mode;
    }

    pub(crate) fn write_to_gpu(&self, queue: &wgpu::Queue) {
        let gpu_globals = GpuGlobals {
            frame: self.frame,
            frame_mode: self.frame_mode as u32,
            frame_res_x: self.frame_res_x,
            frame_res_y: self.frame_res_y,
            window_res_x: self.window_res_x,
            window_res_y: self.window_res_y,
            tex_sheet_size: self.tex_sheet_size,
            pixels_per_unit: self.pixels_per_unit,
            height_units_total: self.height_units_total,
            height_scaled_zero: self.height_scaled_zero,
            lighting_ambient: self.lighting_ambient,
            lighting_sun: self.lighting_sun,
            lighting_unused: 0.0,
            post_bloom: self.post_bloom,
            padding: [0.0; 2],
        };

        queue.write_buffer(&self.globals_buffer, 0, any_as_bytes(&gpu_globals));
    }
}

#[repr(C)]
#[derive(Debug, Clone, Copy, RawData)]
struct GpuGlobals {
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

    padding: [f32; 2],
}
