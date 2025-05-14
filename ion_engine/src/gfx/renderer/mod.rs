use std::{iter, sync::Arc};

use ion_common::log_info;
use render_camera::RenderCamera;
use render_globals::RenderGlobals;
use render_graph::RenderGraph;

use winit::{
    dpi::{PhysicalPosition, PhysicalSize},
    event::WindowEvent,
    event_loop::ActiveEventLoop,
    window::Fullscreen,
};

#[cfg(target_os = "macos")]
use winit::platform::macos::WindowExtMacOS;

use crate::{
    core::Constants,
    gfx::{GfxFrameMode, WASM_COMPATIBLE_RENDERING, renderer::render_ui::RenderUi},
    util::concurrency::block_on,
};

use super::{
    GfxFrameData,
    gfx_config::{GfxConfig, Resolution, VsyncOpts, WindowMode},
    textures::{texture_assets::TextureAssets, texture_loader::TextureLoader},
};

pub(crate) mod gpu_data_types;
pub(crate) mod render_camera;
pub(crate) mod render_globals;
pub(crate) mod render_graph;
pub(crate) mod render_helpers;
pub(crate) mod render_ui;

pub struct Renderer {
    pub(crate) window: Arc<winit::window::Window>,

    #[allow(dead_code)]
    pub(crate) adapter: wgpu::Adapter,
    pub(crate) surface: wgpu::Surface<'static>,
    pub(crate) device: wgpu::Device,
    pub(crate) queue: wgpu::Queue,

    surface_ready: bool,
    surface_config: wgpu::SurfaceConfiguration,
    surface_capabilities: wgpu::SurfaceCapabilities,
    surface_view_descriptor: wgpu::TextureViewDescriptor<'static>,
    command_encoder_descriptor: wgpu::CommandEncoderDescriptor<'static>,

    config: GfxConfig,
    constants: Constants,
    texture_loader: Option<TextureLoader>,
    texture_assets: Option<TextureAssets>,

    render_camera: RenderCamera,
    render_globals: RenderGlobals,
    render_graph: RenderGraph,
    render_ui: RenderUi,
}

impl Renderer {
    pub fn new(constants: &Constants, window: winit::window::Window, event_loop: &ActiveEventLoop) -> Self {
        let window = Arc::new(window);
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
            #[cfg(not(target_arch = "wasm32"))]
            backends: wgpu::Backends::PRIMARY,
            #[cfg(target_arch = "wasm32")]
            backends: wgpu::Backends::GL,
            ..Default::default()
        });

        let surface = instance.create_surface(window.clone()).unwrap();

        let adapter = block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            compatible_surface: Some(&surface),
            force_fallback_adapter: false,
        }))
        .unwrap();

        let features = if cfg!(target_arch = "wasm32") {
            wgpu::Features::default()
        } else {
            wgpu::Features::TEXTURE_BINDING_ARRAY
                | wgpu::Features::SAMPLED_TEXTURE_AND_STORAGE_BUFFER_ARRAY_NON_UNIFORM_INDEXING
                | wgpu::Features::POLYGON_MODE_LINE
                | wgpu::Features::RG11B10UFLOAT_RENDERABLE
        };

        let limits = if cfg!(target_arch = "wasm32") {
            wgpu::Limits {
                max_texture_dimension_2d: adapter.limits().max_texture_dimension_2d,
                ..wgpu::Limits::downlevel_webgl2_defaults()
            }
        } else {
            wgpu::Limits {
                max_texture_dimension_2d: adapter.limits().max_texture_dimension_2d,
                max_binding_array_elements_per_shader_stage: 2048,
                ..wgpu::Limits::default()
            }
        };

        let (device, queue) = block_on(adapter.request_device(&wgpu::DeviceDescriptor {
            label: Some("gpu_device"),
            required_features: features,
            required_limits: limits,
            memory_hints: wgpu::MemoryHints::Performance,
            trace: wgpu::Trace::Off,
        }))
        .unwrap();

        let surface_capabilities = surface.get_capabilities(&adapter);

        let surface_config = wgpu::SurfaceConfiguration {
            alpha_mode: wgpu::CompositeAlphaMode::Opaque,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_capabilities.formats[0],
            view_formats: vec![surface_capabilities.formats[0]],
            width: window.inner_size().width,
            height: window.inner_size().height,
            present_mode: surface_capabilities.present_modes[0],
            desired_maximum_frame_latency: 1,
        };

        let render_globals = RenderGlobals::new(
            &constants.gfx,
            &device,
            window.inner_size(),
            Self::maximum_texture_size(&device),
        );
        let render_camera = RenderCamera::new(
            &device,
            window.inner_size().into(),
            window.scale_factor() as f32,
            &constants.gfx,
        );

        let render_graph = RenderGraph::new(&constants.gfx, &device);
        let render_ui = RenderUi::new(
            event_loop,
            &window,
            &device,
            &surface_config,
            window.inner_size(),
            window.scale_factor() as f32,
        );

        Self {
            window,
            adapter,
            surface,
            device,
            queue,
            surface_ready: false,
            surface_config,
            surface_capabilities,
            surface_view_descriptor: wgpu::TextureViewDescriptor::default(),
            command_encoder_descriptor: wgpu::CommandEncoderDescriptor::default(),
            constants: constants.clone(),
            config: GfxConfig::default(),
            texture_loader: None,
            texture_assets: None,
            render_camera,
            render_globals,
            render_graph,
            render_ui,
        }
    }

    // ---------------------------------------------------------- //
    // ------------------- Public interface --------------------- //
    // ---------------------------------------------------------- //

    pub fn load_texture_assets(&mut self, texture_assets: TextureAssets) {
        assert!(
            self.texture_loader.is_none(),
            "Only one texture load can be in progress at a time"
        );

        let texture_loader = TextureLoader::new(
            &self.constants,
            texture_assets.required_textures(),
            Self::maximum_texture_size(&self.device),
        );
        self.texture_loader = Some(texture_loader);
        self.texture_assets = Some(texture_assets);
    }

    pub fn texture_assets_progress(&self) -> Option<f32> {
        self.texture_loader.as_ref().map(|loader| loader.progress())
    }

    pub fn texture_assets_ready(&self) -> Option<bool> {
        self.texture_assets.as_ref().map(|assets| assets.assets_ready())
    }

    pub fn available_vsync_modes(&self) -> Vec<VsyncOpts> {
        self.surface_capabilities
            .present_modes
            .iter()
            .map(|present_mode| (*present_mode).into())
            .collect()
    }

    pub fn dpi_factor(&self) -> f32 {
        self.window.scale_factor() as f32
    }

    pub fn window_resolution(&self) -> Resolution {
        self.window.inner_size().into()
    }

    pub fn monitor_resolution(&self) -> Resolution {
        self.monitor_handle().size().into()
    }

    pub fn config(&self) -> &GfxConfig {
        &self.config
    }

    pub fn set_config(&mut self, mut config: GfxConfig) {
        #[cfg(not(target_arch = "wasm32"))]
        self.window.set_decorations(config.window_decorations);
        #[cfg(not(target_arch = "wasm32"))]
        self.window.set_transparent(config.window_transparent);

        self.surface_config.present_mode = config.vsync.into();
        self.surface.configure(&self.device, &self.surface_config);

        if config.vsync != VsyncOpts::Off {
            #[cfg(not(target_arch = "wasm32"))]
            {
                let monitor_refresh_rate = self.monitor_handle().refresh_rate_millihertz().unwrap() / 1000;
                config.frame_rate_cap = Some(
                    config
                        .frame_rate_cap
                        .unwrap_or(monitor_refresh_rate)
                        .min(monitor_refresh_rate),
                );
            }

            #[cfg(target_arch = "wasm32")]
            assert!(config.frame_rate_cap.is_some(), "Frame rate cap must be set on wasm");
        }

        #[cfg(not(target_arch = "wasm32"))]
        match config.window_mode {
            WindowMode::Windowed => {
                #[cfg(target_os = "macos")]
                self.window.set_simple_fullscreen(false);

                self.window.set_fullscreen(None);
                let physical_size: PhysicalSize<u32> = config.frame_resolution.into();
                if let Some(new_size) = self.window.request_inner_size(physical_size) {
                    self.resize_window(new_size.into(), None)
                }
            }
            WindowMode::BorderlessFullscreen => {
                #[cfg(target_os = "macos")]
                self.window.set_simple_fullscreen(false);

                self.window.set_fullscreen(Some(Fullscreen::Borderless(None)));
            }
            WindowMode::ExclusiveFullscreen(fullscreen_video_mode) => {
                #[cfg(target_os = "macos")]
                panic!(
                    "Exclusive fullscreen not supported on macos. Mode attempted: {:?}",
                    fullscreen_video_mode
                );

                #[cfg(not(target_os = "macos"))]
                {
                    let video_mode = self
                        .monitor_handle()
                        .video_modes()
                        .find(|video_mode| {
                            video_mode.size().width == fullscreen_video_mode.width
                                && video_mode.size().height == fullscreen_video_mode.height
                                && video_mode.refresh_rate_millihertz() == fullscreen_video_mode.frame_rate * 1000
                        })
                        .expect("Should find a video mode matching given render_config");
                    self.window
                        .set_fullscreen(Some(Fullscreen::Exclusive(video_mode.clone())));
                }
            }
        }

        #[cfg(not(target_arch = "wasm32"))]
        if config.window_mode == WindowMode::Windowed {
            self.set_window_position_center();
        }

        self.resize_renderer(config.frame_resolution);
        self.config = config;
    }

    /// Sets the canvas size on wasm.
    /// This is the size of the canvas on the web page in pixels.
    /// Notably, this is not the same as the rendering resolution.
    /// Does nothing on native targets
    pub fn set_wasm_window_size(&mut self, _resolution: Resolution) {
        #[cfg(target_arch = "wasm32")]
        let _ = self.window.request_inner_size(PhysicalSize::from(_resolution));
    }

    pub fn set_window_position_center(&mut self) {
        let own_size = self.window.outer_size();
        let monitor_size = self.monitor_handle().size();

        let target_x = (monitor_size.width.saturating_sub(own_size.width)) / 2;
        let target_y = (monitor_size.height.saturating_sub(own_size.height)) / 2;

        self.window.set_outer_position(PhysicalPosition {
            x: target_x,
            y: target_y,
        });
    }

    pub fn set_frame_mode(&mut self, frame_mode: GfxFrameMode) {
        self.render_globals.set_frame_mode(frame_mode);
    }

    // ---------------------------------------------------------- //
    // ----------------- Internal implementation ---------------- //
    // ---------------------------------------------------------- //

    pub(crate) fn resize_window(&mut self, new_window_res: Resolution, new_dpi_factor: Option<f32>) {
        if new_window_res.width > 0 && new_window_res.height > 0 {
            log_info!("resize window: {:?}", new_window_res);
            self.surface_config.width = new_window_res.width;
            self.surface_config.height = new_window_res.height;

            self.surface.configure(&self.device, &self.surface_config);
            self.surface_ready = true;

            self.render_camera.resize_window(new_window_res, new_dpi_factor);
            self.render_globals.resize_window(new_window_res);
            self.render_ui.resize_window(new_window_res, new_dpi_factor);
        }
    }

    pub(crate) fn resize_renderer(&mut self, new_render_size: Resolution) {
        if new_render_size.width > 0 && new_render_size.height > 0 {
            log_info!("resize renderer: {:?}", new_render_size);
            self.render_globals.resize_renderer(new_render_size);
            self.render_graph.create_render_targets(&self.device, new_render_size);
        }
    }

    pub(crate) fn camera(&self) -> &RenderCamera {
        &self.render_camera
    }

    pub(crate) fn ui_begin_pass(&mut self) -> egui::Context {
        self.render_ui.ui_begin_pass(&self.window)
    }

    pub(crate) fn ui_event_process(&mut self, event: &WindowEvent) -> bool {
        self.render_ui.ui_event_process(&self.window, event)
    }

    fn monitor_handle(&self) -> winit::monitor::MonitorHandle {
        self.window
            .current_monitor()
            .expect("Current monitor must be available")
    }

    fn maximum_texture_size(device: &wgpu::Device) -> u32 {
        if WASM_COMPATIBLE_RENDERING {
            device.limits().max_texture_dimension_2d.min(8192)
        } else {
            device.limits().max_texture_dimension_2d
        }
    }

    // ---------------------------------------------------------- //
    // ------------------ Rendering functions ------------------- //
    // ---------------------------------------------------------- //

    /// If frame data is provided, it will be used to update the camera and globals.
    /// If texture loading is in progress, it will be polled.
    pub(crate) fn pre_render(&mut self, frame_data: Option<&GfxFrameData>) {
        if let Some(frame_data) = frame_data {
            self.render_camera.update_location(&frame_data);
            self.render_camera.update_scale(frame_data.global_data.camera_scale);
            self.render_globals.update_globals(&frame_data.global_data);

            self.render_ui.ui_build_debug_labels(
                &frame_data.debug_data.debug_labels,
                &self.render_camera,
                self.window.inner_size(),
                self.dpi_factor(),
            );
        }

        if self.texture_loader.is_some() {
            let poll_complete = self
                .texture_loader
                .as_mut()
                .unwrap()
                .poll_loading(&self.device, &self.queue);

            if poll_complete {
                self.texture_assets
                    .as_mut()
                    .unwrap()
                    .take_finished_loader(&self.device, self.texture_loader.take().unwrap());

                self.render_graph.create_render_passes(
                    &self.device,
                    &self.queue,
                    &self.render_camera,
                    &self.render_globals,
                    &self.texture_assets.as_ref().unwrap(),
                    &self.config,
                    &self.surface_config,
                );

                self.render_graph
                    .create_render_targets(&self.device, self.config.frame_resolution);
            }
        }
    }

    pub(crate) fn render(&mut self, frame_data: Option<&GfxFrameData>) {
        if !self.surface_ready {
            return;
        }

        let surface_texture: wgpu::SurfaceTexture;
        let surface_view: wgpu::TextureView;
        let mut encoder: wgpu::CommandEncoder;

        {
            surface_texture = self.surface.get_current_texture().unwrap();
            surface_view = surface_texture.texture.create_view(&self.surface_view_descriptor);
            encoder = self.device.create_command_encoder(&self.command_encoder_descriptor);
        }

        self.render_globals.write_to_gpu(&self.queue);
        self.render_camera.write_to_gpu(&self.queue);

        if let Some(frame_data) = &frame_data {
            debug_assert!(
                self.render_graph.render_graph_ready(),
                "Render graph must be ready before rendering. "
            );

            if self.texture_assets.as_ref().map(|a| a.assets_ready()).unwrap_or(false) {
                self.render_graph.execute_render_graph(
                    &mut encoder,
                    &self.device,
                    &self.queue,
                    &surface_view,
                    &self.render_camera,
                    &self.render_globals,
                    &self.texture_assets.as_ref().unwrap(),
                    &frame_data,
                );
            }
        }

        self.render_ui
            .render_ui(&self.window, &self.device, &self.queue, &mut encoder, &surface_view);

        self.queue.submit(iter::once(encoder.finish()));

        surface_texture.present();
    }

    pub(crate) fn post_render(&mut self) {
        self.render_ui.render_ui_cleanup();
    }
}
