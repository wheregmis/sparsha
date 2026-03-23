//! Application runner and main event loop.

#[cfg(not(target_arch = "wasm32"))]
use crate::accessibility::action_from_accesskit;
#[cfg(not(target_arch = "wasm32"))]
use crate::component::ComponentStateStore;
#[cfg(not(target_arch = "wasm32"))]
use crate::platform::events::{NativeEventTranslator, NativeKeyboardDispatch};
#[cfg(not(target_arch = "wasm32"))]
use crate::platform::{NativePlatform, PlatformId};
use crate::router::Router;
#[cfg(not(target_arch = "wasm32"))]
use crate::router::RouterHost;
#[cfg(not(target_arch = "wasm32"))]
use crate::runtime_core::{focused_text_editor_state, RuntimeCoreContext, RuntimeHost};
#[cfg(not(target_arch = "wasm32"))]
use crate::runtime_widget::{WidgetPath, WidgetRuntimeRegistry};
use sparsha_core::Color;
use sparsha_core::WgpuInitError;
use sparsha_signals::{ReadSignal, Signal};
use sparsha_widgets::Theme;
#[cfg(not(target_arch = "wasm32"))]
use sparsha_widgets::{set_current_theme, set_current_viewport, ViewportInfo, Widget};

#[cfg(not(target_arch = "wasm32"))]
use crate::tasks::{TaskRuntime, TaskStatus};
#[cfg(not(target_arch = "wasm32"))]
use sparsha_core::{init_wgpu, SurfaceState};
#[cfg(not(target_arch = "wasm32"))]
use sparsha_input::{FocusManager, InputEvent, Modifiers, PointerButton};
#[cfg(not(target_arch = "wasm32"))]
use sparsha_layout::LayoutTree;
#[cfg(not(target_arch = "wasm32"))]
use sparsha_render::DrawList;
#[cfg(not(target_arch = "wasm32"))]
use sparsha_render::Renderer;
#[cfg(not(target_arch = "wasm32"))]
use sparsha_signals::{RuntimeHandle, SubscriberKind};
#[cfg(not(target_arch = "wasm32"))]
use sparsha_text::TextSystem;
#[cfg(not(target_arch = "wasm32"))]
use sparsha_widgets::{PaintCommands, PaintContext};
#[cfg(not(target_arch = "wasm32"))]
use wgpu::{Device, Queue};
#[cfg(not(target_arch = "wasm32"))]
use winit::event::WindowEvent;
#[cfg(not(target_arch = "wasm32"))]
use winit::event_loop::EventLoopProxy;

#[cfg(not(target_arch = "wasm32"))]
use std::sync::{Arc, Mutex};
#[cfg(not(target_arch = "wasm32"))]
use std::time::Instant;
#[cfg(not(target_arch = "wasm32"))]
use winit::platform::modifier_supplement::KeyEventExtModifierSupplement;

/// Application configuration.
pub struct AppConfig {
    /// Window title.
    pub title: String,
    /// Initial window width.
    pub width: u32,
    /// Initial window height.
    pub height: u32,
    /// Optional background color override.
    ///
    /// When unset, the active theme background is used.
    pub background_override: Option<Color>,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            title: String::from("Sparsha App"),
            width: 800,
            height: 600,
            background_override: None,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ThemeMode {
    #[default]
    Light,
    Dark,
}

#[allow(clippy::large_enum_variant)]
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

#[derive(Clone)]
pub(crate) enum ThemeModeSource {
    Static(ThemeMode),
    Dynamic(ReadSignal<ThemeMode>),
}

impl ThemeModeSource {
    pub(crate) fn resolve(&self) -> ThemeMode {
        match self {
            Self::Static(mode) => *mode,
            Self::Dynamic(mode) => mode.get(),
        }
    }
}

#[allow(clippy::large_enum_variant)]
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

pub enum ThemeModeInput {
    Static(ThemeMode),
    Dynamic(ReadSignal<ThemeMode>),
}

impl ThemeModeInput {
    fn into_source(self) -> ThemeModeSource {
        match self {
            Self::Static(mode) => ThemeModeSource::Static(mode),
            Self::Dynamic(mode) => ThemeModeSource::Dynamic(mode),
        }
    }
}

impl From<ThemeMode> for ThemeModeInput {
    fn from(value: ThemeMode) -> Self {
        Self::Static(value)
    }
}

impl From<Signal<ThemeMode>> for ThemeModeInput {
    fn from(value: Signal<ThemeMode>) -> Self {
        Self::Dynamic(value.read_only())
    }
}

impl From<ReadSignal<ThemeMode>> for ThemeModeInput {
    fn from(value: ReadSignal<ThemeMode>) -> Self {
        Self::Dynamic(value)
    }
}

#[derive(Clone)]
pub(crate) struct AppTheme {
    light: ThemeSource,
    dark: Option<ThemeSource>,
    mode: ThemeModeSource,
}

impl AppTheme {
    pub(crate) fn new(light: ThemeSource) -> Self {
        Self {
            light,
            dark: None,
            mode: ThemeModeSource::Static(ThemeMode::Light),
        }
    }

    pub(crate) fn resolve_theme(&self) -> Theme {
        match self.mode.resolve() {
            ThemeMode::Light => self.light.resolve(),
            ThemeMode::Dark => self
                .dark
                .as_ref()
                .map(ThemeSource::resolve)
                .unwrap_or_else(|| self.light.resolve()),
        }
    }

    pub(crate) fn resolve_background(&self, background_override: Option<Color>) -> Color {
        background_override.unwrap_or_else(|| self.resolve_theme().colors.background)
    }
}

#[derive(Debug)]
pub enum AppRunError {
    EventLoopCreation(String),
    EventLoopRun(String),
    WindowCreation(String),
    GraphicsInit(WgpuInitError),
    TaskRuntimeInit(String),
    WebEnvironment(&'static str),
    DomMount(String),
    HybridSurfaceInit(String),
}

impl core::fmt::Display for AppRunError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::EventLoopCreation(message) => {
                write!(f, "failed to create native event loop: {message}")
            }
            Self::EventLoopRun(message) => write!(f, "native event loop failed: {message}"),
            Self::WindowCreation(message) => write!(f, "failed to create window: {message}"),
            Self::GraphicsInit(err) => write!(f, "failed to initialize graphics: {err}"),
            Self::TaskRuntimeInit(message) => {
                write!(f, "failed to initialize background task runtime: {message}")
            }
            Self::WebEnvironment(message) => {
                write!(f, "missing required web environment: {message}")
            }
            Self::DomMount(message) => write!(f, "failed to mount DOM renderer: {message}"),
            Self::HybridSurfaceInit(message) => {
                write!(f, "failed to initialize hybrid web surfaces: {message}")
            }
        }
    }
}

impl std::error::Error for AppRunError {}

impl From<WgpuInitError> for AppRunError {
    fn from(value: WgpuInitError) -> Self {
        Self::GraphicsInit(value)
    }
}

#[cfg(not(target_arch = "wasm32"))]
#[derive(Debug)]
enum NativeUserEvent {
    Accessibility(accesskit_winit::Event),
}

#[cfg(not(target_arch = "wasm32"))]
impl From<accesskit_winit::Event> for NativeUserEvent {
    fn from(value: accesskit_winit::Event) -> Self {
        Self::Accessibility(value)
    }
}

/// The main application struct.
pub struct App {
    config: AppConfig,
    theme: AppTheme,
    router: Router,
}

impl App {
    /// Create a new app with default configuration.
    pub fn new() -> Self {
        Self {
            config: AppConfig::default(),
            theme: AppTheme::new(ThemeSource::Static(Theme::default())),
            router: Router::new()
                .route("/", || sparsha_widgets::Container::new().fill())
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
        self.config.background_override = Some(color);
        self
    }

    /// Set application theme source.
    pub fn theme<T: Into<ThemeInput>>(mut self, theme: T) -> Self {
        self.theme.light = theme.into().into_source();
        self
    }

    /// Set dark theme source.
    pub fn dark_theme<T: Into<ThemeInput>>(mut self, theme: T) -> Self {
        self.theme.dark = Some(theme.into().into_source());
        self
    }

    /// Set theme mode source.
    pub fn theme_mode<T: Into<ThemeModeInput>>(mut self, mode: T) -> Self {
        self.theme.mode = mode.into().into_source();
        self
    }

    /// Set the app router.
    pub fn router(mut self, router: Router) -> Self {
        self.router = router;
        self
    }

    /// Run the application.
    ///
    /// Returns an error if the native event loop, window bootstrap, graphics setup,
    /// or background task runtime cannot be initialized.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn run(self) -> Result<(), AppRunError> {
        let event_loop = winit::event_loop::EventLoop::<NativeUserEvent>::with_user_event()
            .build()
            .map_err(|err| AppRunError::EventLoopCreation(err.to_string()))?;
        let event_loop_proxy = event_loop.create_proxy();
        let startup_error = Arc::new(Mutex::new(None));
        let runner = AppRunner::new(
            self.config,
            self.theme,
            self.router,
            Arc::clone(&startup_error),
            event_loop_proxy,
        );
        let runner_leaked: &'static mut AppRunner = Box::leak(Box::new(runner));
        let run_result = event_loop.run_app(runner_leaked);
        let startup_error = match startup_error.lock() {
            Ok(mut guard) => guard.take(),
            Err(poisoned) => {
                log::warn!("recovering from poisoned startup error state");
                poisoned.into_inner().take()
            }
        };
        if let Some(error) = startup_error {
            return Err(error);
        }
        run_result.map_err(|err| AppRunError::EventLoopRun(err.to_string()))
    }

    /// Run the application.
    ///
    /// On web targets this returns after registering the app with the browser event loop.
    #[cfg(target_arch = "wasm32")]
    pub fn run(self) -> Result<(), AppRunError> {
        crate::web_app::run_dom_app(self.config, self.theme, self.router)
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
    theme: AppTheme,
    router: Router,
    state: Option<AppState>,
    startup_error: Arc<Mutex<Option<AppRunError>>>,
    event_loop_proxy: EventLoopProxy<NativeUserEvent>,
}

#[cfg(not(target_arch = "wasm32"))]
struct AppState {
    navigator: crate::router::Navigator,
    platform: NativePlatform,
    window: &'static winit::window::Window,
    device: Device,
    queue: Queue,
    surface_state: SurfaceState<'static>,
    renderer: Renderer,
    text_system: TextSystem,
    draw_list: DrawList,
    layout_tree: LayoutTree,
    widget_registry: WidgetRuntimeRegistry,
    component_states: ComponentStateStore,
    focus_manager: FocusManager,
    focused_path: Option<WidgetPath>,
    capture_path: Option<WidgetPath>,
    signal_runtime: RuntimeHandle,
    task_runtime: TaskRuntime,
    theme: AppTheme,
    root_widget: Box<dyn Widget>,
    start_time: Instant,
    mouse_pos: glam::Vec2,
    modifiers: Modifiers,
    scale_factor: f32,
    ime_composing: bool,
    needs_layout: bool,
    needs_repaint: bool,
}

#[cfg(not(target_arch = "wasm32"))]
impl AppState {
    fn logical_viewport(&self) -> ViewportInfo {
        native_viewport_info(self.surface_state.size, self.scale_factor)
    }

    fn focused_text_editor_state(&self) -> Option<&sparsha_widgets::TextEditorState> {
        focused_text_editor_state(&self.widget_registry, self.focused_path.as_deref())
    }

    fn runtime_host(&mut self) -> RuntimeHost<'_> {
        let viewport = self.logical_viewport();
        let shortcut_profile = self.platform.shortcut_profile();
        RuntimeHost::from(RuntimeCoreContext {
            theme: &self.theme,
            navigator: self.navigator.clone(),
            root_widget: self.root_widget.as_mut(),
            layout_tree: &mut self.layout_tree,
            widget_registry: &mut self.widget_registry,
            component_states: &mut self.component_states,
            focus_manager: &mut self.focus_manager,
            focused_path: &mut self.focused_path,
            capture_path: &mut self.capture_path,
            signal_runtime: self.signal_runtime.clone(),
            task_runtime: self.task_runtime.clone(),
            text_system: &mut self.text_system,
            viewport,
            shortcut_profile,
            ime_composing: &mut self.ime_composing,
            needs_layout: &mut self.needs_layout,
            needs_repaint: &mut self.needs_repaint,
        })
    }

    fn sync_window_metrics(&mut self) -> bool {
        let mut changed = false;

        let actual_scale_factor = self.window.scale_factor() as f32;
        if should_sync_native_scale_factor(self.scale_factor, actual_scale_factor) {
            self.scale_factor = actual_scale_factor;
            changed = true;
        }

        let actual_size = self.window.inner_size();
        if should_sync_native_surface_size(self.surface_state.size, actual_size) {
            self.surface_state
                .resize(&self.device, actual_size.width, actual_size.height);
            changed = true;
        }

        if changed {
            self.needs_layout = true;
            self.needs_repaint = true;
        }

        changed
    }
}

#[cfg(not(target_arch = "wasm32"))]
impl AppRunner {
    fn new(
        config: AppConfig,
        theme: AppTheme,
        router: Router,
        startup_error: Arc<Mutex<Option<AppRunError>>>,
        event_loop_proxy: EventLoopProxy<NativeUserEvent>,
    ) -> Self {
        Self {
            config,
            theme,
            router,
            state: None,
            startup_error,
            event_loop_proxy,
        }
    }

    fn fail_startup(
        &mut self,
        event_loop: &winit::event_loop::ActiveEventLoop,
        error: AppRunError,
    ) {
        match self.startup_error.lock() {
            Ok(mut guard) => {
                *guard = Some(error);
            }
            Err(poisoned) => {
                log::warn!("recovering from poisoned startup error state");
                *poisoned.into_inner() = Some(error);
            }
        }
        event_loop.exit();
    }

    fn refresh_accessibility(&mut self) {
        let Some(state) = self.state.as_mut() else {
            return;
        };
        let (snapshot, focused_editor_state, has_capture) = {
            let mut host = state.runtime_host();
            let snapshot = host.refresh_accessibility();
            let focused_editor_state = host.focused_text_editor_state().cloned();
            let has_capture = host.has_pointer_capture();
            (snapshot, focused_editor_state, has_capture)
        };
        state.platform.apply_effects(
            state.window,
            &self.config.title,
            &crate::platform::PlatformEffects::default(),
            focused_editor_state.as_ref(),
            has_capture,
            &snapshot,
        );
    }

    fn handle_accessibility_action(
        &mut self,
        request: crate::accessibility::RoutedAccessibilityAction,
    ) {
        let Some(state) = self.state.as_mut() else {
            return;
        };
        let (effects, snapshot, focused_editor_state, has_capture) = {
            let mut host = state.runtime_host();
            let effects =
                host.handle_accessibility_action(request.node_id, request.action, request.value);
            let snapshot = host.refresh_accessibility();
            let focused_editor_state = host.focused_text_editor_state().cloned();
            let has_capture = host.has_pointer_capture();
            (effects, snapshot, focused_editor_state, has_capture)
        };
        state.platform.apply_effects(
            state.window,
            &self.config.title,
            &effects,
            focused_editor_state.as_ref(),
            has_capture,
            &snapshot,
        );
        if let Some(state) = self.state.as_ref() {
            if state.needs_layout || state.needs_repaint {
                state.window.request_redraw();
            }
        }
    }

    fn build_layout(&mut self) {
        let Some(state) = self.state.as_mut() else {
            return;
        };
        let (effects, snapshot, focused_editor_state, has_capture) = {
            let mut host = state.runtime_host();
            let effects = host.build_layout();
            let snapshot = host.refresh_accessibility();
            let focused_editor_state = host.focused_text_editor_state().cloned();
            let has_capture = host.has_pointer_capture();
            (effects, snapshot, focused_editor_state, has_capture)
        };
        state.platform.apply_effects(
            state.window,
            &self.config.title,
            &effects,
            focused_editor_state.as_ref(),
            has_capture,
            &snapshot,
        );
    }

    fn paint(&mut self) {
        let Some(state) = self.state.as_mut() else {
            return;
        };
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
                let scaled_layout = sparsha_layout::ComputedLayout::new(sparsha_core::Rect::new(
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
            set_current_theme(state.theme.resolve_theme());
            set_current_viewport(state.logical_viewport());
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

        state.needs_layout |= paint_commands.request_layout;
        state.needs_repaint = paint_commands.request_next_frame || paint_commands.request_layout;
    }

    fn handle_event(&mut self, event: InputEvent) {
        let Some(state) = self.state.as_mut() else {
            return;
        };
        let clipboard_text = state.platform.read_clipboard_text();
        let (effects, snapshot, focused_editor_state, has_capture) = {
            let mut host = state.runtime_host();
            let effects = host.handle_input_event(event, clipboard_text);
            let snapshot = host.refresh_accessibility();
            let focused_editor_state = host.focused_text_editor_state().cloned();
            let has_capture = host.has_pointer_capture();
            (effects, snapshot, focused_editor_state, has_capture)
        };
        state.platform.apply_effects(
            state.window,
            &self.config.title,
            &effects,
            focused_editor_state.as_ref(),
            has_capture,
            &snapshot,
        );
        if let Some(state) = self.state.as_ref() {
            if state.needs_repaint || state.needs_layout {
                state.window.request_redraw();
            }
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
#[cfg_attr(not(test), allow(dead_code))]
fn map_winit_key(key: &winit::keyboard::Key<&str>) -> Option<sparsha_input::Key> {
    NativeEventTranslator::new(PlatformId::current_native()).map_key(key)
}

#[cfg(not(target_arch = "wasm32"))]
#[cfg_attr(not(test), allow(dead_code))]
fn should_emit_native_text(text: &str, modifiers: Modifiers) -> bool {
    NativeEventTranslator::new(PlatformId::current_native()).should_emit_text(text, modifiers)
}

#[cfg(not(target_arch = "wasm32"))]
fn should_sync_native_surface_size(
    current: winit::dpi::PhysicalSize<u32>,
    actual: winit::dpi::PhysicalSize<u32>,
) -> bool {
    actual.width > 0 && actual.height > 0 && actual != current
}

#[cfg(not(target_arch = "wasm32"))]
fn should_sync_native_scale_factor(current: f32, actual: f32) -> bool {
    actual.is_finite() && actual > 0.0 && (actual - current).abs() > f32::EPSILON
}

#[cfg(not(target_arch = "wasm32"))]
fn native_viewport_info(size: winit::dpi::PhysicalSize<u32>, scale_factor: f32) -> ViewportInfo {
    let scale_factor = scale_factor.max(1.0);
    ViewportInfo::new(
        size.width as f32 / scale_factor,
        size.height as f32 / scale_factor,
    )
}

#[cfg(not(target_arch = "wasm32"))]
#[allow(deprecated)]
fn set_native_ime_allowed(window: &winit::window::Window, allowed: bool) {
    window.set_ime_allowed(allowed);
}

#[cfg(not(target_arch = "wasm32"))]
impl winit::application::ApplicationHandler<NativeUserEvent> for AppRunner {
    fn resumed(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
        if self.state.is_some() {
            return;
        }
        let window_attributes = winit::window::WindowAttributes::default()
            .with_title(&self.config.title)
            .with_visible(false)
            .with_inner_size(winit::dpi::LogicalSize::new(
                self.config.width,
                self.config.height,
            ));

        let window = event_loop.create_window(window_attributes);
        let window = match window {
            Ok(window) => window,
            Err(err) => {
                self.fail_startup(event_loop, AppRunError::WindowCreation(err.to_string()));
                return;
            }
        };

        let window: &'static winit::window::Window = Box::leak(Box::new(window));

        #[cfg(not(target_arch = "wasm32"))]
        {
            let (device, queue, surface_state) = match pollster::block_on(init_wgpu(window)) {
                Ok(value) => value,
                Err(err) => {
                    self.fail_startup(event_loop, AppRunError::from(err));
                    return;
                }
            };

            let renderer = Renderer::new(&device, surface_state.config.format);
            let text_system = TextSystem::new(&device);
            let draw_list = DrawList::new();
            let layout_tree = LayoutTree::new();
            let widget_registry = WidgetRuntimeRegistry::default();
            let component_states = ComponentStateStore::default();
            let focus_manager = FocusManager::new();
            let signal_runtime = RuntimeHandle::current_or_default();
            let task_runtime = match TaskRuntime::try_new() {
                Ok(runtime) => runtime,
                Err(err) => {
                    self.fail_startup(event_loop, AppRunError::TaskRuntimeInit(err.to_string()));
                    return;
                }
            };
            task_runtime.set_current();
            let window_for_scheduler = window;
            signal_runtime.set_scheduler(move || {
                window_for_scheduler.request_redraw();
            });

            set_current_theme(self.theme.resolve_theme());
            set_current_viewport(native_viewport_info(
                surface_state.size,
                window.scale_factor() as f32,
            ));
            self.router.initialize(None);
            let router = self.router.clone();
            let root_widget = signal_runtime
                .run_with_current(|| Box::new(RouterHost::new(router.clone())) as Box<dyn Widget>);

            let platform_id = PlatformId::current_native();

            self.state = Some(AppState {
                navigator: self.router.navigator(),
                platform: NativePlatform::new(platform_id),
                window,
                device,
                queue,
                surface_state,
                renderer,
                text_system,
                draw_list,
                layout_tree,
                widget_registry,
                component_states,
                focus_manager,
                focused_path: None,
                capture_path: None,
                signal_runtime,
                task_runtime,
                theme: self.theme.clone(),
                root_widget,
                start_time: Instant::now(),
                mouse_pos: glam::Vec2::ZERO,
                modifiers: Modifiers::default(),
                scale_factor: window.scale_factor() as f32,
                ime_composing: false,
                needs_layout: true,
                needs_repaint: true,
            });

            // Build initial layout
            self.build_layout();

            if let Some(state) = self.state.as_mut() {
                let activation_handler =
                    state.platform.activation_handler(self.config.title.clone());
                let adapter = accesskit_winit::Adapter::with_mixed_handlers(
                    event_loop,
                    state.window,
                    activation_handler,
                    self.event_loop_proxy.clone(),
                );
                state.platform.set_accessibility_adapter(adapter);
                state.window.set_visible(true);
                state.sync_window_metrics();
            }

            self.refresh_accessibility();
            // Trigger the very first frame; some platforms won't emit an initial redraw.
            if let Some(state) = self.state.as_ref() {
                state.window.request_redraw();
            }
        }
    }

    fn window_event(
        &mut self,
        event_loop: &winit::event_loop::ActiveEventLoop,
        _id: winit::window::WindowId,
        event: WindowEvent,
    ) {
        if let Some(state) = self.state.as_mut() {
            if let Some(adapter) = state.platform.accessibility_adapter_mut() {
                adapter.process_event(state.window, &event);
            }
        }

        match event {
            WindowEvent::CloseRequested => {
                event_loop.exit();
            }
            WindowEvent::Resized(size) => {
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
            WindowEvent::CursorMoved { position, .. } => {
                let pos = if let Some(state) = self.state.as_mut() {
                    let pos = state.platform.event_translator().cursor_position(
                        position.x as f32,
                        position.y as f32,
                        state.scale_factor,
                    );
                    state.mouse_pos = pos;
                    pos
                } else {
                    glam::Vec2::ZERO
                };
                self.handle_event(InputEvent::PointerMove { pos });
            }
            WindowEvent::MouseInput {
                state: btn_state,
                button,
                ..
            } => {
                let pos = self.state.as_ref().map(|s| s.mouse_pos).unwrap_or_default();
                let button = self
                    .state
                    .as_ref()
                    .map(|state| state.platform.event_translator().map_mouse_button(button))
                    .unwrap_or(PointerButton::Primary);

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
                let modifiers = self
                    .state
                    .as_ref()
                    .map(|state| state.modifiers)
                    .unwrap_or_default();
                self.handle_event(InputEvent::Scroll {
                    pos,
                    delta,
                    modifiers,
                });
            }
            WindowEvent::ModifiersChanged(modifiers) => {
                if let Some(state) = self.state.as_mut() {
                    state.modifiers = state
                        .platform
                        .event_translator()
                        .map_modifiers(modifiers.state());
                }
            }
            WindowEvent::KeyboardInput { event, .. } => {
                let modifiers = self
                    .state
                    .as_ref()
                    .map(|state| state.modifiers)
                    .unwrap_or_default();
                let key_without_modifiers = event.key_without_modifiers();
                let dispatch = self
                    .state
                    .as_ref()
                    .map(|state| {
                        state.platform.event_translator().translate_keyboard(
                            &key_without_modifiers.as_ref(),
                            event.state,
                            modifiers,
                            event.repeat,
                            event.text.as_deref(),
                        )
                    })
                    .unwrap_or(NativeKeyboardDispatch {
                        keyboard_event: None,
                        text_event: None,
                    });
                if let Some(keyboard_event) = dispatch.keyboard_event {
                    self.handle_event(keyboard_event);
                }
                if let Some(text_event) = dispatch.text_event {
                    self.handle_event(text_event);
                }
            }
            WindowEvent::Ime(event) => {
                let Some(state) = self.state.as_mut() else {
                    return;
                };
                let was_composing = state.ime_composing;
                let translated = state
                    .platform
                    .event_translator()
                    .translate_ime(&event, was_composing);
                match event {
                    winit::event::Ime::Preedit(_, _) => state.ime_composing = true,
                    winit::event::Ime::Commit(_) | winit::event::Ime::Disabled => {
                        state.ime_composing = false
                    }
                    winit::event::Ime::Enabled => {}
                }
                for translated_event in translated {
                    self.handle_event(translated_event);
                }
            }
            WindowEvent::Focused(focused) => {
                if focused {
                    if let Some(state) = self.state.as_ref() {
                        set_native_ime_allowed(
                            state.window,
                            state.focused_text_editor_state().is_some(),
                        );
                    }
                    self.handle_event(InputEvent::FocusGained);
                } else {
                    if let Some(state) = self.state.as_mut() {
                        state.ime_composing = false;
                        set_native_ime_allowed(state.window, false);
                    }
                    self.handle_event(InputEvent::FocusLost);
                }
            }
            WindowEvent::RedrawRequested => {
                if self.state.is_none() {
                    return;
                }

                if let Some(state) = self.state.as_mut() {
                    state.sync_window_metrics();
                }

                if self
                    .state
                    .as_ref()
                    .map(|state| state.needs_layout)
                    .unwrap_or(false)
                {
                    self.build_layout();
                }

                if self
                    .state
                    .as_ref()
                    .map(|state| state.needs_repaint)
                    .unwrap_or(false)
                {
                    self.paint();
                }

                let Some(state) = self.state.as_mut() else {
                    return;
                };

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
                            label: Some("sparsha_encoder"),
                        });

                let bg = state
                    .theme
                    .resolve_background(self.config.background_override);
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

    fn user_event(
        &mut self,
        _event_loop: &winit::event_loop::ActiveEventLoop,
        user_event: NativeUserEvent,
    ) {
        match user_event {
            NativeUserEvent::Accessibility(event) => {
                let Some(state) = self.state.as_ref() else {
                    return;
                };
                if event.window_id != state.window.id() {
                    return;
                }
                match event.window_event {
                    accesskit_winit::WindowEvent::InitialTreeRequested => {}
                    accesskit_winit::WindowEvent::ActionRequested(request) => {
                        if let Some(action) = action_from_accesskit(request) {
                            self.handle_accessibility_action(action);
                        }
                    }
                    accesskit_winit::WindowEvent::AccessibilityDeactivated => {}
                }
            }
        }
    }

    fn about_to_wait(&mut self, _event_loop: &winit::event_loop::ActiveEventLoop) {
        if let Some(state) = self.state.as_mut() {
            state.sync_window_metrics();
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

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
    use super::*;
    use sparsha_signals::{RuntimeHandle, SubscriberKind};

    #[test]
    fn app_theme_mode_resolves_light_or_dark() {
        let mut light = Theme::light();
        light.colors.primary = Color::from_hex(0x22C55E);
        let mut dark = Theme::dark();
        dark.colors.primary = Color::from_hex(0xF59E0B);

        let light_active = AppTheme {
            light: ThemeSource::Static(light.clone()),
            dark: Some(ThemeSource::Static(dark.clone())),
            mode: ThemeModeSource::Static(ThemeMode::Light),
        };
        assert_eq!(light_active.resolve_theme(), light);

        let dark_active = AppTheme {
            light: ThemeSource::Static(light.clone()),
            dark: Some(ThemeSource::Static(dark.clone())),
            mode: ThemeModeSource::Static(ThemeMode::Dark),
        };
        assert_eq!(dark_active.resolve_theme(), dark);
    }

    #[test]
    fn app_theme_dark_mode_falls_back_to_light_when_missing_dark_theme() {
        let mut light = Theme::light();
        light.colors.primary = Color::from_hex(0x6366F1);

        let app_theme = AppTheme {
            light: ThemeSource::Static(light.clone()),
            dark: None,
            mode: ThemeModeSource::Static(ThemeMode::Dark),
        };
        assert_eq!(app_theme.resolve_theme(), light);
    }

    #[test]
    fn app_theme_background_uses_theme_unless_override_is_set() {
        let mut light = Theme::light();
        light.colors.background = Color::from_hex(0x111827);
        let app_theme = AppTheme {
            light: ThemeSource::Static(light),
            dark: None,
            mode: ThemeModeSource::Static(ThemeMode::Light),
        };

        assert_eq!(
            app_theme.resolve_background(None),
            Color::from_hex(0x111827)
        );
        assert_eq!(
            app_theme.resolve_background(Some(Color::from_hex(0xFEF3C7))),
            Color::from_hex(0xFEF3C7)
        );
    }

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

    #[test]
    fn dynamic_theme_mode_source_marks_rebuild_dirty() {
        let runtime = RuntimeHandle::new();
        runtime.run_with_current(|| {
            let mode_signal = Signal::new(ThemeMode::Light);
            let source = AppTheme {
                light: ThemeSource::Static(Theme::light()),
                dark: Some(ThemeSource::Static(Theme::dark())),
                mode: ThemeModeInput::from(mode_signal.read_only()).into_source(),
            };

            runtime.with_tracking(SubscriberKind::Rebuild, || {
                let _ = source.resolve_theme();
            });

            mode_signal.set(ThemeMode::Dark);

            let dirty = runtime.take_dirty_flags();
            assert!(dirty.rebuild);
            assert!(dirty.layout);
            assert!(dirty.paint);
        });
    }

    #[test]
    fn app_run_error_wraps_graphics_init_errors() {
        let error = AppRunError::from(WgpuInitError::NoSurfaceFormat);
        assert!(matches!(
            error,
            AppRunError::GraphicsInit(WgpuInitError::NoSurfaceFormat)
        ));
    }

    #[test]
    fn app_run_error_formats_task_runtime_failures() {
        let error = AppRunError::TaskRuntimeInit("startup failed".to_owned());
        assert_eq!(
            error.to_string(),
            "failed to initialize background task runtime: startup failed"
        );
    }

    #[test]
    fn native_key_mapping_normalizes_space_to_character() {
        use sparsha_input::Key;

        let mapped = map_winit_key(&winit::keyboard::Key::Named(
            winit::keyboard::NamedKey::Space,
        ));

        assert_eq!(mapped, Some(Key::Character(" ".to_owned())));
    }

    #[test]
    fn native_text_emission_allows_plain_space() {
        assert!(should_emit_native_text(" ", Modifiers::empty()));
    }

    #[test]
    fn native_text_emission_rejects_primary_shortcuts_and_alt_text() {
        let primary_modifiers = {
            #[cfg(any(target_os = "macos", target_arch = "wasm32"))]
            {
                Modifiers::META
            }

            #[cfg(not(any(target_os = "macos", target_arch = "wasm32")))]
            {
                Modifiers::CONTROL
            }
        };

        assert!(!should_emit_native_text(" ", primary_modifiers));
        assert!(!should_emit_native_text(" ", Modifiers::ALT));
    }

    #[test]
    fn native_surface_sync_only_runs_for_non_zero_size_changes() {
        use winit::dpi::PhysicalSize;

        assert!(!should_sync_native_surface_size(
            PhysicalSize::new(1200, 900),
            PhysicalSize::new(1200, 900)
        ));
        assert!(!should_sync_native_surface_size(
            PhysicalSize::new(1, 1),
            PhysicalSize::new(0, 900)
        ));
        assert!(should_sync_native_surface_size(
            PhysicalSize::new(1, 1),
            PhysicalSize::new(1200, 900)
        ));
    }

    #[test]
    fn native_scale_factor_sync_only_runs_for_valid_changes() {
        assert!(!should_sync_native_scale_factor(2.0, 2.0));
        assert!(!should_sync_native_scale_factor(2.0, 0.0));
        assert!(!should_sync_native_scale_factor(2.0, f32::NAN));
        assert!(should_sync_native_scale_factor(2.0, 1.5));
    }

    #[test]
    fn native_viewport_info_uses_logical_window_size() {
        use sparsha_widgets::ViewportClass;
        use winit::dpi::PhysicalSize;

        let viewport = native_viewport_info(PhysicalSize::new(1600, 2400), 2.0);
        assert_eq!(viewport.width, 800.0);
        assert_eq!(viewport.height, 1200.0);
        assert_eq!(viewport.class, ViewportClass::Tablet);
    }
}
