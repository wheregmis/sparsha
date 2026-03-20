//! Application runner and main event loop.

use spark_core::{init_wgpu, Color, SurfaceState};
use spark_input::{FocusManager, InputEvent, PointerButton};
use spark_layout::LayoutTree;
use spark_render::{DrawList, Renderer};
use spark_text::TextSystem;
use spark_widgets::{EventContext, PaintContext, Widget};
#[cfg(target_arch = "wasm32")]
use std::{cell::RefCell, rc::Rc};
use wgpu::{Device, Queue};
use winit::event::WindowEvent;

#[cfg(not(target_arch = "wasm32"))]
use std::time::Instant;

#[cfg(target_arch = "wasm32")]
use web_time::Instant;

/// Application configuration.
pub struct AppConfig {
    /// Window title.
    pub title: String,
    /// Initial window width.
    pub width: u32,
    /// Initial window height.
    pub height: u32,
    /// Background color.
    pub background: Color,
    /// Enable VSync.
    pub vsync: bool,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            title: String::from("Spark App"),
            width: 800,
            height: 600,
            background: Color::from_hex(0xF3F4F6),
            vsync: true,
        }
    }
}

/// The main application struct.
pub struct App {
    config: AppConfig,
}

impl App {
    /// Create a new app with default configuration.
    pub fn new() -> Self {
        Self {
            config: AppConfig::default(),
        }
    }

    /// Set the window title.
    pub fn with_title(mut self, title: impl Into<String>) -> Self {
        self.config.title = title.into();
        self
    }

    /// Set the initial window size.
    pub fn with_size(mut self, width: u32, height: u32) -> Self {
        self.config.width = width;
        self.config.height = height;
        self
    }

    /// Set the background color.
    pub fn with_background(mut self, color: Color) -> Self {
        self.config.background = color;
        self
    }

    /// Run the application with the given root widget.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn run<F>(self, build_ui: F) -> !
    where
        F: FnOnce() -> Box<dyn Widget> + 'static,
    {
        let event_loop = winit::event_loop::EventLoop::new().unwrap();
        let runner = AppRunner::new(self.config, build_ui);
        let runner_leaked: &'static mut AppRunner<F> = Box::leak(Box::new(runner));
        event_loop.run_app(runner_leaked).unwrap();
        std::process::exit(0);
    }

    /// Run the application with the given root widget.
    ///
    /// On web targets this returns after registering the app with the browser event loop.
    #[cfg(target_arch = "wasm32")]
    pub fn run<F>(self, build_ui: F)
    where
        F: FnOnce() -> Box<dyn Widget> + 'static,
    {
        let event_loop = winit::event_loop::EventLoop::new().unwrap();
        let runner = AppRunner::new(self.config, build_ui);
        let runner_leaked: &'static mut AppRunner<F> = Box::leak(Box::new(runner));
        event_loop.run_app(runner_leaked).unwrap();
    }
}

impl Default for App {
    fn default() -> Self {
        Self::new()
    }
}

/// Internal application runner that handles the event loop.
struct AppRunner<F: FnOnce() -> Box<dyn Widget>> {
    config: AppConfig,
    build_ui: Option<F>,
    state: Option<AppState>,
    #[cfg(target_arch = "wasm32")]
    pending_init: Option<Rc<RefCell<Option<AppState>>>>,
}

struct AppState {
    window: &'static dyn winit::window::Window,
    device: Device,
    queue: Queue,
    surface_state: SurfaceState<'static>,
    renderer: Renderer,
    text_system: TextSystem,
    draw_list: DrawList,
    layout_tree: LayoutTree,
    focus_manager: FocusManager,
    root_widget: Box<dyn Widget>,
    start_time: Instant,
    mouse_pos: glam::Vec2,
    scale_factor: f32,
    needs_layout: bool,
    needs_repaint: bool,
}

impl<F: FnOnce() -> Box<dyn Widget>> AppRunner<F> {
    fn new(config: AppConfig, build_ui: F) -> Self {
        Self {
            config,
            build_ui: Some(build_ui),
            state: None,
            #[cfg(target_arch = "wasm32")]
            pending_init: None,
        }
    }

    fn build_layout(&mut self) {
        let state = self.state.as_mut().unwrap();

        // Clear layout tree
        state.layout_tree = LayoutTree::new();

        // Build layout tree from widget tree
        fn add_to_layout(
            widget: &mut dyn Widget,
            tree: &mut LayoutTree,
            in_scroll: bool,
        ) -> spark_layout::WidgetId {
            let mut style = widget.style();
            if in_scroll {
                style.flex_shrink = 0.0;
            }
            let is_scroll = widget.is_scroll_container();
            let children_ids: Vec<_> = widget
                .children_mut()
                .iter_mut()
                .map(|child| add_to_layout(child.as_mut(), tree, in_scroll || is_scroll))
                .collect();

            let id = if children_ids.is_empty() {
                tree.new_leaf(style)
            } else {
                tree.new_with_children(style, &children_ids)
            };

            widget.set_id(id);
            id
        }

        let root_id = add_to_layout(state.root_widget.as_mut(), &mut state.layout_tree, false);
        state.layout_tree.set_root(root_id);

        // Compute layout
        // Use surface size - this should be in physical pixels
        // But we need to convert to logical pixels for layout
        let size = state.surface_state.size;
        let logical_width = (size.width as f32) / state.scale_factor;
        let logical_height = (size.height as f32) / state.scale_factor;
        state
            .layout_tree
            .compute_layout(logical_width, logical_height);

        state.needs_layout = false;
        state.needs_repaint = true;
    }

    fn paint(&mut self) {
        let state = self.state.as_mut().unwrap();
        state.draw_list.clear();

        // Get elapsed time for animations
        let elapsed_time = state.start_time.elapsed().as_secs_f32();

        // We need to use raw pointers to pass mutable references through the recursive function
        // This is safe because we control the lifetime and don't alias
        let text_system_ptr = &mut state.text_system as *mut TextSystem;
        let device_ptr = &state.device as *const Device;
        let queue_ptr = &state.queue as *const Queue;

        #[allow(clippy::too_many_arguments)]
        fn paint_widget(
            widget: &dyn Widget,
            layout_tree: &LayoutTree,
            focus: &FocusManager,
            draw_list: &mut DrawList,
            scale_factor: f32,
            text_system_ptr: *mut TextSystem,
            device_ptr: *const Device,
            queue_ptr: *const Queue,
            elapsed_time: f32,
        ) {
            let id = widget.id();

            if let Some(layout) = layout_tree.get_absolute_layout(id) {
                // SAFETY: We control the lifetime and ensure no aliasing within this function
                let text_system = unsafe { &mut *text_system_ptr };
                let device = unsafe { &*device_ptr };
                let queue = unsafe { &*queue_ptr };

                // Scale layout bounds from logical to physical pixels
                // Layout is computed in logical pixels, but renderer uses physical pixels
                let scaled_layout = spark_layout::ComputedLayout::new(spark_core::Rect::new(
                    layout.bounds.x * scale_factor,
                    layout.bounds.y * scale_factor,
                    layout.bounds.width * scale_factor,
                    layout.bounds.height * scale_factor,
                ));

                let mut ctx = PaintContext {
                    draw_list,
                    layout: scaled_layout,
                    layout_tree,
                    focus,
                    widget_id: id,
                    scale_factor,
                    text_system,
                    device,
                    queue,
                    elapsed_time,
                };
                widget.paint(&mut ctx);

                // Paint children
                for child in widget.children() {
                    paint_widget(
                        child.as_ref(),
                        layout_tree,
                        focus,
                        ctx.draw_list,
                        scale_factor,
                        text_system_ptr,
                        device_ptr,
                        queue_ptr,
                        elapsed_time,
                    );
                }

                // Call after-paint hook for cleanup (e.g., pop transforms/clips)
                widget.paint_after_children(&mut ctx);
            }
        }

        paint_widget(
            state.root_widget.as_ref(),
            &state.layout_tree,
            &state.focus_manager,
            &mut state.draw_list,
            state.scale_factor,
            text_system_ptr,
            device_ptr,
            queue_ptr,
            elapsed_time,
        );

        state.needs_repaint = false;
    }

    fn handle_event(&mut self, event: InputEvent) {
        let Some(state) = self.state.as_mut() else {
            return;
        };

        // Simple event dispatch - dispatch to all widgets, let them check bounds
        fn dispatch_event(
            widget: &mut dyn Widget,
            layout_tree: &LayoutTree,
            focus_id: Option<spark_layout::WidgetId>,
            event: &InputEvent,
        ) -> (spark_widgets::EventResponse, Option<spark_layout::WidgetId>) {
            let id = widget.id();
            let layout = match layout_tree.get_absolute_layout(id) {
                Some(l) => l,
                None => {
                    return (spark_widgets::EventResponse::default(), focus_id);
                }
            };

            // First dispatch to children (bubble up)
            let mut new_focus = focus_id;
            for child in widget.children_mut() {
                let (response, focus) =
                    dispatch_event(child.as_mut(), layout_tree, new_focus, event);
                new_focus = focus;
                if response.handled {
                    return (response, new_focus);
                }
            }

            // Create a temporary focus manager for this dispatch
            let mut temp_focus = FocusManager::new();
            if let Some(fid) = new_focus {
                temp_focus.set_focus(fid);
            }

            let mut ctx = EventContext {
                layout,
                layout_tree,
                focus: &mut temp_focus,
                widget_id: id,
                has_capture: false,
            };

            let response = widget.event(&mut ctx, event);

            // Update focus
            if response.request_focus {
                new_focus = Some(id);
            } else if response.release_focus && new_focus == Some(id) {
                new_focus = None;
            }

            (response, new_focus)
        }

        let current_focus = state.focus_manager.focused();
        let (response, new_focus) = dispatch_event(
            state.root_widget.as_mut(),
            &state.layout_tree,
            current_focus,
            &event,
        );

        // Update focus manager
        if let Some(fid) = new_focus {
            state.focus_manager.set_focus(fid);
        } else if current_focus.is_some() && new_focus.is_none() {
            state.focus_manager.clear_focus();
        }

        if response.repaint {
            state.needs_repaint = true;
        }
        if response.relayout {
            state.needs_layout = true;
        }

        // Request redraw if we need to repaint or relayout
        if state.needs_repaint || state.needs_layout {
            state.window.request_redraw();
        }
    }
}

impl<F: FnOnce() -> Box<dyn Widget>> winit::application::ApplicationHandler for AppRunner<F> {
    fn can_create_surfaces(&mut self, event_loop: &dyn winit::event_loop::ActiveEventLoop) {
        let mut window_attributes = winit::window::WindowAttributes::default()
            .with_title(&self.config.title)
            .with_surface_size(winit::dpi::LogicalSize::new(
                self.config.width,
                self.config.height,
            ));

        #[cfg(target_arch = "wasm32")]
        {
            use winit::platform::web::WindowAttributesWeb;
            // Ensure the web canvas is attached to the document body automatically.
            window_attributes = window_attributes.with_platform_attributes(Box::new(
                WindowAttributesWeb::default().with_append(true),
            ));
        }

        let window = event_loop
            .create_window(window_attributes)
            .expect("create window");

        let window_leaked: &'static mut Box<dyn winit::window::Window> =
            Box::leak(Box::new(window));
        let window: &'static dyn winit::window::Window = &**window_leaked;

        #[cfg(not(target_arch = "wasm32"))]
        {
            let (device, queue, surface_state) =
                pollster::block_on(init_wgpu(window)).expect("initialize wgpu");

            let renderer = Renderer::new(&device, surface_state.config.format);
            let text_system = TextSystem::new(&device);
            let draw_list = DrawList::new();
            let layout_tree = LayoutTree::new();
            let focus_manager = FocusManager::new();

            // Build the UI
            let build_ui = self.build_ui.take().expect("build_ui already called");
            let root_widget = build_ui();

            let scale_factor = window.scale_factor() as f32;

            self.state = Some(AppState {
                window,
                device,
                queue,
                surface_state,
                renderer,
                text_system,
                draw_list,
                layout_tree,
                focus_manager,
                root_widget,
                start_time: Instant::now(),
                mouse_pos: glam::Vec2::ZERO,
                scale_factor,
                needs_layout: true,
                needs_repaint: true,
            });

            // Build initial layout
            self.build_layout();
        }

        #[cfg(target_arch = "wasm32")]
        {
            let build_ui = self.build_ui.take().expect("build_ui already called");
            let root_widget = build_ui();
            let scale_factor = window.scale_factor() as f32;
            let pending = Rc::new(RefCell::new(None));
            let pending_for_task = Rc::clone(&pending);

            wasm_bindgen_futures::spawn_local(async move {
                match init_wgpu(window).await {
                    Ok((device, queue, surface_state)) => {
                        let renderer = Renderer::new(&device, surface_state.config.format);
                        let text_system = TextSystem::new(&device);
                        let draw_list = DrawList::new();
                        let layout_tree = LayoutTree::new();
                        let focus_manager = FocusManager::new();

                        *pending_for_task.borrow_mut() = Some(AppState {
                            window,
                            device,
                            queue,
                            surface_state,
                            renderer,
                            text_system,
                            draw_list,
                            layout_tree,
                            focus_manager,
                            root_widget,
                            start_time: Instant::now(),
                            mouse_pos: glam::Vec2::ZERO,
                            scale_factor,
                            needs_layout: true,
                            needs_repaint: true,
                        });

                        window.request_redraw();
                    }
                    Err(err) => {
                        log::error!("failed to initialize wgpu on web: {err}");
                        if let Some(window) = web_sys::window() {
                            if let Some(document) = window.document() {
                                if let Some(body) = document.body() {
                                    body.set_inner_html(
                                        "<div style=\"margin:24px;font-family:system-ui,sans-serif;color:#b00020;\">\
                                         Spark could not initialize a compatible GPU adapter in this browser.\
                                         </div>",
                                    );
                                }
                            }
                        }
                    }
                }
            });

            self.pending_init = Some(pending);
        }
    }

    fn window_event(
        &mut self,
        event_loop: &dyn winit::event_loop::ActiveEventLoop,
        _id: winit::window::WindowId,
        event: WindowEvent,
    ) {
        match event {
            WindowEvent::CloseRequested => {
                event_loop.exit();
            }
            WindowEvent::SurfaceResized(size) => {
                if let Some(state) = self.state.as_mut() {
                    if size.width > 0 && size.height > 0 {
                        state
                            .surface_state
                            .resize(&state.device, size.width, size.height);
                        state.needs_layout = true;
                    }
                }
            }
            WindowEvent::ScaleFactorChanged { scale_factor, .. } => {
                if let Some(state) = self.state.as_mut() {
                    state.scale_factor = scale_factor as f32;
                    state.needs_layout = true;
                }
            }
            WindowEvent::PointerMoved { position, .. } => {
                // Convert physical pixels to logical pixels for event handling
                let scale_factor = self.state.as_ref().map(|s| s.scale_factor).unwrap_or(1.0);
                let pos = glam::Vec2::new(
                    position.x as f32 / scale_factor,
                    position.y as f32 / scale_factor,
                );
                if let Some(s) = self.state.as_mut() {
                    s.mouse_pos = pos;
                }
                self.handle_event(InputEvent::PointerMove { pos });
            }
            WindowEvent::PointerButton {
                state: btn_state,
                button,
                ..
            } => {
                let pos = self.state.as_ref().map(|s| s.mouse_pos).unwrap_or_default();
                let button = match button {
                    winit::event::ButtonSource::Mouse(mb) => match mb {
                        winit::event::MouseButton::Left => PointerButton::Primary,
                        winit::event::MouseButton::Right => PointerButton::Secondary,
                        winit::event::MouseButton::Middle => PointerButton::Auxiliary,
                        _ => PointerButton::Primary,
                    },
                    _ => PointerButton::Primary,
                };

                match btn_state {
                    winit::event::ElementState::Pressed => {
                        self.handle_event(InputEvent::PointerDown { pos, button });
                    }
                    winit::event::ElementState::Released => {
                        self.handle_event(InputEvent::PointerUp { pos, button });
                    }
                }
            }
            WindowEvent::MouseWheel { delta, .. } => {
                let pos = self.state.as_ref().map(|s| s.mouse_pos).unwrap_or_default();
                let delta = match delta {
                    winit::event::MouseScrollDelta::LineDelta(x, y) => glam::Vec2::new(x, y),
                    winit::event::MouseScrollDelta::PixelDelta(p) => {
                        glam::Vec2::new(p.x as f32 / 20.0, p.y as f32 / 20.0)
                    }
                };
                self.handle_event(InputEvent::Scroll { pos, delta });
            }
            WindowEvent::KeyboardInput { event, .. } => {
                use spark_input::{ui_events::keyboard::Code, Key, KeyboardEvent, NamedKey};

                let key = match &event.logical_key {
                    winit::keyboard::Key::Character(c) => Key::Character(c.to_string()),
                    winit::keyboard::Key::Named(named) => {
                        use winit::keyboard::NamedKey as WN;
                        Key::Named(match named {
                            WN::Enter => NamedKey::Enter,
                            WN::Tab => NamedKey::Tab,
                            WN::Backspace => NamedKey::Backspace,
                            WN::Delete => NamedKey::Delete,
                            WN::Escape => NamedKey::Escape,
                            WN::ArrowUp => NamedKey::ArrowUp,
                            WN::ArrowDown => NamedKey::ArrowDown,
                            WN::ArrowLeft => NamedKey::ArrowLeft,
                            WN::ArrowRight => NamedKey::ArrowRight,
                            WN::Home => NamedKey::Home,
                            WN::End => NamedKey::End,
                            WN::PageUp => NamedKey::PageUp,
                            WN::PageDown => NamedKey::PageDown,
                            _ => return,
                        })
                    }
                    _ => return,
                };

                // Use a generic code since we're translating from logical key
                let code = Code::Unidentified;

                let kb_event = if event.state.is_pressed() {
                    KeyboardEvent::key_down(key.clone(), code)
                } else {
                    KeyboardEvent::key_up(key, code)
                };

                if event.state.is_pressed() {
                    self.handle_event(InputEvent::KeyDown { event: kb_event });
                } else {
                    self.handle_event(InputEvent::KeyUp { event: kb_event });
                }

                // Handle text input
                if event.state.is_pressed() && !event.repeat {
                    if let Some(text) = event.text.as_ref() {
                        let text = text.to_string();
                        if !text.is_empty() && text.chars().all(|c| !c.is_control()) {
                            self.handle_event(InputEvent::TextInput { text });
                        }
                    }
                }
            }
            WindowEvent::Focused(focused) => {
                if focused {
                    self.handle_event(InputEvent::FocusGained);
                } else {
                    self.handle_event(InputEvent::FocusLost);
                }
            }
            WindowEvent::RedrawRequested => {
                if self.state.is_none() {
                    return;
                }

                let state = self.state.as_mut().expect("state checked above");

                if state.needs_layout {
                    self.build_layout();
                }

                let state = self.state.as_mut().unwrap();
                if state.needs_repaint {
                    self.paint();
                }

                let state = self.state.as_mut().unwrap();

                // Update renderer
                let size = state.surface_state.size;
                state.renderer.set_viewport(
                    size.width as f32,
                    size.height as f32,
                    state.scale_factor,
                );
                state
                    .renderer
                    .set_time(state.start_time.elapsed().as_secs_f32());

                // Prepare render
                state.renderer.prepare(
                    &state.device,
                    &state.queue,
                    &state.draw_list,
                    state.text_system.atlas(),
                );

                // Get frame
                let frame = match state.surface_state.surface.get_current_texture() {
                    Ok(frame) => frame,
                    Err(_) => {
                        state.surface_state.reconfigure(&state.device);
                        state.surface_state.surface.get_current_texture().unwrap()
                    }
                };

                let view = frame
                    .texture
                    .create_view(&wgpu::TextureViewDescriptor::default());

                let mut encoder =
                    state
                        .device
                        .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                            label: Some("spark_encoder"),
                        });

                let bg = self.config.background;
                state.renderer.render(
                    &mut encoder,
                    &view,
                    wgpu::Color {
                        r: bg.r as f64,
                        g: bg.g as f64,
                        b: bg.b as f64,
                        a: bg.a as f64,
                    },
                );

                state.queue.submit(Some(encoder.finish()));
                frame.present();
            }
            _ => {}
        }
    }

    fn about_to_wait(&mut self, _event_loop: &dyn winit::event_loop::ActiveEventLoop) {
        #[cfg(target_arch = "wasm32")]
        if self.state.is_none() {
            let ready_state = self
                .pending_init
                .as_ref()
                .and_then(|pending| pending.borrow_mut().take());
            if let Some(state) = ready_state {
                self.state = Some(state);
                self.build_layout();
            }
        }

        // Request redraw for animation
        // In a real app, you'd only request this when needed
    }
}
