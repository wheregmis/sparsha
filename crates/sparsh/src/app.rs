//! Application runner and main event loop.

use crate::router::Router;
#[cfg(not(target_arch = "wasm32"))]
use crate::router::RouterHost;
use sparsh_core::Color;
use sparsh_signals::{ReadSignal, Signal};
use sparsh_widgets::Theme;
#[cfg(not(target_arch = "wasm32"))]
use sparsh_widgets::{set_current_theme, Widget};

#[cfg(not(target_arch = "wasm32"))]
use crate::tasks::{TaskRuntime, TaskStatus};
#[cfg(not(target_arch = "wasm32"))]
use sparsh_core::{init_wgpu, SurfaceState};
#[cfg(not(target_arch = "wasm32"))]
use sparsh_input::{FocusManager, InputEvent, PointerButton};
#[cfg(not(target_arch = "wasm32"))]
use sparsh_layout::LayoutTree;
#[cfg(not(target_arch = "wasm32"))]
use sparsh_render::DrawList;
#[cfg(not(target_arch = "wasm32"))]
use sparsh_render::Renderer;
#[cfg(not(target_arch = "wasm32"))]
use sparsh_signals::{RuntimeHandle, SubscriberKind};
#[cfg(not(target_arch = "wasm32"))]
use sparsh_text::TextSystem;
#[cfg(not(target_arch = "wasm32"))]
use sparsh_widgets::{
    BuildContext, EventCommands, EventContext, LayoutContext, PaintCommands, PaintContext,
};
#[cfg(not(target_arch = "wasm32"))]
use wgpu::{Device, Queue};
#[cfg(not(target_arch = "wasm32"))]
use winit::event::WindowEvent;

#[cfg(not(target_arch = "wasm32"))]
use std::time::Instant;

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
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            title: String::from("Sparsh App"),
            width: 800,
            height: 600,
            background: Color::from_hex(0xF3F4F6),
        }
    }
}

#[derive(Clone)]
pub(crate) enum ThemeSource {
    Static(Theme),
    Dynamic(ReadSignal<Theme>),
}

impl ThemeSource {
    pub(crate) fn resolve(&self) -> Theme {
        match self {
            Self::Static(theme) => theme.clone(),
            Self::Dynamic(theme) => theme.get(),
        }
    }
}

pub enum ThemeInput {
    Static(Theme),
    Dynamic(ReadSignal<Theme>),
}

impl ThemeInput {
    fn into_source(self) -> ThemeSource {
        match self {
            Self::Static(theme) => ThemeSource::Static(theme),
            Self::Dynamic(theme) => ThemeSource::Dynamic(theme),
        }
    }
}

impl From<Theme> for ThemeInput {
    fn from(value: Theme) -> Self {
        Self::Static(value)
    }
}

impl From<Signal<Theme>> for ThemeInput {
    fn from(value: Signal<Theme>) -> Self {
        Self::Dynamic(value.read_only())
    }
}

impl From<ReadSignal<Theme>> for ThemeInput {
    fn from(value: ReadSignal<Theme>) -> Self {
        Self::Dynamic(value)
    }
}

/// The main application struct.
pub struct App {
    config: AppConfig,
    theme: ThemeSource,
    router: Router,
}

impl App {
    /// Create a new app with default configuration.
    pub fn new() -> Self {
        Self {
            config: AppConfig::default(),
            theme: ThemeSource::Static(Theme::default()),
            router: Router::new()
                .route("/", || Box::new(sparsh_widgets::Container::new().fill()))
                .fallback("/"),
        }
    }

    /// Set the window title.
    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.config.title = title.into();
        self
    }

    /// Set the initial window size.
    pub fn size(mut self, width: u32, height: u32) -> Self {
        self.config.width = width;
        self.config.height = height;
        self
    }

    /// Set the background color.
    pub fn background(mut self, color: Color) -> Self {
        self.config.background = color;
        self
    }

    /// Set application theme source.
    pub fn theme<T: Into<ThemeInput>>(mut self, theme: T) -> Self {
        self.theme = theme.into().into_source();
        self
    }

    /// Set the app router.
    pub fn router(mut self, router: Router) -> Self {
        self.router = router;
        self
    }

    /// Run the application.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn run(self) -> ! {
        let event_loop = winit::event_loop::EventLoop::new().unwrap();
        let runner = AppRunner::new(self.config, self.theme, self.router);
        let runner_leaked: &'static mut AppRunner = Box::leak(Box::new(runner));
        event_loop.run_app(runner_leaked).unwrap();
        std::process::exit(0);
    }

    /// Run the application.
    ///
    /// On web targets this returns after registering the app with the browser event loop.
    #[cfg(target_arch = "wasm32")]
    pub fn run(self) {
        crate::web_app::run_dom_app(self.config, self.theme, self.router);
    }
}

impl Default for App {
    fn default() -> Self {
        Self::new()
    }
}

/// Internal application runner that handles the event loop.
#[cfg(not(target_arch = "wasm32"))]
struct AppRunner {
    config: AppConfig,
    theme: ThemeSource,
    router: Router,
    state: Option<AppState>,
}

#[cfg(not(target_arch = "wasm32"))]
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
    signal_runtime: RuntimeHandle,
    task_runtime: TaskRuntime,
    theme: ThemeSource,
    root_widget: Box<dyn Widget>,
    start_time: Instant,
    mouse_pos: glam::Vec2,
    scale_factor: f32,
    needs_layout: bool,
    needs_repaint: bool,
}

#[cfg(not(target_arch = "wasm32"))]
impl AppRunner {
    fn new(config: AppConfig, theme: ThemeSource, router: Router) -> Self {
        Self {
            config,
            theme,
            router,
            state: None,
        }
    }

    fn build_layout(&mut self) {
        let state = self.state.as_mut().unwrap();
        let runtime = state.signal_runtime.clone();

        // Clear layout tree
        state.layout_tree = LayoutTree::new();

        runtime.with_tracking(SubscriberKind::Rebuild, || {
            set_current_theme(state.theme.resolve());

            fn rebuild_widget(widget: &mut dyn Widget, build_ctx: &mut BuildContext) {
                widget.rebuild(build_ctx);
                for child in widget.children_mut() {
                    rebuild_widget(child.as_mut(), build_ctx);
                }
            }

            let mut build_ctx = BuildContext::default();
            rebuild_widget(state.root_widget.as_mut(), &mut build_ctx);
        });

        // Build layout tree from widget tree
        fn add_to_layout(
            widget: &mut dyn Widget,
            tree: &mut LayoutTree,
            text_system: &mut TextSystem,
            in_scroll: bool,
        ) -> sparsh_layout::WidgetId {
            use sparsh_layout::taffy::Dimension;

            let mut style = widget.style();
            if in_scroll {
                style.flex_shrink = 0.0;
            }
            let is_scroll = widget.is_scroll_container();
            let children_ids: Vec<_> = widget
                .children_mut()
                .iter_mut()
                .map(|child| {
                    add_to_layout(child.as_mut(), tree, text_system, in_scroll || is_scroll)
                })
                .collect();

            let id = if children_ids.is_empty() {
                // Apply intrinsic height for leaf widgets that provide measurement.
                // This prevents text-like widgets from collapsing to zero height.
                let mut layout_ctx = LayoutContext {
                    text: text_system,
                    max_width: None,
                    max_height: None,
                };
                if let Some((_, measured_height)) = widget.measure(&mut layout_ctx) {
                    let valid_height = measured_height.is_finite() && measured_height > 0.0;
                    if valid_height {
                        let current_min_height = style.min_size.height;
                        let current_min_height_value = if current_min_height.is_auto() {
                            0.0
                        } else {
                            current_min_height.value()
                        };
                        if measured_height > current_min_height_value {
                            style.min_size.height = Dimension::length(measured_height);
                        }
                    }
                }
                tree.new_leaf(style)
            } else {
                tree.new_with_children(style, &children_ids)
            };

            widget.set_id(id);
            id
        }

        let root_id = runtime.with_tracking(SubscriberKind::Layout, || {
            set_current_theme(state.theme.resolve());
            add_to_layout(
                state.root_widget.as_mut(),
                &mut state.layout_tree,
                &mut state.text_system,
                false,
            )
        });
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
        let runtime = state.signal_runtime.clone();
        state.draw_list.clear();

        // Get elapsed time for animations
        let elapsed_time = state.start_time.elapsed().as_secs_f32();

        // We need to use raw pointers to pass mutable references through the recursive function
        // This is safe because we control the lifetime and don't alias
        let text_system_ptr = &mut state.text_system as *mut TextSystem;
        let mut paint_commands = PaintCommands::default();

        #[allow(clippy::too_many_arguments)]
        fn paint_widget(
            widget: &dyn Widget,
            layout_tree: &LayoutTree,
            focus: &FocusManager,
            draw_list: &mut DrawList,
            scale_factor: f32,
            text_system_ptr: *mut TextSystem,
            elapsed_time: f32,
            paint_commands: &mut PaintCommands,
        ) {
            let id = widget.id();

            if let Some(layout) = layout_tree.get_absolute_layout(id) {
                // SAFETY: We control the lifetime and ensure no aliasing within this function
                let text_system = unsafe { &mut *text_system_ptr };
                let mut local_commands = PaintCommands::default();

                // Scale layout bounds from logical to physical pixels
                // Layout is computed in logical pixels, but renderer uses physical pixels
                let scaled_layout = sparsh_layout::ComputedLayout::new(sparsh_core::Rect::new(
                    layout.bounds.x * scale_factor,
                    layout.bounds.y * scale_factor,
                    layout.bounds.width * scale_factor,
                    layout.bounds.height * scale_factor,
                ));

                {
                    let mut ctx = PaintContext {
                        draw_list,
                        layout: scaled_layout,
                        layout_tree,
                        focus,
                        widget_id: id,
                        scale_factor,
                        text_system,
                        elapsed_time,
                        commands: &mut local_commands,
                    };
                    widget.paint(&mut ctx);
                }

                // Paint children
                for child in widget.children() {
                    paint_widget(
                        child.as_ref(),
                        layout_tree,
                        focus,
                        draw_list,
                        scale_factor,
                        text_system_ptr,
                        elapsed_time,
                        &mut local_commands,
                    );
                }

                // Call after-paint hook for cleanup (e.g., pop transforms/clips)
                let text_system = unsafe { &mut *text_system_ptr };
                let mut ctx = PaintContext {
                    draw_list,
                    layout: scaled_layout,
                    layout_tree,
                    focus,
                    widget_id: id,
                    scale_factor,
                    text_system,
                    elapsed_time,
                    commands: &mut local_commands,
                };
                widget.paint_after_children(&mut ctx);
                paint_commands.merge(local_commands);
            }
        }

        runtime.with_tracking(SubscriberKind::Paint, || {
            set_current_theme(state.theme.resolve());
            paint_widget(
                state.root_widget.as_ref(),
                &state.layout_tree,
                &state.focus_manager,
                &mut state.draw_list,
                state.scale_factor,
                text_system_ptr,
                elapsed_time,
                &mut paint_commands,
            );
        });

        state.needs_repaint = paint_commands.request_next_frame;
    }

    fn handle_event(&mut self, event: InputEvent) {
        let Some(state) = self.state.as_mut() else {
            return;
        };

        fn dispatch_event(
            widget: &mut dyn Widget,
            layout_tree: &LayoutTree,
            focus_id: Option<sparsh_layout::WidgetId>,
            event: &InputEvent,
            aggregate: &mut EventCommands,
        ) -> Option<sparsh_layout::WidgetId> {
            let id = widget.id();
            let layout = match layout_tree.get_absolute_layout(id) {
                Some(l) => l,
                None => return focus_id,
            };

            let mut new_focus = focus_id;
            for child in widget.children_mut() {
                new_focus =
                    dispatch_event(child.as_mut(), layout_tree, new_focus, event, aggregate);
                if aggregate.stop_propagation {
                    return new_focus;
                }
            }

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
                commands: EventCommands::default(),
            };
            widget.event(&mut ctx, event);

            if ctx.commands.request_focus {
                new_focus = Some(id);
            } else if ctx.commands.clear_focus && new_focus == Some(id) {
                new_focus = None;
            }

            aggregate.merge(ctx.commands);
            new_focus
        }

        let runtime = state.signal_runtime.clone();
        let current_focus = state.focus_manager.focused();
        let mut commands = EventCommands::default();
        let new_focus = runtime.run_with_current(|| {
            dispatch_event(
                state.root_widget.as_mut(),
                &state.layout_tree,
                current_focus,
                &event,
                &mut commands,
            )
        });

        // Update focus manager
        if let Some(fid) = new_focus {
            state.focus_manager.set_focus(fid);
        } else if current_focus.is_some() && new_focus.is_none() {
            state.focus_manager.clear_focus();
        }

        if commands.request_paint {
            state.needs_repaint = true;
        }
        if commands.request_layout {
            state.needs_layout = true;
        }

        runtime.run_effects(64);
        let dirty = runtime.take_dirty_flags();
        if dirty.rebuild || dirty.layout {
            state.needs_layout = true;
        }
        if dirty.paint {
            state.needs_repaint = true;
        }

        // Request redraw if we need to repaint or relayout
        if state.needs_repaint || state.needs_layout {
            state.window.request_redraw();
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
impl winit::application::ApplicationHandler for AppRunner {
    fn can_create_surfaces(&mut self, event_loop: &dyn winit::event_loop::ActiveEventLoop) {
        let window_attributes = winit::window::WindowAttributes::default()
            .with_title(&self.config.title)
            .with_surface_size(winit::dpi::LogicalSize::new(
                self.config.width,
                self.config.height,
            ));

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
            let signal_runtime = RuntimeHandle::current_or_default();
            let task_runtime = TaskRuntime::new();
            task_runtime.set_current();
            let window_for_scheduler = window;
            signal_runtime.set_scheduler(move || {
                window_for_scheduler.request_redraw();
            });

            self.router.initialize(None);
            let router = self.router.clone();
            let root_widget = signal_runtime
                .run_with_current(|| Box::new(RouterHost::new(router.clone())) as Box<dyn Widget>);

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
                signal_runtime,
                task_runtime,
                theme: self.theme.clone(),
                root_widget,
                start_time: Instant::now(),
                mouse_pos: glam::Vec2::ZERO,
                scale_factor,
                needs_layout: true,
                needs_repaint: true,
            });

            // Build initial layout
            self.build_layout();
            // Trigger the very first frame; some platforms won't emit an initial redraw.
            if let Some(state) = self.state.as_ref() {
                state.window.request_redraw();
            }
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
                use sparsh_input::{ui_events::keyboard::Code, Key, KeyboardEvent, NamedKey};

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
                    &mut state.text_system,
                );

                // Get frame
                let frame = match state.surface_state.surface.get_current_texture() {
                    wgpu::CurrentSurfaceTexture::Success(frame) => frame,
                    wgpu::CurrentSurfaceTexture::Suboptimal(frame) => {
                        state.surface_state.reconfigure(&state.device);
                        frame
                    }
                    wgpu::CurrentSurfaceTexture::Outdated | wgpu::CurrentSurfaceTexture::Lost => {
                        state.surface_state.reconfigure(&state.device);
                        return;
                    }
                    wgpu::CurrentSurfaceTexture::Timeout
                    | wgpu::CurrentSurfaceTexture::Occluded
                    | wgpu::CurrentSurfaceTexture::Validation => {
                        return;
                    }
                };

                let view = frame
                    .texture
                    .create_view(&wgpu::TextureViewDescriptor::default());

                let mut encoder =
                    state
                        .device
                        .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                            label: Some("sparsh_encoder"),
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
        if let Some(state) = self.state.as_mut() {
            let mut had_task_results = false;
            state.task_runtime.drain_completed(|result| {
                had_task_results = true;
                if let TaskStatus::Error(message) = &result.status {
                    log::warn!(
                        "background task failed (id={}, kind={}): {}",
                        result.task_id,
                        result.task_kind,
                        message
                    );
                }
            });

            state.signal_runtime.run_effects(64);
            let dirty = state.signal_runtime.take_dirty_flags();
            if dirty.rebuild || dirty.layout {
                state.needs_layout = true;
            }
            if dirty.paint {
                state.needs_repaint = true;
            }
            if had_task_results {
                state.needs_repaint = true;
            }
            if state.needs_layout || state.needs_repaint {
                state.window.request_redraw();
            }
            if state.task_runtime.has_in_flight() {
                state.window.request_redraw();
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sparsh_signals::{RuntimeHandle, SubscriberKind};

    #[test]
    fn dynamic_theme_source_marks_rebuild_dirty() {
        let runtime = RuntimeHandle::new();
        runtime.run_with_current(|| {
            let theme_signal = Signal::new(Theme::default());
            let source = ThemeInput::from(theme_signal).into_source();

            runtime.with_tracking(SubscriberKind::Rebuild, || {
                let _ = source.resolve();
            });

            let mut updated = Theme::default();
            updated.typography.body_size = 20.0;
            theme_signal.set(updated);

            let dirty = runtime.take_dirty_flags();
            assert!(dirty.rebuild);
            assert!(dirty.layout);
            assert!(dirty.paint);
        });
    }
}
