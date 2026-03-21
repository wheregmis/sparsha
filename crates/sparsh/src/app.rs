//! Application runner and main event loop.

#[cfg(not(target_arch = "wasm32"))]
use crate::accessibility::{action_from_accesskit, AccessibilityTreeSnapshot};
use crate::router::Router;
#[cfg(not(target_arch = "wasm32"))]
use crate::router::RouterHost;
#[cfg(not(target_arch = "wasm32"))]
use crate::runtime_widget::{
    add_widget_to_layout, apply_focus_change, collect_accessibility_tree, dispatch_widget_event,
    move_focus_path, remap_path, sync_focus_manager, with_widget_mut, WidgetPath,
    WidgetRuntimeRegistry,
};
use sparsh_core::Color;
use sparsh_core::WgpuInitError;
use sparsh_signals::{ReadSignal, Signal};
use sparsh_widgets::Theme;
#[cfg(not(target_arch = "wasm32"))]
use sparsh_widgets::{set_current_theme, Widget};

#[cfg(not(target_arch = "wasm32"))]
use crate::tasks::{TaskRuntime, TaskStatus};
#[cfg(not(target_arch = "wasm32"))]
use sparsh_core::{init_wgpu, SurfaceState};
#[cfg(not(target_arch = "wasm32"))]
use sparsh_input::{
    Action, ActionMapper, FocusManager, InputEvent, Modifiers, PointerButton, StandardAction,
};
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
use sparsh_widgets::{BuildContext, PaintCommands, PaintContext};
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
            title: String::from("Sparsh App"),
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
    window: &'static winit::window::Window,
    device: Device,
    queue: Queue,
    surface_state: SurfaceState<'static>,
    renderer: Renderer,
    text_system: TextSystem,
    draw_list: DrawList,
    layout_tree: LayoutTree,
    widget_registry: WidgetRuntimeRegistry,
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
    clipboard: Option<arboard::Clipboard>,
    ime_composing: bool,
    accessibility_snapshot: Arc<Mutex<AccessibilityTreeSnapshot>>,
    accessibility_adapter: Option<accesskit_winit::Adapter>,
    needs_layout: bool,
    needs_repaint: bool,
}

#[cfg(not(target_arch = "wasm32"))]
impl AppState {
    fn focused_text_editor_state(&self) -> Option<&sparsh_widgets::TextEditorState> {
        self.focused_path
            .as_ref()
            .and_then(|path| self.widget_registry.text_editor_state_for_path(path))
    }

    fn set_clipboard_text(&mut self, text: &str) {
        let Some(clipboard) = self.clipboard.as_mut() else {
            return;
        };
        if let Err(err) = clipboard.set_text(text.to_owned()) {
            log::warn!("failed to write clipboard text: {err}");
        }
    }

    fn clipboard_text(&mut self) -> Option<String> {
        let clipboard = self.clipboard.as_mut()?;
        match clipboard.get_text() {
            Ok(text) => Some(text),
            Err(err) => {
                log::warn!("failed to read clipboard text: {err}");
                None
            }
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
struct NativeAccessibilityActivationHandler {
    title: String,
    snapshot: Arc<Mutex<AccessibilityTreeSnapshot>>,
}

#[cfg(not(target_arch = "wasm32"))]
impl accesskit::ActivationHandler for NativeAccessibilityActivationHandler {
    fn request_initial_tree(&mut self) -> Option<accesskit::TreeUpdate> {
        let snapshot = match self.snapshot.lock() {
            Ok(guard) => guard.clone(),
            Err(poisoned) => {
                log::warn!("recovering from poisoned accessibility snapshot");
                poisoned.into_inner().clone()
            }
        };
        Some(snapshot.to_tree_update(&self.title))
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

        state.widget_registry.accessibility = collect_accessibility_tree(
            state.root_widget.as_ref(),
            &state.layout_tree,
            state.focused_path.as_ref(),
        );
        let snapshot = state.widget_registry.accessibility_tree().clone();
        match state.accessibility_snapshot.lock() {
            Ok(mut guard) => {
                *guard = snapshot.clone();
            }
            Err(poisoned) => {
                log::warn!("recovering from poisoned accessibility snapshot");
                *poisoned.into_inner() = snapshot.clone();
            }
        }

        if let Some(adapter) = state.accessibility_adapter.as_mut() {
            let title = self.config.title.clone();
            adapter.update_if_active(|| snapshot.to_tree_update(&title));
        }
    }

    fn handle_accessibility_action(
        &mut self,
        request: crate::accessibility::RoutedAccessibilityAction,
    ) {
        {
            let Some(state) = self.state.as_mut() else {
                return;
            };

            let Some(path) = state
                .widget_registry
                .path_for_accessibility_node(request.node_id)
                .map(ToOwned::to_owned)
            else {
                return;
            };

            match request.action {
                sparsh_widgets::AccessibilityAction::Focus => {
                    let focus_changed = apply_focus_change(
                        state.root_widget.as_mut(),
                        &mut state.focus_manager,
                        &state.widget_registry,
                        &mut state.focused_path,
                        Some(path),
                    );
                    if focus_changed {
                        set_native_ime_allowed(
                            state.window,
                            state.focused_text_editor_state().is_some(),
                        );
                        state.needs_repaint = true;
                    }
                }
                action => {
                    let handled = with_widget_mut(state.root_widget.as_mut(), &path, |widget| {
                        widget.handle_accessibility_action(action, request.value.clone())
                    })
                    .unwrap_or(false);
                    if handled {
                        if matches!(action, sparsh_widgets::AccessibilityAction::SetValue) {
                            state.needs_layout = true;
                        }
                        state.needs_repaint = true;
                    }
                }
            }

            state.signal_runtime.run_effects(64);
            let dirty = state.signal_runtime.take_dirty_flags();
            if dirty.rebuild || dirty.layout {
                state.needs_layout = true;
            }
            if dirty.paint {
                state.needs_repaint = true;
            }
        }
        self.refresh_accessibility();
        if let Some(state) = self.state.as_ref() {
            if state.needs_layout || state.needs_repaint {
                state.window.request_redraw();
            }
        }
    }

    fn build_layout(&mut self) {
        {
            let Some(state) = self.state.as_mut() else {
                return;
            };
            let runtime = state.signal_runtime.clone();

            state.layout_tree = LayoutTree::new();

            runtime.with_tracking(SubscriberKind::Rebuild, || {
                set_current_theme(state.theme.resolve_theme());

                fn rebuild_widget(widget: &mut dyn Widget, build_ctx: &mut BuildContext) {
                    widget.rebuild(build_ctx);
                    for child in widget.children_mut() {
                        rebuild_widget(child.as_mut(), build_ctx);
                    }
                }

                let mut build_ctx = BuildContext::default();
                rebuild_widget(state.root_widget.as_mut(), &mut build_ctx);
            });

            let mut widget_registry = WidgetRuntimeRegistry::default();
            let root_id = runtime.with_tracking(SubscriberKind::Layout, || {
                set_current_theme(state.theme.resolve_theme());
                let mut path = Vec::new();
                add_widget_to_layout(
                    state.root_widget.as_mut(),
                    &mut state.layout_tree,
                    &mut state.text_system,
                    &mut widget_registry,
                    &mut path,
                    false,
                )
            });
            state.layout_tree.set_root(root_id);
            state.widget_registry = widget_registry;

            let size = state.surface_state.size;
            let logical_width = (size.width as f32) / state.scale_factor;
            let logical_height = (size.height as f32) / state.scale_factor;
            state
                .layout_tree
                .compute_layout(logical_width, logical_height);

            state.focused_path = remap_path(state.focused_path.take(), &state.widget_registry);
            state.capture_path = remap_path(state.capture_path.take(), &state.widget_registry);
            sync_focus_manager(
                &mut state.focus_manager,
                &state.widget_registry,
                state.focused_path.as_ref(),
            );
            set_native_ime_allowed(state.window, state.focused_text_editor_state().is_some());

            state.needs_layout = false;
            state.needs_repaint = true;
        }
        self.refresh_accessibility();
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
            set_current_theme(state.theme.resolve_theme());
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
        let mut handled_focus_navigation = false;
        {
            let Some(state) = self.state.as_mut() else {
                return;
            };

            let runtime = state.signal_runtime.clone();
            let mapper = ActionMapper::new();
            let mut dispatch_event = event.clone();

            if let Some(Action::Standard(action)) = mapper.map_event(&event) {
                match action {
                    StandardAction::FocusNext | StandardAction::FocusPrevious => {
                        let next_focus = move_focus_path(
                            state.focused_path.as_ref(),
                            &state.widget_registry,
                            matches!(action, StandardAction::FocusNext),
                        );
                        let focus_changed = apply_focus_change(
                            state.root_widget.as_mut(),
                            &mut state.focus_manager,
                            &state.widget_registry,
                            &mut state.focused_path,
                            next_focus,
                        );
                        if focus_changed {
                            set_native_ime_allowed(
                                state.window,
                                state.focused_text_editor_state().is_some(),
                            );
                            state.needs_repaint = true;
                        }
                        handled_focus_navigation = true;
                    }
                    StandardAction::Paste if state.focused_text_editor_state().is_some() => {
                        let Some(text) = state.clipboard_text() else {
                            return;
                        };
                        dispatch_event = InputEvent::Paste { text };
                    }
                    _ => {}
                }
            }

            if !handled_focus_navigation {
                let current_focus_id = state
                    .focused_path
                    .as_ref()
                    .and_then(|path| state.widget_registry.id_for_path(path));
                let current_capture_path = state.capture_path.clone();
                let outcome = runtime.run_with_current(|| {
                    dispatch_widget_event(
                        state.root_widget.as_mut(),
                        &state.layout_tree,
                        current_focus_id,
                        current_capture_path.as_ref(),
                        &dispatch_event,
                    )
                });

                if outcome.commands.request_focus || outcome.commands.clear_focus {
                    let focus_changed = apply_focus_change(
                        state.root_widget.as_mut(),
                        &mut state.focus_manager,
                        &state.widget_registry,
                        &mut state.focused_path,
                        outcome.focus_path.clone(),
                    );
                    if focus_changed {
                        set_native_ime_allowed(
                            state.window,
                            state.focused_text_editor_state().is_some(),
                        );
                        if state.focused_text_editor_state().is_none() {
                            state.ime_composing = false;
                        }
                        state.needs_repaint = true;
                    }
                }

                if outcome.commands.capture_pointer || outcome.commands.release_pointer {
                    state.capture_path = outcome.capture_path;
                    state.needs_repaint = true;
                }

                if let Some(text) = outcome.commands.clipboard_write.as_deref() {
                    state.set_clipboard_text(text);
                }

                if outcome.commands.request_paint {
                    state.needs_repaint = true;
                }
                if outcome.commands.request_layout {
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
            }
        }
        if handled_focus_navigation {
            self.refresh_accessibility();
            if let Some(state) = self.state.as_ref() {
                if state.needs_repaint || state.needs_layout {
                    state.window.request_redraw();
                }
            }
            return;
        }
        self.refresh_accessibility();
        if let Some(state) = self.state.as_ref() {
            if state.needs_repaint || state.needs_layout {
                state.window.request_redraw();
            }
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn map_winit_key(key: &winit::keyboard::Key<&str>) -> Option<sparsh_input::Key> {
    use sparsh_input::{Key, NamedKey};

    Some(match key {
        winit::keyboard::Key::Character(value) => Key::Character(value.to_string()),
        winit::keyboard::Key::Named(named) => Key::Named(match named {
            winit::keyboard::NamedKey::Enter => NamedKey::Enter,
            winit::keyboard::NamedKey::Tab => NamedKey::Tab,
            winit::keyboard::NamedKey::Backspace => NamedKey::Backspace,
            winit::keyboard::NamedKey::Delete => NamedKey::Delete,
            winit::keyboard::NamedKey::Escape => NamedKey::Escape,
            winit::keyboard::NamedKey::ArrowUp => NamedKey::ArrowUp,
            winit::keyboard::NamedKey::ArrowDown => NamedKey::ArrowDown,
            winit::keyboard::NamedKey::ArrowLeft => NamedKey::ArrowLeft,
            winit::keyboard::NamedKey::ArrowRight => NamedKey::ArrowRight,
            winit::keyboard::NamedKey::Home => NamedKey::Home,
            winit::keyboard::NamedKey::End => NamedKey::End,
            winit::keyboard::NamedKey::PageUp => NamedKey::PageUp,
            winit::keyboard::NamedKey::PageDown => NamedKey::PageDown,
            _ => return None,
        }),
        _ => return None,
    })
}

#[cfg(not(target_arch = "wasm32"))]
fn winit_modifiers_to_input(modifiers: winit::keyboard::ModifiersState) -> Modifiers {
    let mut converted = Modifiers::empty();
    if modifiers.shift_key() {
        converted |= Modifiers::SHIFT;
    }
    if modifiers.control_key() {
        converted |= Modifiers::CONTROL;
    }
    if modifiers.alt_key() {
        converted |= Modifiers::ALT;
    }
    if modifiers.super_key() {
        converted |= Modifiers::META;
    }
    converted
}

#[cfg(not(target_arch = "wasm32"))]
fn should_emit_native_text(text: &str, modifiers: Modifiers) -> bool {
    !text.is_empty()
        && text.chars().all(|ch| !ch.is_control())
        && !sparsh_input::shortcuts::primary_modifier(modifiers)
        && !modifiers.alt()
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
            self.router.initialize(None);
            let router = self.router.clone();
            let root_widget = signal_runtime
                .run_with_current(|| Box::new(RouterHost::new(router.clone())) as Box<dyn Widget>);

            let scale_factor = window.scale_factor() as f32;
            let clipboard = match arboard::Clipboard::new() {
                Ok(clipboard) => Some(clipboard),
                Err(err) => {
                    log::warn!("native clipboard unavailable: {err}");
                    None
                }
            };

            self.state = Some(AppState {
                window,
                device,
                queue,
                surface_state,
                renderer,
                text_system,
                draw_list,
                layout_tree,
                widget_registry,
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
                scale_factor,
                clipboard,
                ime_composing: false,
                accessibility_snapshot: Arc::new(Mutex::new(AccessibilityTreeSnapshot::default())),
                accessibility_adapter: None,
                needs_layout: true,
                needs_repaint: true,
            });

            // Build initial layout
            self.build_layout();

            if let Some(state) = self.state.as_mut() {
                let activation_handler = NativeAccessibilityActivationHandler {
                    title: self.config.title.clone(),
                    snapshot: Arc::clone(&state.accessibility_snapshot),
                };
                let adapter = accesskit_winit::Adapter::with_mixed_handlers(
                    event_loop,
                    state.window,
                    activation_handler,
                    self.event_loop_proxy.clone(),
                );
                state.accessibility_adapter = Some(adapter);
                state.window.set_visible(true);
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
            if let Some(adapter) = state.accessibility_adapter.as_mut() {
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
            WindowEvent::MouseInput {
                state: btn_state,
                button,
                ..
            } => {
                let pos = self.state.as_ref().map(|s| s.mouse_pos).unwrap_or_default();
                let button = match button {
                    winit::event::MouseButton::Left => PointerButton::Primary,
                    winit::event::MouseButton::Right => PointerButton::Secondary,
                    winit::event::MouseButton::Middle => PointerButton::Auxiliary,
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
                    state.modifiers = winit_modifiers_to_input(modifiers.state());
                }
            }
            WindowEvent::KeyboardInput { event, .. } => {
                use sparsh_input::{ui_events::keyboard::Code, KeyboardEvent};

                let modifiers = self
                    .state
                    .as_ref()
                    .map(|state| state.modifiers)
                    .unwrap_or_default();
                let key_without_modifiers = event.key_without_modifiers();
                let Some(key) = map_winit_key(&key_without_modifiers.as_ref()) else {
                    return;
                };

                let mut kb_event = if event.state.is_pressed() {
                    KeyboardEvent::key_down(key, Code::Unidentified)
                } else {
                    KeyboardEvent::key_up(key, Code::Unidentified)
                };
                kb_event.modifiers = modifiers;
                kb_event.repeat = event.repeat;

                if event.state.is_pressed() {
                    self.handle_event(InputEvent::KeyDown {
                        event: kb_event.clone(),
                    });
                } else {
                    self.handle_event(InputEvent::KeyUp { event: kb_event });
                }

                if event.state.is_pressed() && !event.repeat {
                    if let Some(text) = event.text.as_ref() {
                        let text = text.to_string();
                        if should_emit_native_text(&text, modifiers) {
                            self.handle_event(InputEvent::TextInput { text });
                        }
                    }
                }
            }
            WindowEvent::Ime(event) => {
                let Some(state) = self.state.as_mut() else {
                    return;
                };
                match event {
                    winit::event::Ime::Enabled => {}
                    winit::event::Ime::Preedit(text, _) => {
                        if !state.ime_composing {
                            state.ime_composing = true;
                            self.handle_event(InputEvent::CompositionStart);
                        }
                        self.handle_event(InputEvent::CompositionUpdate { text });
                    }
                    winit::event::Ime::Commit(text) => {
                        state.ime_composing = false;
                        self.handle_event(InputEvent::CompositionEnd { text });
                    }
                    winit::event::Ime::Disabled => {
                        if state.ime_composing {
                            state.ime_composing = false;
                            self.handle_event(InputEvent::CompositionEnd {
                                text: String::new(),
                            });
                        }
                    }
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
                            label: Some("sparsh_encoder"),
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
}
