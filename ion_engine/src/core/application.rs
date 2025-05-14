use core::panic;
use std::sync::mpsc::Sender;

use ion_common::log_info;
use winit::{application::ApplicationHandler, event::WindowEvent, event_loop::EventLoop};

use crate::{
    core::{coordinates::Position, world::CommandType},
    gfx::renderer::Renderer,
    input::Input,
    util,
};

use super::Constants;

pub enum ApplicationEvent {
    /// Emitted when the window close button is pressed by the user.
    /// Only on native. On Web, the `CloseRequested` event is never emitted.
    CloseRequested,

    /// Emitted when the window gains focus.
    /// On native, this is emitted when this app window becomes the active window.
    /// On Web, this is emitted when the canvas element gains focus.
    FocusGained,

    /// Emitted when the window loses focus.
    /// On native, this is emitted when some other window becomes the active window.
    /// On Web, this is emitted when the canvas element loses focus.
    FocusLost,

    /// On native, the `Resumed` event should never be emitted.
    /// On Web, the `Resumed` event is emitted in response to a [`pageshow`] event
    /// with the property [`persisted`] being true, which means that the page is being
    /// restored from the [`bfcache`] (back/forward cache) - an in-memory cache that
    /// stores a complete snapshot of a page (including the JavaScript heap) as the
    /// user is navigating away.
    ///
    /// [`pageshow`]: https://developer.mozilla.org/en-US/docs/Web/API/Window/pageshow_event
    /// [`persisted`]: https://developer.mozilla.org/en-US/docs/Web/API/PageTransitionEvent/persisted
    /// [`bfcache`]: https://web.dev/bfcache/
    Resumed,

    /// On native, the `Suspended` event should never be emitted.
    /// On Web, the `Suspended` event is emitted in response to a [`pagehide`] event
    /// with the property [`persisted`] being true, which means that the page is being
    /// put in the [`bfcache`] (back/forward cache) - an in-memory cache that stores a
    /// complete snapshot of a page (including the JavaScript heap) as the user is
    /// navigating away.
    ///
    /// [`pagehide`]: https://developer.mozilla.org/en-US/docs/Web/API/Window/pagehide_event
    /// [`persisted`]: https://developer.mozilla.org/en-US/docs/Web/API/PageTransitionEvent/persisted
    /// [`bfcache`]: https://web.dev/bfcache/
    Suspended,
}

pub(crate) fn run_render_loop<F, C>(
    constants: Constants,
    input: Input<C>,
    app_event_sender: Sender<ApplicationEvent>,
    on_render_frame: F,
) where
    F: FnMut(&mut Renderer) -> bool + 'static,
    C: CommandType,
{
    util::init_os();

    log_info!("Starting up {}", constants.app_name);

    let event_loop = EventLoop::new().expect("Event loop creation must succeed");
    let mut app_handle = AppHandle {
        constants,
        renderer: None,
        input,
        app_event_sender,
        on_render_frame,
    };

    #[cfg(not(target_arch = "wasm32"))]
    {
        let run_result = event_loop.run_app(&mut app_handle);

        log_info!("Shutting down {}", app_handle.constants.app_name);

        ion_common::flush_logs();

        util::uninit_os();

        match run_result {
            Ok(_) => std::process::exit(0),
            Err(_) => std::process::exit(1),
        }
    }

    #[cfg(target_arch = "wasm32")]
    {
        use winit::platform::web::EventLoopExtWebSys;

        event_loop.spawn_app(app_handle);

        util::uninit_os();
    };
}

pub(crate) struct AppHandle<F, C>
where
    F: FnMut(&mut Renderer) -> bool + 'static,
    C: CommandType,
{
    constants: Constants,
    renderer: Option<Renderer>,
    input: Input<C>,
    app_event_sender: Sender<ApplicationEvent>,

    on_render_frame: F,
}

impl<F, C> ApplicationHandler for AppHandle<F, C>
where
    F: FnMut(&mut Renderer) -> bool + 'static,
    C: CommandType,
{
    fn resumed(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
        if self.renderer.is_none() {
            // On all platforms, resume is emitted once at the start of the app.
            // This is when it is safe to create the window and start the renderer.
            // This is not a "real" resume event so it is not sent as SystemEvent.
            let window = event_loop
                .create_window(winit::window::WindowAttributes::default().with_title(self.constants.app_name))
                .expect("Window creation must succeed");

            #[cfg(target_arch = "wasm32")]
            {
                use winit::platform::web::WindowExtWebSys;

                ion_common::web_sys::window()
                    .and_then(|window| window.document())
                    .and_then(|document| {
                        let dst = document.get_element_by_id("wasm-container")?;
                        let canvas = ion_common::web_sys::Element::from(window.canvas()?);
                        dst.append_child(&canvas).ok()?;
                        Some(())
                    })
                    .expect("Must succeed in appending canvas to document body.");
            }

            self.renderer = Some(Renderer::new(&self.constants, window, event_loop));
        } else {
            self.app_event_sender.send(ApplicationEvent::Resumed).unwrap();
        }
    }

    fn suspended(&mut self, _event_loop: &winit::event_loop::ActiveEventLoop) {
        self.app_event_sender.send(ApplicationEvent::Suspended).unwrap();
    }

    fn window_event(
        &mut self,
        event_loop: &winit::event_loop::ActiveEventLoop,
        window_id: winit::window::WindowId,
        event: winit::event::WindowEvent,
    ) {
        let renderer = self.renderer.as_mut().unwrap();
        let used_by_ui = renderer.ui_event_process(&event);

        match event {
            WindowEvent::Resized(_) => {
                renderer.resize_window(renderer.window.inner_size().into(), None);
            }
            WindowEvent::ScaleFactorChanged { scale_factor, .. } => {
                renderer.resize_window(renderer.window.inner_size().into(), Some(scale_factor as f32));
            }
            WindowEvent::KeyboardInput { .. } => {
                if !used_by_ui {
                    self.input.handle_keyboard_event(&event);
                }
            }
            WindowEvent::MouseInput { .. } => {
                if !used_by_ui {
                    self.input.handle_mouse_button_event(&event);
                }
            }
            WindowEvent::MouseWheel { .. } => {
                if !used_by_ui {
                    self.input.handle_mouse_scroll_event(&event);
                }
            }
            WindowEvent::CursorMoved { position, .. } => {
                let pos = Position::from_physical_position(position, renderer.window_resolution());
                let loc = renderer.camera().pos_to_loc(pos);
                self.input.handle_mouse_move_event(loc, pos);
            }
            WindowEvent::RedrawRequested => {
                if !(self.on_render_frame)(renderer) {
                    event_loop.exit();
                }

                renderer.window.request_redraw();
            }
            WindowEvent::Focused(focused) => {
                if focused {
                    self.app_event_sender.send(ApplicationEvent::FocusGained).unwrap();
                } else {
                    self.app_event_sender.send(ApplicationEvent::FocusLost).unwrap();
                }
            }
            WindowEvent::CloseRequested => {
                self.app_event_sender.send(ApplicationEvent::CloseRequested).unwrap();
            }
            _ => {}
        }

        if renderer.window.id() != window_id {
            panic!("Window ID mismatch: {:?} != {:?}", renderer.window.id(), window_id);
        }
    }
}
