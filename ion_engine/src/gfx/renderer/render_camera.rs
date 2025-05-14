use wgpu::util::DeviceExt;

use crate::core::GfxConstants;
use crate::core::coordinates::{Location, Position};
use crate::gfx::GfxFrameData;
use crate::gfx::gfx_config::Resolution;
use crate::util::casting::{RawData, any_as_bytes};
use derive_engine::RawData;
use ion_common::math::matrix::Matrix4x4;

pub(crate) struct RenderCamera {
    pub(crate) camera_bind_group_layout: wgpu::BindGroupLayout,
    pub(crate) camera_bind_group: wgpu::BindGroup,
    pub(crate) camera_buffer: wgpu::Buffer,

    render_loc: Location,
    real_loc: Location,
    scale: f32,
    angle_cos: f32,
    angle_sin: f32,
    angle_tan: f32,
    last_real_change_x: f32,
    last_real_change_y: f32,

    scale_mat: Matrix4x4,
    rot_mat: Matrix4x4,

    window_res: Resolution,
    window_aspect_ratio: f32,
    window_dpi: f32,
}

impl RenderCamera {
    pub(crate) fn new(
        device: &wgpu::Device,
        window_res: Resolution,
        window_dpi: f32,
        gfx_constants: &GfxConstants,
    ) -> Self {
        let camera_bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
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
            label: Some("camera_bind_group_layout"),
        });
        let camera_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("camera_buffer"),
            contents: any_as_bytes(&GpuCamera {
                vp_mat: [[0.0_f32; 4]; 4],
                vp_mat_inv: [[0.0_f32; 4]; 4],
                z_edges: [0.0, 0.0],
                loc: [0.0, 0.0],
                scale: 1.0,
                angle_cos: 0.0,
                angle_sin: 0.0,
                angle_tan: 0.0,
            }),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });
        let camera_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &camera_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: camera_buffer.as_entire_binding(),
            }],
            label: Some("camera_bind_group"),
        });

        let angle_cos = Self::calc_camera_x_y_ratio(gfx_constants);
        let angle_sin = gfx_constants.camera_angle_deg.to_radians().sin();
        let angle_tan = gfx_constants.camera_angle_deg.to_radians().tan();

        Self {
            camera_bind_group_layout,
            camera_bind_group,
            camera_buffer,
            render_loc: Location { x: 0.0, y: 0.0 },
            real_loc: Location { x: 0.0, y: 0.0 },
            scale: 10.0,
            angle_cos,
            angle_sin,
            angle_tan,
            last_real_change_x: 0.0,
            last_real_change_y: 0.0,

            rot_mat: Matrix4x4::new([
                [std::f32::consts::FRAC_PI_4.cos(), std::f32::consts::FRAC_PI_4.sin(), 0.0, 0.0],
                [-std::f32::consts::FRAC_PI_4.sin(), std::f32::consts::FRAC_PI_4.cos(), 0.0, 0.0],
                [0.0, 0.0, 1.0, 0.0],
                [0.0, 0.0, 0.0, 1.0],
            ]), // 45 degrees
            scale_mat: Matrix4x4::new([
                [1.0, 0.0, 0.0, 0.0],
                [0.0, angle_cos, 0.0, 0.0],
                [0.0, 0.0, 1.0, 0.0],
                [0.0, 0.0, 0.0, 1.0],
            ]),

            window_aspect_ratio: window_res.width as f32 / window_res.height as f32,
            window_dpi,
            window_res,
        }
    }

    pub(crate) fn update_location(&mut self, frame_data: &GfxFrameData) {
        let camera_loc = frame_data.global_data.camera_loc;
        let render_frame_offset = frame_data.timing_data.render_frame_offset;
        if frame_data.timing_data.render_data_use_count == 0 {
            self.last_real_change_x = camera_loc.x - self.real_loc.x;
            self.last_real_change_y = camera_loc.y - self.real_loc.y;
            self.real_loc = camera_loc;
        }

        let new_updated_loc = Location {
            x: camera_loc.x + self.last_real_change_x * render_frame_offset,
            y: camera_loc.y + self.last_real_change_y * render_frame_offset,
        };

        self.render_loc = new_updated_loc;
    }

    pub(crate) fn update_scale(&mut self, camera_scale: f32) {
        self.scale = camera_scale;
    }

    pub(crate) fn write_to_gpu(&self, queue: &wgpu::Queue) {
        let z_span = self.angle_sin * self.scale * self.window_aspect_ratio;
        let z_edges = [16.0, 16.0 + z_span];
        let gpu_camera = GpuCamera {
            vp_mat: self.calc_vp_mat(None).raw(),
            vp_mat_inv: self.calc_vp_mat(None).inverse().unwrap().raw(),
            z_edges,
            loc: [self.render_loc.x, self.render_loc.y],
            scale: self.scale,
            angle_cos: self.angle_cos,
            angle_sin: self.angle_sin,
            angle_tan: self.angle_tan,
        };
        queue.write_buffer(&self.camera_buffer, 0, any_as_bytes(&gpu_camera));
    }

    pub(crate) fn interpolation_x(&self) -> f32 {
        self.render_loc.x - self.real_loc.x
    }

    pub(crate) fn interpolation_y(&self) -> f32 {
        self.render_loc.y - self.real_loc.y
    }

    pub(crate) fn last_real_change_x(&self) -> f32 {
        self.last_real_change_x
    }
    pub(crate) fn last_real_change_y(&self) -> f32 {
        self.last_real_change_y
    }

    #[allow(dead_code)]
    pub(crate) fn pos_to_loc(&self, position: Position) -> Location {
        let cursor_loc_x = (position.x - 0.5) * 2.0;
        let cursor_loc_y = (0.5 - position.y) * 2.0;
        let cursor_loc_vec = [cursor_loc_x, cursor_loc_y, 0.0, 1.0];

        let vp_mat_inverse = self
            .calc_vp_mat(None)
            .inverse()
            .expect("view_projection matrix should always be invertible");

        let world_loc = vp_mat_inverse * cursor_loc_vec;

        Location {
            x: world_loc[0],
            y: world_loc[1],
        }
    }

    #[allow(dead_code)]
    pub(crate) fn loc_to_pos(&self, location: Location) -> Position {
        let world_loc_vec = [location.x, location.y, 0.0, 1.0];
        let vp_mat = self.calc_vp_mat(None);
        let clip_space = vp_mat * world_loc_vec;

        let ndc_x = clip_space[0] / clip_space[3];
        let ndc_y = clip_space[1] / clip_space[3];

        Position {
            x: (ndc_x + 1.0) * 0.5,
            y: (1.0 - ndc_y) * 0.5,
        }
    }

    pub(crate) fn calc_vp_mat(&self, custom_scale: Option<f32>) -> Matrix4x4 {
        let scale = custom_scale.unwrap_or(self.scale) / self.window_aspect_ratio;

        let left = scale * -1. * self.window_aspect_ratio;
        let right = scale * self.window_aspect_ratio;
        let bottom = scale * -1.;
        let top = scale;

        let rcp_width = 1.0 / (right - left);
        let rcp_height = 1.0 / (scale - bottom);
        let r = 1.0 / (0.0 - 1.0);

        let view = Matrix4x4::new([
            [1.0, 0.0, 0.0, 0.0],
            [0.0, 1.0, 0.0, 0.0],
            [0.0, 0.0, 1.0, 0.0],
            [-self.render_loc.x, -self.render_loc.y, 0.0, 1.0],
        ]);

        // Orthographic right-hand projection matrix
        let proj = Matrix4x4::new([
            [rcp_width + rcp_width, 0.0, 0.0, 0.0],
            [0.0, rcp_height + rcp_height, 0.0, 0.0],
            [0.0, 0.0, r, 0.0],
            [-(left + right) * rcp_width, -(top + bottom) * rcp_height, 0.0, 1.0],
        ]);

        proj * self.scale_mat * self.rot_mat * view
    }

    pub(crate) fn resize_window(&mut self, new_window_res: Resolution, new_screen_dpi: Option<f32>) {
        self.window_aspect_ratio = new_window_res.width as f32 / new_window_res.height as f32;
        self.window_res = new_window_res;
        self.window_dpi = new_screen_dpi.unwrap_or(self.window_dpi);
    }

    pub(crate) fn calc_camera_x_y_ratio(gfx_constants: &GfxConstants) -> f32 {
        gfx_constants.camera_angle_deg.to_radians().cos()
    }
}

#[repr(C)]
#[derive(Debug, Clone, Copy, RawData)]
struct GpuCamera {
    vp_mat: [[f32; 4]; 4],
    vp_mat_inv: [[f32; 4]; 4],
    z_edges: [f32; 2],
    loc: [f32; 2],
    scale: f32,
    angle_cos: f32,
    angle_sin: f32,
    angle_tan: f32,
}
