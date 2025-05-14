use egui::{ViewportId, ViewportInfo};
use egui_wgpu::ScreenDescriptor;
use egui_winit::update_viewport_info;
use wgpu::CommandEncoder;
use winit::dpi::PhysicalSize;
use winit::event::WindowEvent;
use winit::event_loop::ActiveEventLoop;

use crate::core::coordinates::Location;
use crate::gfx::gfx_config::Resolution;

use super::render_camera::RenderCamera;

// ---------------------------------------------------------- //
// ---------------------- Ui Renderer ----------------------- //
// ---------------------------------------------------------- //

pub(super) struct RenderUi {
    winit_state: egui_winit::State,
    wgpu_renderer: egui_wgpu::Renderer,
    viewport_info: ViewportInfo,
    screen_descriptor: ScreenDescriptor,
    latest_texture_delta: egui::TexturesDelta,
}

impl RenderUi {
    pub(super) fn new(
        event_loop: &ActiveEventLoop,
        window: &winit::window::Window,
        device: &wgpu::Device,
        surface_config: &wgpu::SurfaceConfiguration,
        ui_window_size: PhysicalSize<u32>,
        ui_dpi_factor: f32,
    ) -> Self {
        let ctx = egui::Context::default();
        let state = egui_winit::State::new(
            ctx,
            ViewportId::ROOT,
            event_loop,
            Some(ui_dpi_factor),
            event_loop.system_theme(),
            Some(device.limits().max_texture_dimension_2d as usize),
        );
        let renderer = egui_wgpu::Renderer::new(device, surface_config.format, None, 1, false);
        let screen_descriptor = ScreenDescriptor {
            size_in_pixels: [ui_window_size.width, ui_window_size.height],
            pixels_per_point: ui_dpi_factor,
        };

        let mut viewport_info = ViewportInfo::default();
        update_viewport_info(&mut viewport_info, state.egui_ctx(), window, true);

        Self {
            winit_state: state,
            wgpu_renderer: renderer,
            viewport_info,
            screen_descriptor,
            latest_texture_delta: egui::TexturesDelta::default(),
        }
    }

    pub(super) fn resize_window(&mut self, new_window_res: Resolution, new_screen_dpi: Option<f32>) {
        self.screen_descriptor = ScreenDescriptor {
            size_in_pixels: [new_window_res.width, new_window_res.height],
            pixels_per_point: new_screen_dpi.unwrap_or(self.screen_descriptor.pixels_per_point),
        };
    }

    pub(crate) fn ui_event_process(&mut self, native_window: &winit::window::Window, event: &WindowEvent) -> bool {
        self.winit_state.on_window_event(native_window, event).consumed
    }

    pub(crate) fn ui_begin_pass(&mut self, native_window: &winit::window::Window) -> egui::Context {
        let context = self.winit_state.egui_ctx().clone();

        update_viewport_info(&mut self.viewport_info, &context, native_window, false);

        let data = self.winit_state.take_egui_input(native_window);

        context.begin_pass(data);
        context
    }

    pub(super) fn ui_build_debug_labels(
        &mut self,
        labels: &[(String, Location)],
        render_camera: &RenderCamera,
        window_res: PhysicalSize<u32>,
        dpi_scale: f32,
    ) {
        let ctx = self.winit_state.egui_ctx();
        let prev_style = ctx.style();

        ctx.set_style(self.debug_label_style());

        for (i, (text, loc)) in labels.iter().enumerate() {
            let pos = render_camera.loc_to_pos(*loc);
            let pos_x = pos.x * window_res.width as f32 / dpi_scale;
            let pos_y = pos.y * window_res.height as f32 / dpi_scale;

            if pos.is_visible() {
                egui::Window::new(format!("debug_label_{}", i))
                    .resizable(false)
                    .title_bar(false)
                    .fixed_pos([pos_x, pos_y])
                    .collapsible(false)
                    .show(&ctx, |ui| {
                        ui.label(egui::RichText::new(text).color(egui::Color32::WHITE).size(9.0));
                    });
            }
        }

        ctx.set_style(prev_style);
    }

    #[allow(clippy::too_many_arguments)]
    pub(super) fn render_ui(
        &mut self,
        native_window: &winit::window::Window,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        encoder: &mut CommandEncoder,
        surface_view: &wgpu::TextureView,
    ) {
        let mut render_pass = encoder
            .begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("render_pass_egui"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: surface_view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    },
                })],
                ..Default::default()
            })
            .forget_lifetime();

        let context = self.winit_state.egui_ctx().clone();
        let egui::FullOutput {
            platform_output,
            textures_delta,
            shapes,
            pixels_per_point,
            ..
        } = context.end_pass();
        self.winit_state.handle_platform_output(native_window, platform_output);
        let primitives = self.winit_state.egui_ctx().tessellate(shapes, pixels_per_point);
        for (id, image_delta) in &textures_delta.set {
            self.wgpu_renderer.update_texture(device, queue, *id, image_delta);
        }
        self.wgpu_renderer
            .update_buffers(device, queue, encoder, primitives.as_slice(), &self.screen_descriptor);
        self.wgpu_renderer
            .render(&mut render_pass, primitives.as_slice(), &self.screen_descriptor);

        self.latest_texture_delta = textures_delta;
    }

    pub(super) fn render_ui_cleanup(&mut self) {
        for id in &self.latest_texture_delta.free {
            self.wgpu_renderer.free_texture(id);
        }
    }

    fn debug_label_style(&self) -> egui::Style {
        egui::Style {
            spacing: egui::Spacing {
                item_spacing: egui::vec2(0.0, 0.0),
                ..Default::default()
            },
            visuals: egui::Visuals {
                window_stroke: egui::Stroke {
                    width: 0.0,
                    color: egui::Color32::TRANSPARENT,
                },
                window_fill: egui::Color32::TRANSPARENT,
                window_shadow: egui::Shadow {
                    color: egui::Color32::TRANSPARENT,
                    ..Default::default()
                },
                ..Default::default()
            },
            ..Default::default()
        }
    }
}
