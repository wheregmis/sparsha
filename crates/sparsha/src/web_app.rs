//! Web runtime that renders Sparsha draw commands into the DOM.

#![cfg(target_arch = "wasm32")]

use crate::tasks::{TaskRuntime, TaskStatus};
use crate::{
    accessibility::AccessibilityTreeSnapshot,
    app::{AppConfig, AppRunError, AppTheme},
    component::ComponentStateStore,
    dom_renderer::{DomFrameSnapshot, DomRenderer},
    platform::events::WebEventTranslator,
    platform::WebPlatform,
    router::{hash_to_path, path_to_hash, Navigator, Router, RouterHost},
    runtime_core::{focused_text_editor_state, RuntimeCoreContext, RuntimeHost},
    runtime_widget::{WidgetPath, WidgetRuntimeRegistry},
    web_surface_manager::{HybridSurfaceManager, HybridSurfaceStatus, SurfaceFrame},
};
use sparsha_core::Color;
use sparsha_input::{FocusManager, InputEvent, Modifiers, PointerButton, StandardAction};
use sparsha_layout::LayoutTree;
use sparsha_render::DrawList;
use sparsha_signals::{RuntimeHandle, SubscriberKind};
use sparsha_text::TextSystem;
use sparsha_widgets::{
    set_current_theme, set_current_viewport, AccessibilityAction, PaintCommands, PaintContext,
    TextEditorState, ViewportInfo, Widget,
};
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};
use wasm_bindgen::{closure::Closure, JsCast};
use web_sys::{
    ClipboardEvent, CompositionEvent as WebCompositionEvent, CustomEvent, Element, Event,
    HtmlElement, HtmlInputElement, HtmlTextAreaElement, InputEvent as WebInputEvent,
    KeyboardEvent as WebKeyboardEvent, MouseEvent, Touch, TouchEvent, TouchList, WheelEvent,
    Window,
};

#[wasm_bindgen::prelude::wasm_bindgen(
    inline_js = r#"
export function startRouteViewTransition(document) {
  const startViewTransition = document?.startViewTransition;
  if (typeof startViewTransition !== "function") {
    return;
  }
  try {
    // The render/update happens in Rust; this callback intentionally no-ops.
    startViewTransition.call(document, () => {});
  } catch (_) {
    // Ignore unsupported and timing-related failures.
  }
}
"#
)]
extern "C" {
    fn startRouteViewTransition(document: &web_sys::Document);
}

fn format_js_error(error: &wasm_bindgen::JsValue) -> String {
    error.as_string().unwrap_or_else(|| format!("{error:?}"))
}

const WEB_SCROLL_DELTA_UNIT: f32 = 20.0;

pub(crate) fn run_dom_app(
    config: AppConfig,
    theme: AppTheme,
    router: Router,
) -> Result<(), AppRunError> {
    let window = web_sys::window().ok_or(AppRunError::WebEnvironment("window"))?;
    let document = window
        .document()
        .ok_or(AppRunError::WebEnvironment("document"))?;
    document.set_title(&config.title);
    let dom_renderer = DomRenderer::mount_to_body(&document)
        .map_err(|err| AppRunError::DomMount(format_js_error(&err)))?;
    let surface_manager = HybridSurfaceManager::new(dom_renderer.root())
        .map_err(|err| AppRunError::HybridSurfaceInit(format_js_error(&err)))?;
    let platform = WebPlatform::new(&document, dom_renderer.root())
        .map_err(|err| AppRunError::DomMount(format_js_error(&err)))?;
    let signal_runtime = RuntimeHandle::current_or_default();
    let task_runtime =
        TaskRuntime::try_new().map_err(|err| AppRunError::TaskRuntimeInit(err.to_string()))?;
    task_runtime.set_worker_script_url("sparsha-worker.js?v=2");
    task_runtime.set_current();
    let navigator = router.navigator();
    let initial_path = window
        .location()
        .hash()
        .ok()
        .map(|hash| hash_to_path(&hash));
    set_current_theme(theme.resolve_theme());
    set_current_viewport(ViewportInfo::new(config.width as f32, config.height as f32));
    router.initialize(initial_path.as_deref());
    let router_for_build = router.clone();
    let root_widget = signal_runtime.run_with_current(|| {
        Box::new(RouterHost::new(router_for_build.clone())) as Box<dyn Widget>
    });

    let initial_route_path = navigator.current_path();
    let mut state = WebAppState {
        config,
        theme,
        platform,
        router_navigator: navigator,
        dom_renderer,
        text_system: TextSystem::new_headless(),
        draw_list: DrawList::new(),
        surface_frames: Vec::new(),
        layout_tree: LayoutTree::new(),
        widget_registry: WidgetRuntimeRegistry::default(),
        component_states: ComponentStateStore::default(),
        focus_manager: FocusManager::new(),
        focused_path: None,
        capture_path: None,
        signal_runtime,
        task_runtime,
        root_widget,
        start_time: web_time::Instant::now(),
        mouse_pos: glam::Vec2::ZERO,
        active_touch_id: None,
        scale_factor: 1.0,
        viewport_width: 0.0,
        viewport_height: 0.0,
        needs_layout: true,
        needs_repaint: true,
        first_paint_emitted: false,
        surface_manager,
        ime_composing: false,
        pending_surface_retry: false,
        last_route_path: initial_route_path,
    };
    state.update_viewport();

    let state = Rc::new(RefCell::new(state));
    let frame_cb: Rc<RefCell<Option<Closure<dyn FnMut(f64)>>>> = Rc::new(RefCell::new(None));
    let pending_animation_frame = Rc::new(Cell::new(false));
    {
        let signal_runtime = state.borrow().signal_runtime.clone();
        let window_for_scheduler = window.clone();
        let frame_cb = Rc::clone(&frame_cb);
        let pending_animation_frame = Rc::clone(&pending_animation_frame);
        signal_runtime.set_scheduler(move || {
            schedule_animation_frame(&window_for_scheduler, &pending_animation_frame, &frame_cb);
        });
    }
    install_event_listeners(&window, &state, &pending_animation_frame, &frame_cb);
    start_animation_loop(&window, &state, &pending_animation_frame, &frame_cb);
    Ok(())
}

struct WebAppState {
    config: AppConfig,
    theme: AppTheme,
    platform: WebPlatform,
    router_navigator: Navigator,
    dom_renderer: DomRenderer,
    text_system: TextSystem,
    draw_list: DrawList,
    surface_frames: Vec<SurfaceFrame>,
    layout_tree: LayoutTree,
    widget_registry: WidgetRuntimeRegistry,
    component_states: ComponentStateStore,
    focus_manager: FocusManager,
    focused_path: Option<WidgetPath>,
    capture_path: Option<WidgetPath>,
    signal_runtime: RuntimeHandle,
    task_runtime: TaskRuntime,
    root_widget: Box<dyn Widget>,
    start_time: web_time::Instant,
    mouse_pos: glam::Vec2,
    active_touch_id: Option<i32>,
    scale_factor: f32,
    viewport_width: f32,
    viewport_height: f32,
    needs_layout: bool,
    needs_repaint: bool,
    first_paint_emitted: bool,
    surface_manager: HybridSurfaceManager,
    ime_composing: bool,
    pending_surface_retry: bool,
    last_route_path: String,
}

struct WebFrameSnapshot<'a> {
    draw_list: &'a DrawList,
    surface_frames: &'a [SurfaceFrame],
    background: Color,
    viewport_width: f32,
    viewport_height: f32,
    accessibility: AccessibilityTreeSnapshot,
}

fn paint_widget_subtree(
    widget: &dyn Widget,
    layout_tree: &LayoutTree,
    focus: &FocusManager,
    draw_list: &mut DrawList,
    surface_frames: &mut Vec<SurfaceFrame>,
    scale_factor: f32,
    text_system_ptr: *mut TextSystem,
    elapsed_time: f32,
    paint_commands: &mut PaintCommands,
) {
    let id = widget.id();
    if let Some(layout) = layout_tree.get_absolute_layout(id) {
        if let Some(surface) = widget.draw_surface() {
            let mut local_commands = PaintCommands::default();
            let mut surface_draw_list = DrawList::new();
            let mut surface_ctx = sparsha_widgets::DrawSurfaceContext {
                draw_list: &mut surface_draw_list,
                bounds: sparsha_core::Rect::new(
                    0.0,
                    0.0,
                    layout.bounds.width * scale_factor,
                    layout.bounds.height * scale_factor,
                ),
                scale_factor,
                elapsed_time,
                commands: &mut local_commands,
            };
            surface.scene(&mut surface_ctx);
            surface_frames.push(SurfaceFrame {
                css_bounds: layout.bounds,
                scale_factor,
                elapsed_time,
                draw_list: surface_draw_list,
            });
            paint_commands.merge(local_commands);
        }

        let text_system = unsafe { &mut *text_system_ptr };
        let mut local_commands = PaintCommands::default();
        {
            let mut ctx = PaintContext {
                draw_list,
                layout,
                layout_tree,
                focus,
                widget_id: id,
                scale_factor: 1.0,
                text_system,
                elapsed_time,
                commands: &mut local_commands,
            };
            widget.paint(&mut ctx);
        }
        for child in widget.children() {
            paint_widget_subtree(
                child.as_ref(),
                layout_tree,
                focus,
                draw_list,
                surface_frames,
                scale_factor,
                text_system_ptr,
                elapsed_time,
                &mut local_commands,
            );
        }
        let text_system = unsafe { &mut *text_system_ptr };
        let mut ctx = PaintContext {
            draw_list,
            layout,
            layout_tree,
            focus,
            widget_id: id,
            scale_factor: 1.0,
            text_system,
            elapsed_time,
            commands: &mut local_commands,
        };
        widget.paint_after_children(&mut ctx);
        paint_commands.merge(local_commands);
    }
}

impl WebAppState {
    fn logical_viewport(&self) -> ViewportInfo {
        web_viewport_info(self.viewport_width, self.viewport_height)
    }

    fn runtime_host(&mut self) -> RuntimeHost<'_> {
        let viewport = self.logical_viewport();
        let shortcut_profile = self.platform.shortcut_profile();
        RuntimeHost::from(RuntimeCoreContext {
            theme: &self.theme,
            navigator: self.router_navigator.clone(),
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

    fn focused_text_editor_state(&self) -> Option<&sparsha_widgets::TextEditorState> {
        focused_text_editor_state(&self.widget_registry, self.focused_path.as_deref())
    }

    fn event_translator(&self) -> &crate::platform::events::WebEventTranslator {
        self.platform.event_translator()
    }

    fn semantic_root(&self) -> HtmlElement {
        self.platform.semantic_root().clone()
    }

    fn text_input_element(&self) -> HtmlTextAreaElement {
        self.platform.text_input_element().clone()
    }

    fn text_input_is_syncing(&self) -> bool {
        self.platform.text_input_is_syncing()
    }

    fn set_accessibility_text_focus_node(&mut self, node_id: Option<u64>) {
        self.platform.set_accessibility_text_focus_node(node_id);
    }

    fn take_pending_clipboard_write(&mut self) -> Option<String> {
        self.platform.take_pending_clipboard_write()
    }

    fn focused_selection_text(&self) -> Option<String> {
        let state = self.focused_text_editor_state()?;
        let (start, end) = state.selection_range();
        (start < end).then(|| state.text.get(start..end).unwrap_or_default().to_owned())
    }

    #[allow(dead_code)]
    fn refresh_accessibility(&mut self) {
        let _ = self.runtime_host().refresh_accessibility();
    }

    fn accessibility_text_focus_matches_widget_focus(&self) -> bool {
        self.platform.accessibility_text_focus_matches_widget_focus(
            &self.widget_registry,
            self.focused_path.as_deref(),
        )
    }

    fn sync_text_input_bridge(&mut self) {
        let suppress_text_bridge = self.accessibility_text_focus_matches_widget_focus();
        let editor_state = self.focused_text_editor_state().cloned();
        self.sync_text_input_bridge_with_state(editor_state.as_ref(), suppress_text_bridge);
    }

    fn sync_text_input_bridge_with_state(
        &mut self,
        editor_state: Option<&TextEditorState>,
        suppress_text_bridge: bool,
    ) {
        self.platform
            .sync_text_input_bridge(editor_state, suppress_text_bridge);
    }

    fn handle_accessibility_action(
        &mut self,
        node_id: u64,
        action: AccessibilityAction,
        value: Option<String>,
    ) {
        let (effects, focused_editor_state, has_capture) = {
            let mut host = self.runtime_host();
            let effects = host.handle_accessibility_action(node_id, action, value);
            let _ = host.refresh_accessibility();
            let focused_editor_state = host.focused_text_editor_state().cloned();
            let has_capture = host.has_pointer_capture();
            (effects, focused_editor_state, has_capture)
        };
        let suppress_bridge = self.accessibility_text_focus_matches_widget_focus();
        self.platform.apply_effects(
            &effects,
            focused_editor_state.as_ref(),
            has_capture,
            suppress_bridge,
        );
    }

    fn update_viewport(&mut self) {
        if let Some(window) = web_sys::window() {
            if let Ok(width) = window.inner_width() {
                self.viewport_width = width.as_f64().unwrap_or(self.config.width as f64) as f32;
            }
            if let Ok(height) = window.inner_height() {
                self.viewport_height = height.as_f64().unwrap_or(self.config.height as f64) as f32;
            }
            self.scale_factor = window.device_pixel_ratio() as f32;
        }
    }

    fn emit_first_paint_event(&mut self) {
        if self.first_paint_emitted {
            return;
        }
        self.first_paint_emitted = true;

        let Some(window) = web_sys::window() else {
            return;
        };

        match CustomEvent::new("SparshaFirstPaint") {
            Ok(event) => {
                let _ = window.dispatch_event(event.as_ref());
            }
            Err(err) => {
                log::warn!("failed to emit SparshaFirstPaint event: {:?}", err);
            }
        }
    }

    fn shortcut_profile(&self) -> sparsha_input::ShortcutProfile {
        self.platform.shortcut_profile()
    }

    fn sync_route_hash(&self, desired_hash: &str) {
        let Some(window) = web_sys::window() else {
            return;
        };

        let current_hash = window.location().hash().ok().unwrap_or_default();
        if current_hash == desired_hash {
            return;
        }

        let next_hash = desired_hash.trim_start_matches('#');
        let _ = window.location().set_hash(next_hash);
    }

    fn build_layout(&mut self) {
        let (effects, focused_editor_state, has_capture) = {
            let mut host = self.runtime_host();
            let effects = host.build_layout();
            let _ = host.refresh_accessibility();
            let focused_editor_state = host.focused_text_editor_state().cloned();
            let has_capture = host.has_pointer_capture();
            (effects, focused_editor_state, has_capture)
        };
        let suppress_bridge = self.accessibility_text_focus_matches_widget_focus();
        self.platform.apply_effects(
            &effects,
            focused_editor_state.as_ref(),
            has_capture,
            suppress_bridge,
        );
    }

    fn paint(&mut self) {
        let runtime = self.signal_runtime.clone();
        self.draw_list.clear();
        self.surface_frames.clear();
        let elapsed_time = self.start_time.elapsed().as_secs_f32();
        let text_system_ptr = &mut self.text_system as *mut TextSystem;
        let mut paint_commands = PaintCommands::default();

        runtime.with_tracking(SubscriberKind::Paint, || {
            set_current_theme(self.theme.resolve_theme());
            set_current_viewport(self.logical_viewport());
            paint_widget_subtree(
                self.root_widget.as_ref(),
                &self.layout_tree,
                &self.focus_manager,
                &mut self.draw_list,
                &mut self.surface_frames,
                self.scale_factor,
                text_system_ptr,
                elapsed_time,
                &mut paint_commands,
            );
        });
        self.needs_layout |= paint_commands.request_layout;
        self.needs_repaint = paint_commands.request_next_frame || paint_commands.request_layout;
    }

    fn handle_event(&mut self, event: InputEvent) {
        let (effects, focused_editor_state, has_capture) = {
            let mut host = self.runtime_host();
            let effects = host.handle_input_event(event, None);
            let _ = host.refresh_accessibility();
            let focused_editor_state = host.focused_text_editor_state().cloned();
            let has_capture = host.has_pointer_capture();
            (effects, focused_editor_state, has_capture)
        };
        let suppress_bridge = self.accessibility_text_focus_matches_widget_focus();
        self.platform.apply_effects(
            &effects,
            focused_editor_state.as_ref(),
            has_capture,
            suppress_bridge,
        );
    }

    fn frame(&mut self) {
        let current_route_path = self.router_navigator.current_path();
        let desired_hash = path_to_hash(&current_route_path);
        let route_changed = current_route_path != self.last_route_path;
        let mut had_task_results = false;
        self.task_runtime.drain_completed(|result| {
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

        self.signal_runtime.run_effects(64);
        let dirty = self.signal_runtime.take_dirty_flags();
        if dirty.rebuild || dirty.layout {
            self.needs_layout = true;
        }
        if dirty.paint {
            self.needs_repaint = true;
        }
        if had_task_results {
            self.needs_repaint = true;
        }

        if self.needs_layout {
            self.build_layout();
        }
        let painted_frame = if self.needs_repaint {
            self.paint();
            true
        } else {
            false
        };

        let should_render_layers = should_render_web_layers(
            painted_frame,
            self.pending_surface_retry,
            !self.surface_frames.is_empty(),
            self.surface_manager.status(),
        );
        if !should_render_layers {
            self.sync_route_hash(&desired_hash);
            self.last_route_path = current_route_path;
            return;
        }

        let focused_editor_state = self.focused_text_editor_state().cloned();
        let suppress_text_bridge = self.accessibility_text_focus_matches_widget_focus();
        self.sync_route_hash(&desired_hash);
        self.sync_text_input_bridge_with_state(focused_editor_state.as_ref(), suppress_text_bridge);

        let (dom_rendered, pending_surface_retry) = {
            if can_start_route_view_transition(route_changed, self.first_paint_emitted) {
                start_document_view_transition();
            }

            let snapshot = WebFrameSnapshot {
                draw_list: &self.draw_list,
                surface_frames: &self.surface_frames,
                background: self
                    .theme
                    .resolve_background(self.config.background_override),
                viewport_width: self.viewport_width,
                viewport_height: self.viewport_height,
                accessibility: self.widget_registry.accessibility_tree().clone(),
            };

            let mut dom_rendered = false;
            if let Err(err) = self.dom_renderer.render(&DomFrameSnapshot {
                draw_list: snapshot.draw_list,
                background: snapshot.background,
                viewport_width: snapshot.viewport_width,
                viewport_height: snapshot.viewport_height,
            }) {
                log::error!("dom render failed: {:?}", err);
            } else {
                dom_rendered = true;
            }

            if let Err(err) = self.platform.render_semantic_dom(&snapshot.accessibility) {
                log::error!("semantic dom render failed: {:?}", err);
                dom_rendered = false;
            }

            let pending_surface_retry = match self
                .surface_manager
                .render(snapshot.surface_frames, Color::TRANSPARENT)
            {
                Ok(surface_outcome) => {
                    surface_outcome.needs_retry
                        || (!snapshot.surface_frames.is_empty()
                            && matches!(
                                self.surface_manager.status(),
                                HybridSurfaceStatus::Initializing
                            ))
                }
                Err(err) => {
                    log::error!("hybrid surface render failed: {:?}", err);
                    false
                }
            };

            (dom_rendered, pending_surface_retry)
        };

        self.pending_surface_retry = pending_surface_retry;
        if dom_rendered {
            log::trace!(
                "dom frame: active_nodes={} mutated_nodes={}",
                self.dom_renderer.active_node_count(),
                self.dom_renderer.mutated_node_count()
            );
            self.emit_first_paint_event();
        }

        if !dom_rendered {
            self.pending_surface_retry = false;
        }

        self.last_route_path = current_route_path;
        self.needs_repaint |= self.pending_surface_retry;
    }

    fn should_schedule_frame(&self) -> bool {
        self.needs_layout
            || self.needs_repaint
            || self.pending_surface_retry
            || (!self.surface_frames.is_empty()
                && matches!(
                    self.surface_manager.status(),
                    HybridSurfaceStatus::Initializing
                ))
            || self.task_runtime.has_in_flight()
    }
}

fn start_animation_loop(
    window: &Window,
    state: &Rc<RefCell<WebAppState>>,
    pending_animation_frame: &Rc<Cell<bool>>,
    frame_cb: &Rc<RefCell<Option<Closure<dyn FnMut(f64)>>>>,
) {
    let frame_cb_clone = Rc::clone(frame_cb);
    let window_for_loop = window.clone();
    let state = Rc::clone(state);
    let state_for_callback = Rc::clone(&state);
    let frame_cb_for_callback = Rc::clone(frame_cb);
    let pending_animation_frame_for_callback = Rc::clone(pending_animation_frame);

    *frame_cb_clone.borrow_mut() = Some(Closure::wrap(Box::new(move |_ts: f64| {
        pending_animation_frame_for_callback.set(false);
        {
            let mut state = state_for_callback.borrow_mut();
            state.frame();
        }
        if state_for_callback.borrow().should_schedule_frame() {
            schedule_animation_frame(
                &window_for_loop,
                &pending_animation_frame_for_callback,
                &frame_cb_for_callback,
            );
        }
    }) as Box<dyn FnMut(f64)>));

    schedule_animation_frame(window, pending_animation_frame, frame_cb);
}

fn schedule_animation_frame(
    window: &Window,
    pending_animation_frame: &Rc<Cell<bool>>,
    frame_cb: &Rc<RefCell<Option<Closure<dyn FnMut(f64)>>>>,
) {
    if pending_animation_frame.get() {
        return;
    }
    pending_animation_frame.set(true);

    let cb_ref = frame_cb.borrow();
    let Some(cb) = cb_ref.as_ref() else {
        pending_animation_frame.set(false);
        log::warn!("animation callback requested before initialization");
        return;
    };
    if let Err(err) = window.request_animation_frame(cb.as_ref().unchecked_ref()) {
        pending_animation_frame.set(false);
        log::warn!("requestAnimationFrame failed: {:?}", err);
    }
}

fn install_event_listeners(
    window: &Window,
    state: &Rc<RefCell<WebAppState>>,
    pending_animation_frame: &Rc<Cell<bool>>,
    frame_cb: &Rc<RefCell<Option<Closure<dyn FnMut(f64)>>>>,
) {
    let root = state.borrow().dom_renderer.root().clone();
    let semantic_root = state.borrow().semantic_root();

    {
        let state = Rc::clone(state);
        let pending_animation_frame = Rc::clone(pending_animation_frame);
        let frame_cb = Rc::clone(frame_cb);
        let window = window.clone();
        let target = root.clone();
        let root_for_event = target.clone();
        let on_move = Closure::wrap(Box::new(move |event: MouseEvent| {
            if state.borrow().capture_path.is_some() {
                return;
            }
            let pos = mouse_pos(&root_for_event, &event);
            let mut state_ref = state.borrow_mut();
            state_ref.mouse_pos = pos;
            state_ref.handle_event(InputEvent::PointerMove { pos });
            let should_schedule = state_ref.should_schedule_frame();
            drop(state_ref);
            if should_schedule {
                schedule_animation_frame(&window, &pending_animation_frame, &frame_cb);
            }
        }) as Box<dyn FnMut(_)>);
        let _ =
            target.add_event_listener_with_callback("mousemove", on_move.as_ref().unchecked_ref());
        on_move.forget();
    }

    {
        let state = Rc::clone(state);
        let pending_animation_frame = Rc::clone(pending_animation_frame);
        let frame_cb = Rc::clone(frame_cb);
        let window = window.clone();
        let target = root.clone();
        let root_for_event = target.clone();
        let on_down = Closure::wrap(Box::new(move |event: MouseEvent| {
            let pos = mouse_pos(&root_for_event, &event);
            let button = state
                .borrow()
                .event_translator()
                .map_mouse_button(event.button());
            root_for_event.focus().ok();
            let mut state_ref = state.borrow_mut();
            state_ref.mouse_pos = pos;
            state_ref.handle_event(InputEvent::PointerDown { pos, button });
            let should_schedule = state_ref.should_schedule_frame();
            drop(state_ref);
            if should_schedule {
                schedule_animation_frame(&window, &pending_animation_frame, &frame_cb);
            }
        }) as Box<dyn FnMut(_)>);
        let _ =
            target.add_event_listener_with_callback("mousedown", on_down.as_ref().unchecked_ref());
        on_down.forget();
    }

    {
        let state = Rc::clone(state);
        let pending_animation_frame = Rc::clone(pending_animation_frame);
        let frame_cb = Rc::clone(frame_cb);
        let window = window.clone();
        let target = root.clone();
        let root_for_event = target.clone();
        let on_touch_start = Closure::wrap(Box::new(move |event: TouchEvent| {
            event.prevent_default();
            let Some((touch_id, pos)) = first_changed_touch(&root_for_event, &event) else {
                return;
            };
            let mut state_ref = state.borrow_mut();
            if state_ref.active_touch_id.is_some() {
                return;
            }
            state_ref.active_touch_id = Some(touch_id);
            state_ref.mouse_pos = pos;
            state_ref.handle_event(InputEvent::PointerDown {
                pos,
                button: PointerButton::Primary,
            });
            let should_schedule = state_ref.should_schedule_frame();
            drop(state_ref);
            if should_schedule {
                schedule_animation_frame(&window, &pending_animation_frame, &frame_cb);
            }
        }) as Box<dyn FnMut(_)>);
        let _ = target.add_event_listener_with_callback(
            "touchstart",
            on_touch_start.as_ref().unchecked_ref(),
        );
        on_touch_start.forget();
    }

    {
        let state = Rc::clone(state);
        let pending_animation_frame = Rc::clone(pending_animation_frame);
        let frame_cb = Rc::clone(frame_cb);
        let window = window.clone();
        let target = root.clone();
        let root_for_event = target.clone();
        let on_up = Closure::wrap(Box::new(move |event: MouseEvent| {
            if state.borrow().capture_path.is_some() {
                return;
            }
            let pos = mouse_pos(&root_for_event, &event);
            let button = state
                .borrow()
                .event_translator()
                .map_mouse_button(event.button());
            let mut state_ref = state.borrow_mut();
            state_ref.mouse_pos = pos;
            state_ref.handle_event(InputEvent::PointerUp { pos, button });
            let should_schedule = state_ref.should_schedule_frame();
            drop(state_ref);
            if should_schedule {
                schedule_animation_frame(&window, &pending_animation_frame, &frame_cb);
            }
        }) as Box<dyn FnMut(_)>);
        let _ = target.add_event_listener_with_callback("mouseup", on_up.as_ref().unchecked_ref());
        on_up.forget();
    }

    {
        let state = Rc::clone(state);
        let pending_animation_frame = Rc::clone(pending_animation_frame);
        let frame_cb = Rc::clone(frame_cb);
        let window = window.clone();
        let window_for_listener = window.clone();
        let root_for_event = root.clone();
        let on_move = Closure::wrap(Box::new(move |event: MouseEvent| {
            if state.borrow().capture_path.is_none() {
                return;
            }
            let pos = mouse_pos(&root_for_event, &event);
            let mut state_ref = state.borrow_mut();
            state_ref.mouse_pos = pos;
            state_ref.handle_event(InputEvent::PointerMove { pos });
            let should_schedule = state_ref.should_schedule_frame();
            drop(state_ref);
            if should_schedule {
                schedule_animation_frame(&window_for_listener, &pending_animation_frame, &frame_cb);
            }
        }) as Box<dyn FnMut(_)>);
        let _ =
            window.add_event_listener_with_callback("mousemove", on_move.as_ref().unchecked_ref());
        on_move.forget();
    }

    {
        let state = Rc::clone(state);
        let pending_animation_frame = Rc::clone(pending_animation_frame);
        let frame_cb = Rc::clone(frame_cb);
        let window = window.clone();
        let target = root.clone();
        let root_for_event = target.clone();
        let on_touch_move = Closure::wrap(Box::new(move |event: TouchEvent| {
            event.prevent_default();
            let Some(touch_id) = state.borrow().active_touch_id else {
                return;
            };
            if state.borrow().capture_path.is_some() {
                return;
            }
            let Some(pos) = changed_touch_pos(&root_for_event, &event, touch_id) else {
                return;
            };
            let mut state_ref = state.borrow_mut();
            let previous_pos = state_ref.mouse_pos;
            state_ref.mouse_pos = pos;
            state_ref.handle_event(InputEvent::PointerMove { pos });
            let delta = glam::Vec2::new(
                (pos.x - previous_pos.x) / WEB_SCROLL_DELTA_UNIT,
                (pos.y - previous_pos.y) / WEB_SCROLL_DELTA_UNIT,
            );
            state_ref.handle_event(InputEvent::Scroll {
                pos,
                delta,
                modifiers: Modifiers::default(),
            });
            let should_schedule = state_ref.should_schedule_frame();
            drop(state_ref);
            if should_schedule {
                schedule_animation_frame(&window, &pending_animation_frame, &frame_cb);
            }
        }) as Box<dyn FnMut(_)>);
        let _ = target
            .add_event_listener_with_callback("touchmove", on_touch_move.as_ref().unchecked_ref());
        on_touch_move.forget();
    }

    {
        let state = Rc::clone(state);
        let pending_animation_frame = Rc::clone(pending_animation_frame);
        let frame_cb = Rc::clone(frame_cb);
        let window = window.clone();
        let window_for_listener = window.clone();
        let root_for_event = root.clone();
        let on_move_captured = Closure::wrap(Box::new(move |event: TouchEvent| {
            event.prevent_default();
            let Some(touch_id) = state.borrow().active_touch_id else {
                return;
            };
            if state.borrow().capture_path.is_none() {
                return;
            }
            let Some(pos) = changed_touch_pos(&root_for_event, &event, touch_id) else {
                return;
            };
            let mut state_ref = state.borrow_mut();
            state_ref.mouse_pos = pos;
            state_ref.handle_event(InputEvent::PointerMove { pos });
            let should_schedule = state_ref.should_schedule_frame();
            drop(state_ref);
            if should_schedule {
                schedule_animation_frame(&window_for_listener, &pending_animation_frame, &frame_cb);
            }
        }) as Box<dyn FnMut(_)>);
        let _ = window.add_event_listener_with_callback(
            "touchmove",
            on_move_captured.as_ref().unchecked_ref(),
        );
        on_move_captured.forget();
    }

    {
        let state = Rc::clone(state);
        let pending_animation_frame = Rc::clone(pending_animation_frame);
        let frame_cb = Rc::clone(frame_cb);
        let window = window.clone();
        let window_for_listener = window.clone();
        let root_for_event = root.clone();
        let on_up = Closure::wrap(Box::new(move |event: MouseEvent| {
            if state.borrow().capture_path.is_none() {
                return;
            }
            let pos = mouse_pos(&root_for_event, &event);
            let button = state
                .borrow()
                .event_translator()
                .map_mouse_button(event.button());
            let mut state_ref = state.borrow_mut();
            state_ref.mouse_pos = pos;
            state_ref.handle_event(InputEvent::PointerUp { pos, button });
            let should_schedule = state_ref.should_schedule_frame();
            drop(state_ref);
            if should_schedule {
                schedule_animation_frame(&window_for_listener, &pending_animation_frame, &frame_cb);
            }
        }) as Box<dyn FnMut(_)>);
        let _ = window.add_event_listener_with_callback("mouseup", on_up.as_ref().unchecked_ref());
        on_up.forget();
    }

    {
        let state = Rc::clone(state);
        let pending_animation_frame = Rc::clone(pending_animation_frame);
        let frame_cb = Rc::clone(frame_cb);
        let window = window.clone();
        let target = root.clone();
        let root_for_event = target.clone();
        let on_touch_end = Closure::wrap(Box::new(move |event: TouchEvent| {
            event.prevent_default();
            let Some(touch_id) = state.borrow().active_touch_id else {
                return;
            };
            if state.borrow().capture_path.is_some() {
                return;
            }
            let Some(pos) = changed_touch_pos(&root_for_event, &event, touch_id) else {
                return;
            };
            let mut state_ref = state.borrow_mut();
            state_ref.mouse_pos = pos;
            state_ref.handle_event(InputEvent::PointerUp {
                pos,
                button: PointerButton::Primary,
            });
            state_ref.active_touch_id = None;
            clear_touch_hover(&mut state_ref, &root_for_event);
            let should_schedule = state_ref.should_schedule_frame();
            drop(state_ref);
            if should_schedule {
                schedule_animation_frame(&window, &pending_animation_frame, &frame_cb);
            }
        }) as Box<dyn FnMut(_)>);
        let _ = target
            .add_event_listener_with_callback("touchend", on_touch_end.as_ref().unchecked_ref());
        on_touch_end.forget();
    }

    {
        let state = Rc::clone(state);
        let pending_animation_frame = Rc::clone(pending_animation_frame);
        let frame_cb = Rc::clone(frame_cb);
        let window = window.clone();
        let window_for_listener = window.clone();
        let root_for_event = root.clone();
        let on_up_captured = Closure::wrap(Box::new(move |event: TouchEvent| {
            event.prevent_default();
            let Some(touch_id) = state.borrow().active_touch_id else {
                return;
            };
            if state.borrow().capture_path.is_none() {
                return;
            }
            let Some(pos) = changed_touch_pos(&root_for_event, &event, touch_id) else {
                return;
            };
            let mut state_ref = state.borrow_mut();
            state_ref.mouse_pos = pos;
            state_ref.handle_event(InputEvent::PointerUp {
                pos,
                button: PointerButton::Primary,
            });
            state_ref.active_touch_id = None;
            clear_touch_hover(&mut state_ref, &root_for_event);
            let should_schedule = state_ref.should_schedule_frame();
            drop(state_ref);
            if should_schedule {
                schedule_animation_frame(&window_for_listener, &pending_animation_frame, &frame_cb);
            }
        }) as Box<dyn FnMut(_)>);
        let _ = window
            .add_event_listener_with_callback("touchend", on_up_captured.as_ref().unchecked_ref());
        on_up_captured.forget();
    }

    {
        let state = Rc::clone(state);
        let pending_animation_frame = Rc::clone(pending_animation_frame);
        let frame_cb = Rc::clone(frame_cb);
        let window = window.clone();
        let target = root.clone();
        let root_for_event = target.clone();
        let on_cancel = Closure::wrap(Box::new(move |event: TouchEvent| {
            event.prevent_default();
            let Some(touch_id) = state.borrow().active_touch_id else {
                return;
            };
            let Some(_pos) = changed_touch_pos(&root_for_event, &event, touch_id) else {
                return;
            };
            let pos = touch_outside_pos(&root_for_event);
            let mut state_ref = state.borrow_mut();
            state_ref.active_touch_id = None;
            state_ref.mouse_pos = pos;
            state_ref.handle_event(InputEvent::PointerUp {
                pos,
                button: PointerButton::Primary,
            });
            let should_schedule = state_ref.should_schedule_frame();
            drop(state_ref);
            if should_schedule {
                schedule_animation_frame(&window, &pending_animation_frame, &frame_cb);
            }
        }) as Box<dyn FnMut(_)>);
        let _ = target
            .add_event_listener_with_callback("touchcancel", on_cancel.as_ref().unchecked_ref());
        on_cancel.forget();
    }

    {
        let state = Rc::clone(state);
        let pending_animation_frame = Rc::clone(pending_animation_frame);
        let frame_cb = Rc::clone(frame_cb);
        let window = window.clone();
        let target = root.clone();
        let root_for_event = target.clone();
        let on_wheel = Closure::wrap(Box::new(move |event: WheelEvent| {
            event.prevent_default();
            let pos = mouse_pos_wheel(&root_for_event, &event);
            let mut delta_x = event.delta_x() as f32;
            let mut delta_y = -(event.delta_y() as f32);
            if event.delta_mode() == WheelEvent::DOM_DELTA_PIXEL {
                delta_x /= WEB_SCROLL_DELTA_UNIT;
                delta_y /= WEB_SCROLL_DELTA_UNIT;
            }
            let modifiers = state.borrow().event_translator().modifiers_from_flags(
                event.shift_key(),
                event.ctrl_key(),
                event.alt_key(),
                event.meta_key(),
            );
            let mut state_ref = state.borrow_mut();
            state_ref.handle_event(InputEvent::Scroll {
                pos,
                delta: glam::Vec2::new(delta_x, delta_y),
                modifiers,
            });
            let should_schedule = state_ref.should_schedule_frame();
            drop(state_ref);
            if should_schedule {
                schedule_animation_frame(&window, &pending_animation_frame, &frame_cb);
            }
        }) as Box<dyn FnMut(_)>);
        let _ = target.add_event_listener_with_callback("wheel", on_wheel.as_ref().unchecked_ref());
        on_wheel.forget();
    }

    {
        let state = Rc::clone(state);
        let pending_animation_frame = Rc::clone(pending_animation_frame);
        let frame_cb = Rc::clone(frame_cb);
        let window = window.clone();
        let on_key_down = Closure::wrap(Box::new(move |event: WebKeyboardEvent| {
            let focused_text_editor = state.borrow().focused_text_editor_state().is_some();
            let dispatch = state.borrow().event_translator().translate_key_down(
                &event.key(),
                event.shift_key(),
                event.ctrl_key(),
                event.alt_key(),
                event.meta_key(),
                focused_text_editor,
            );
            if dispatch.prevent_default {
                event.prevent_default();
            }
            if let Some(keyboard_event) = dispatch.keyboard_event {
                state.borrow_mut().handle_event(keyboard_event);
            }
            if let Some(text_event) = dispatch.text_event {
                state.borrow_mut().handle_event(text_event);
            }
            if state.borrow().should_schedule_frame() {
                schedule_animation_frame(&window, &pending_animation_frame, &frame_cb);
            }
        }) as Box<dyn FnMut(_)>);
        let _ =
            root.add_event_listener_with_callback("keydown", on_key_down.as_ref().unchecked_ref());
        on_key_down.forget();
    }

    {
        let state = Rc::clone(state);
        let pending_animation_frame = Rc::clone(pending_animation_frame);
        let frame_cb = Rc::clone(frame_cb);
        let window = window.clone();
        let on_key_up = Closure::wrap(Box::new(move |event: WebKeyboardEvent| {
            if let Some(key_event) = state.borrow().event_translator().translate_key_up(
                &event.key(),
                event.shift_key(),
                event.ctrl_key(),
                event.alt_key(),
                event.meta_key(),
            ) {
                state.borrow_mut().handle_event(key_event);
            }
            if state.borrow().should_schedule_frame() {
                schedule_animation_frame(&window, &pending_animation_frame, &frame_cb);
            }
        }) as Box<dyn FnMut(_)>);
        let _ = root.add_event_listener_with_callback("keyup", on_key_up.as_ref().unchecked_ref());
        on_key_up.forget();
    }

    {
        let bridge = state.borrow().text_input_element();
        let state = Rc::clone(state);
        let pending_animation_frame = Rc::clone(pending_animation_frame);
        let frame_cb = Rc::clone(frame_cb);
        let window = window.clone();
        let on_before_input = Closure::wrap(Box::new(move |event: WebInputEvent| {
            let translated = state.borrow().event_translator().translate_before_input(
                &event.input_type(),
                event.data(),
                state.borrow().text_input_is_syncing(),
                state.borrow().focused_text_editor_state().is_some(),
            );
            if let Some(input_event) = translated {
                event.prevent_default();
                state.borrow_mut().handle_event(input_event);
            }
            if state.borrow().should_schedule_frame() {
                schedule_animation_frame(&window, &pending_animation_frame, &frame_cb);
            }
        }) as Box<dyn FnMut(_)>);
        let _ = bridge.add_event_listener_with_callback(
            "beforeinput",
            on_before_input.as_ref().unchecked_ref(),
        );
        on_before_input.forget();
    }

    {
        let bridge = state.borrow().text_input_element();
        let state = Rc::clone(state);
        let pending_animation_frame = Rc::clone(pending_animation_frame);
        let frame_cb = Rc::clone(frame_cb);
        let window = window.clone();
        let on_input = Closure::wrap(Box::new(move |_event: WebInputEvent| {
            if state.borrow().text_input_is_syncing() {
                return;
            }
            if state.borrow().focused_text_editor_state().is_none() {
                return;
            }
            state.borrow_mut().sync_text_input_bridge();
            if state.borrow().should_schedule_frame() {
                schedule_animation_frame(&window, &pending_animation_frame, &frame_cb);
            }
        }) as Box<dyn FnMut(_)>);
        let _ = bridge.add_event_listener_with_callback("input", on_input.as_ref().unchecked_ref());
        on_input.forget();
    }

    {
        let bridge = state.borrow().text_input_element();
        let state = Rc::clone(state);
        let pending_animation_frame = Rc::clone(pending_animation_frame);
        let frame_cb = Rc::clone(frame_cb);
        let window = window.clone();
        let on_composition_start = Closure::wrap(Box::new(move |_event: WebCompositionEvent| {
            let translated = state
                .borrow()
                .event_translator()
                .translate_composition_start(
                    state.borrow().text_input_is_syncing(),
                    state.borrow().focused_text_editor_state().is_some(),
                );
            if let Some(input_event) = translated {
                let mut state_ref = state.borrow_mut();
                state_ref.ime_composing = true;
                state_ref.handle_event(input_event);
                let should_schedule = state_ref.should_schedule_frame();
                drop(state_ref);
                if should_schedule {
                    schedule_animation_frame(&window, &pending_animation_frame, &frame_cb);
                }
            }
        }) as Box<dyn FnMut(_)>);
        let _ = bridge.add_event_listener_with_callback(
            "compositionstart",
            on_composition_start.as_ref().unchecked_ref(),
        );
        on_composition_start.forget();
    }

    {
        let bridge = state.borrow().text_input_element();
        let state = Rc::clone(state);
        let pending_animation_frame = Rc::clone(pending_animation_frame);
        let frame_cb = Rc::clone(frame_cb);
        let window = window.clone();
        let on_composition_update = Closure::wrap(Box::new(move |event: WebCompositionEvent| {
            if let Some(input_event) = state
                .borrow()
                .event_translator()
                .translate_composition_update(
                    event.data().unwrap_or_default(),
                    state.borrow().text_input_is_syncing(),
                    state.borrow().focused_text_editor_state().is_some(),
                )
            {
                state.borrow_mut().handle_event(input_event);
                if state.borrow().should_schedule_frame() {
                    schedule_animation_frame(&window, &pending_animation_frame, &frame_cb);
                }
            }
        }) as Box<dyn FnMut(_)>);
        let _ = bridge.add_event_listener_with_callback(
            "compositionupdate",
            on_composition_update.as_ref().unchecked_ref(),
        );
        on_composition_update.forget();
    }

    {
        let bridge = state.borrow().text_input_element();
        let state = Rc::clone(state);
        let pending_animation_frame = Rc::clone(pending_animation_frame);
        let frame_cb = Rc::clone(frame_cb);
        let window = window.clone();
        let on_composition_end = Closure::wrap(Box::new(move |event: WebCompositionEvent| {
            if let Some(input_event) = state.borrow().event_translator().translate_composition_end(
                event.data().unwrap_or_default(),
                state.borrow().text_input_is_syncing(),
                state.borrow().focused_text_editor_state().is_some(),
            ) {
                let mut state_ref = state.borrow_mut();
                state_ref.ime_composing = false;
                state_ref.handle_event(input_event);
                let should_schedule = state_ref.should_schedule_frame();
                drop(state_ref);
                if should_schedule {
                    schedule_animation_frame(&window, &pending_animation_frame, &frame_cb);
                }
            }
        }) as Box<dyn FnMut(_)>);
        let _ = bridge.add_event_listener_with_callback(
            "compositionend",
            on_composition_end.as_ref().unchecked_ref(),
        );
        on_composition_end.forget();
    }

    {
        let bridge = state.borrow().text_input_element();
        let state = Rc::clone(state);
        let pending_animation_frame = Rc::clone(pending_animation_frame);
        let frame_cb = Rc::clone(frame_cb);
        let window = window.clone();
        let on_paste = Closure::wrap(Box::new(move |event: ClipboardEvent| {
            let translated = state.borrow().event_translator().translate_paste(
                event
                    .clipboard_data()
                    .and_then(|clipboard| clipboard.get_data("text/plain").ok()),
                state.borrow().focused_text_editor_state().is_some(),
            );
            if let Some(input_event) = translated {
                event.prevent_default();
                state.borrow_mut().handle_event(input_event);
                if state.borrow().should_schedule_frame() {
                    schedule_animation_frame(&window, &pending_animation_frame, &frame_cb);
                }
            }
        }) as Box<dyn FnMut(_)>);
        let _ = bridge.add_event_listener_with_callback("paste", on_paste.as_ref().unchecked_ref());
        on_paste.forget();
    }

    {
        let bridge = state.borrow().text_input_element();
        let state = Rc::clone(state);
        let on_copy = Closure::wrap(Box::new(move |event: ClipboardEvent| {
            if state.borrow().focused_text_editor_state().is_none() {
                return;
            }
            let text = {
                let mut state_ref = state.borrow_mut();
                state_ref
                    .take_pending_clipboard_write()
                    .or_else(|| state_ref.focused_selection_text())
            };
            let Some(text) = text else {
                return;
            };
            if let Some(clipboard) = event.clipboard_data() {
                let _ = clipboard.set_data("text/plain", &text);
                event.prevent_default();
            }
        }) as Box<dyn FnMut(_)>);
        let _ = bridge.add_event_listener_with_callback("copy", on_copy.as_ref().unchecked_ref());
        on_copy.forget();
    }

    {
        let bridge = state.borrow().text_input_element();
        let state = Rc::clone(state);
        let pending_animation_frame = Rc::clone(pending_animation_frame);
        let frame_cb = Rc::clone(frame_cb);
        let window = window.clone();
        let on_cut = Closure::wrap(Box::new(move |event: ClipboardEvent| {
            if state.borrow().focused_text_editor_state().is_none() {
                return;
            }

            let fallback_text = state.borrow().focused_selection_text();
            let mut clipboard_text = {
                let mut state_ref = state.borrow_mut();
                state_ref.take_pending_clipboard_write()
            };

            if clipboard_text.is_none() && fallback_text.is_some() {
                let primary_modifiers = state.borrow().shortcut_profile().primary_modifiers();
                let mut synthetic = sparsha_input::KeyboardEvent::key_down(
                    sparsha_input::Key::Character("x".to_owned()),
                    sparsha_input::ui_events::keyboard::Code::Unidentified,
                );
                synthetic.modifiers = primary_modifiers;
                state
                    .borrow_mut()
                    .handle_event(InputEvent::KeyDown { event: synthetic });
                clipboard_text = state.borrow_mut().take_pending_clipboard_write();
                if clipboard_text.is_none() {
                    let mut ctrl_synthetic = sparsha_input::KeyboardEvent::key_down(
                        sparsha_input::Key::Character("x".to_owned()),
                        sparsha_input::ui_events::keyboard::Code::Unidentified,
                    );
                    ctrl_synthetic.modifiers = Modifiers::CONTROL;
                    state.borrow_mut().handle_event(InputEvent::KeyDown {
                        event: ctrl_synthetic,
                    });
                    clipboard_text = state.borrow_mut().take_pending_clipboard_write();
                }
            }

            let Some(text) = clipboard_text.or(fallback_text) else {
                return;
            };
            if let Some(clipboard) = event.clipboard_data() {
                let _ = clipboard.set_data("text/plain", &text);
                event.prevent_default();
            }
            if state.borrow().should_schedule_frame() {
                schedule_animation_frame(&window, &pending_animation_frame, &frame_cb);
            }
        }) as Box<dyn FnMut(_)>);
        let _ = bridge.add_event_listener_with_callback("cut", on_cut.as_ref().unchecked_ref());
        on_cut.forget();
    }

    {
        let state = Rc::clone(state);
        let pending_animation_frame = Rc::clone(pending_animation_frame);
        let frame_cb = Rc::clone(frame_cb);
        let window = window.clone();
        let on_focus = Closure::wrap(Box::new(move || {
            let Ok(mut state_ref) = state.try_borrow_mut() else {
                return;
            };
            state_ref.handle_event(InputEvent::FocusGained);
            let should_schedule = state_ref.should_schedule_frame();
            drop(state_ref);
            if should_schedule {
                schedule_animation_frame(&window, &pending_animation_frame, &frame_cb);
            }
        }) as Box<dyn FnMut()>);
        let _ = root.add_event_listener_with_callback("focus", on_focus.as_ref().unchecked_ref());
        on_focus.forget();
    }

    {
        let state = Rc::clone(state);
        let pending_animation_frame = Rc::clone(pending_animation_frame);
        let frame_cb = Rc::clone(frame_cb);
        let window = window.clone();
        let on_blur = Closure::wrap(Box::new(move || {
            let Ok(mut state_ref) = state.try_borrow_mut() else {
                return;
            };
            state_ref.handle_event(InputEvent::FocusLost);
            let should_schedule = state_ref.should_schedule_frame();
            drop(state_ref);
            if should_schedule {
                schedule_animation_frame(&window, &pending_animation_frame, &frame_cb);
            }
        }) as Box<dyn FnMut()>);
        let _ = root.add_event_listener_with_callback("blur", on_blur.as_ref().unchecked_ref());
        on_blur.forget();
    }

    {
        let state = Rc::clone(state);
        let pending_animation_frame = Rc::clone(pending_animation_frame);
        let frame_cb = Rc::clone(frame_cb);
        let window = window.clone();
        let target = semantic_root.clone();
        let on_focus_in = Closure::wrap(Box::new(move |event: Event| {
            let Some(node_id) = event_target_node_id(event.target()) else {
                return;
            };
            let is_text = event_target_is_text_editor(event.target());
            let Ok(mut state_ref) = state.try_borrow_mut() else {
                return;
            };
            state_ref.set_accessibility_text_focus_node(is_text.then_some(node_id));
            state_ref.handle_accessibility_action(node_id, AccessibilityAction::Focus, None);
            let should_schedule = state_ref.should_schedule_frame();
            drop(state_ref);
            if should_schedule {
                schedule_animation_frame(&window, &pending_animation_frame, &frame_cb);
            }
        }) as Box<dyn FnMut(_)>);
        let _ = target
            .add_event_listener_with_callback("focusin", on_focus_in.as_ref().unchecked_ref());
        on_focus_in.forget();
    }

    {
        let state = Rc::clone(state);
        let target = semantic_root.clone();
        let on_focus_out = Closure::wrap(Box::new(move |_event: Event| {
            let Ok(mut state_ref) = state.try_borrow_mut() else {
                return;
            };
            state_ref.set_accessibility_text_focus_node(None);
            state_ref.sync_text_input_bridge();
        }) as Box<dyn FnMut(_)>);
        let _ = target
            .add_event_listener_with_callback("focusout", on_focus_out.as_ref().unchecked_ref());
        on_focus_out.forget();
    }

    {
        let state = Rc::clone(state);
        let pending_animation_frame = Rc::clone(pending_animation_frame);
        let frame_cb = Rc::clone(frame_cb);
        let window = window.clone();
        let target = semantic_root.clone();
        let on_click = Closure::wrap(Box::new(move |event: Event| {
            if event_target_is_checkbox(event.target())
                || event_target_is_text_editor(event.target())
            {
                return;
            }
            let Some(node_id) = event_target_node_id(event.target()) else {
                return;
            };
            let mut state_ref = state.borrow_mut();
            state_ref.handle_accessibility_action(node_id, AccessibilityAction::Click, None);
            let should_schedule = state_ref.should_schedule_frame();
            drop(state_ref);
            if should_schedule {
                schedule_animation_frame(&window, &pending_animation_frame, &frame_cb);
            }
        }) as Box<dyn FnMut(_)>);
        let _ = target.add_event_listener_with_callback("click", on_click.as_ref().unchecked_ref());
        on_click.forget();
    }

    {
        let state = Rc::clone(state);
        let pending_animation_frame = Rc::clone(pending_animation_frame);
        let frame_cb = Rc::clone(frame_cb);
        let window = window.clone();
        let target = semantic_root.clone();
        let on_change = Closure::wrap(Box::new(move |event: Event| {
            if !event_target_is_checkbox(event.target()) {
                return;
            }
            let Some(node_id) = event_target_node_id(event.target()) else {
                return;
            };
            let mut state_ref = state.borrow_mut();
            state_ref.handle_accessibility_action(node_id, AccessibilityAction::Click, None);
            let should_schedule = state_ref.should_schedule_frame();
            drop(state_ref);
            if should_schedule {
                schedule_animation_frame(&window, &pending_animation_frame, &frame_cb);
            }
        }) as Box<dyn FnMut(_)>);
        let _ =
            target.add_event_listener_with_callback("change", on_change.as_ref().unchecked_ref());
        on_change.forget();
    }

    {
        let state = Rc::clone(state);
        let pending_animation_frame = Rc::clone(pending_animation_frame);
        let frame_cb = Rc::clone(frame_cb);
        let window = window.clone();
        let target = semantic_root.clone();
        let on_input = Closure::wrap(Box::new(move |event: Event| {
            if !event_target_is_text_editor(event.target()) {
                return;
            }
            let Some(node_id) = event_target_node_id(event.target()) else {
                return;
            };
            let Some(value) = event_target_text_value(event.target()) else {
                return;
            };
            let mut state_ref = state.borrow_mut();
            state_ref.set_accessibility_text_focus_node(Some(node_id));
            state_ref.handle_accessibility_action(
                node_id,
                AccessibilityAction::SetValue,
                Some(value),
            );
            let should_schedule = state_ref.should_schedule_frame();
            drop(state_ref);
            if should_schedule {
                schedule_animation_frame(&window, &pending_animation_frame, &frame_cb);
            }
        }) as Box<dyn FnMut(_)>);
        let _ = target.add_event_listener_with_callback("input", on_input.as_ref().unchecked_ref());
        on_input.forget();
    }

    {
        let state = Rc::clone(state);
        let pending_animation_frame = Rc::clone(pending_animation_frame);
        let frame_cb = Rc::clone(frame_cb);
        let window_for_resize = window.clone();
        let on_resize = Closure::wrap(Box::new(move || {
            let mut state_ref = state.borrow_mut();
            state_ref.update_viewport();
            state_ref.needs_layout = true;
            let should_schedule = state_ref.should_schedule_frame();
            drop(state_ref);
            if should_schedule {
                schedule_animation_frame(&window_for_resize, &pending_animation_frame, &frame_cb);
            }
        }) as Box<dyn FnMut()>);
        let _ =
            window.add_event_listener_with_callback("resize", on_resize.as_ref().unchecked_ref());
        on_resize.forget();
    }

    {
        let state = Rc::clone(state);
        let pending_animation_frame = Rc::clone(pending_animation_frame);
        let frame_cb = Rc::clone(frame_cb);
        let window_for_hash = window.clone();
        let navigator = state.borrow().router_navigator.clone();
        let on_hash_change = Closure::wrap(Box::new(move || {
            let hash = window_for_hash.location().hash().ok().unwrap_or_default();
            if !should_sync_external_hash(&navigator.current_path(), &hash) {
                return;
            }
            let path = hash_to_path(&hash);
            navigator.sync_external_path(&path);
            let mut state_ref = state.borrow_mut();
            state_ref.needs_layout = true;
            let should_schedule = state_ref.should_schedule_frame();
            drop(state_ref);
            if should_schedule {
                schedule_animation_frame(&window_for_hash, &pending_animation_frame, &frame_cb);
            }
        }) as Box<dyn FnMut()>);
        let _ = window.add_event_listener_with_callback(
            "hashchange",
            on_hash_change.as_ref().unchecked_ref(),
        );
        on_hash_change.forget();
    }
}

#[cfg_attr(not(test), allow(dead_code))]
fn mouse_button(button: i16) -> PointerButton {
    WebEventTranslator::new().map_mouse_button(button)
}

fn event_target_element(target: Option<web_sys::EventTarget>) -> Option<Element> {
    let target = target?;
    let element: Element = target.dyn_into().ok()?;
    element.closest("[data-sparsha-a11y-node]").ok().flatten()
}

fn event_target_node_id(target: Option<web_sys::EventTarget>) -> Option<u64> {
    event_target_element(target)?
        .get_attribute("data-sparsha-a11y-node")?
        .parse()
        .ok()
}

fn event_target_is_checkbox(target: Option<web_sys::EventTarget>) -> bool {
    event_target_element(target)
        .and_then(|element| element.dyn_into::<HtmlInputElement>().ok())
        .is_some_and(|input| input.type_() == "checkbox")
}

fn web_viewport_info(width: f32, height: f32) -> ViewportInfo {
    ViewportInfo::new(width, height)
}

fn event_target_is_text_editor(target: Option<web_sys::EventTarget>) -> bool {
    if let Some(input) = event_target_element(target.clone())
        .and_then(|element| element.dyn_into::<HtmlInputElement>().ok())
    {
        return input.type_() == "text";
    }
    event_target_element(target)
        .and_then(|element| element.dyn_into::<HtmlTextAreaElement>().ok())
        .is_some()
}

fn event_target_text_value(target: Option<web_sys::EventTarget>) -> Option<String> {
    if let Some(input) = event_target_element(target.clone())
        .and_then(|element| element.dyn_into::<HtmlInputElement>().ok())
    {
        return Some(input.value());
    }
    event_target_element(target)
        .and_then(|element| element.dyn_into::<HtmlTextAreaElement>().ok())
        .map(|textarea| textarea.value())
}

fn mouse_pos(root: &web_sys::HtmlElement, event: &MouseEvent) -> glam::Vec2 {
    let rect = root.get_bounding_client_rect();
    glam::Vec2::new(
        event.client_x() as f32 - rect.left() as f32,
        event.client_y() as f32 - rect.top() as f32,
    )
}

fn mouse_pos_wheel(root: &web_sys::HtmlElement, event: &WheelEvent) -> glam::Vec2 {
    let rect = root.get_bounding_client_rect();
    glam::Vec2::new(
        event.client_x() as f32 - rect.left() as f32,
        event.client_y() as f32 - rect.top() as f32,
    )
}

fn first_changed_touch(
    root: &web_sys::HtmlElement,
    event: &TouchEvent,
) -> Option<(i32, glam::Vec2)> {
    let touch = event.changed_touches().item(0)?;
    Some((touch.identifier(), touch_to_local_pos(root, &touch)))
}

fn changed_touch_pos(
    root: &web_sys::HtmlElement,
    event: &TouchEvent,
    touch_id: i32,
) -> Option<glam::Vec2> {
    touch_by_id(&event.changed_touches(), touch_id).map(|touch| touch_to_local_pos(root, &touch))
}

fn touch_by_id(list: &TouchList, touch_id: i32) -> Option<Touch> {
    (0..list.length()).find_map(|index| {
        let touch = list.item(index)?;
        (touch.identifier() == touch_id).then_some(touch)
    })
}

fn touch_to_local_pos(root: &web_sys::HtmlElement, touch: &Touch) -> glam::Vec2 {
    let rect = root.get_bounding_client_rect();
    glam::Vec2::new(
        touch.client_x() as f32 - rect.left() as f32,
        touch.client_y() as f32 - rect.top() as f32,
    )
}

fn touch_outside_pos(root: &web_sys::HtmlElement) -> glam::Vec2 {
    glam::Vec2::new(
        -(root.client_width() as f32) - 1.0,
        -(root.client_height() as f32) - 1.0,
    )
}

fn clear_touch_hover(state: &mut WebAppState, root: &web_sys::HtmlElement) {
    let pos = touch_outside_pos(root);
    state.mouse_pos = pos;
    state.handle_event(InputEvent::PointerMove { pos });
}

#[cfg_attr(not(test), allow(dead_code))]
fn browser_modifiers(event: &WebKeyboardEvent) -> Modifiers {
    browser_modifiers_from_flags(
        event.shift_key(),
        event.ctrl_key(),
        event.alt_key(),
        event.meta_key(),
    )
}

#[cfg_attr(not(test), allow(dead_code))]
fn browser_wheel_modifiers(event: &WheelEvent) -> Modifiers {
    browser_modifiers_from_flags(
        event.shift_key(),
        event.ctrl_key(),
        event.alt_key(),
        event.meta_key(),
    )
}

#[cfg_attr(not(test), allow(dead_code))]
fn browser_modifiers_from_flags(shift: bool, ctrl: bool, alt: bool, meta: bool) -> Modifiers {
    WebEventTranslator::new().modifiers_from_flags(shift, ctrl, alt, meta)
}

#[cfg_attr(not(test), allow(dead_code))]
fn should_prevent_keydown_for_text_editor_action(
    event: &sparsha_input::KeyboardEvent,
    action: StandardAction,
) -> bool {
    WebEventTranslator::new().should_prevent_keydown_for_text_editor_action(event, action)
}

#[cfg_attr(not(test), allow(dead_code))]
fn should_emit_text(event: &WebKeyboardEvent) -> bool {
    WebEventTranslator::new().should_emit_text(
        &event.key(),
        event.ctrl_key(),
        event.alt_key(),
        event.meta_key(),
    )
}

#[cfg_attr(not(test), allow(dead_code))]
fn should_forward_keydown_to_widget_tree(
    focused_text_editor: bool,
    key: &str,
    ctrl: bool,
    alt: bool,
    meta: bool,
) -> bool {
    WebEventTranslator::new().should_forward_keydown_to_widget_tree(
        focused_text_editor,
        key,
        ctrl,
        alt,
        meta,
    )
}

fn should_sync_external_hash(current_path: &str, hash: &str) -> bool {
    hash_to_path(hash) != current_path
}

fn can_start_route_view_transition(route_changed: bool, first_paint_emitted: bool) -> bool {
    route_changed && first_paint_emitted
}

fn start_document_view_transition() {
    let Some(window) = web_sys::window() else {
        return;
    };
    let Some(document) = window.document() else {
        return;
    };

    startRouteViewTransition(&document);
}

fn should_render_web_layers(
    painted_frame: bool,
    pending_surface_retry: bool,
    has_surface_frames: bool,
    surface_status: HybridSurfaceStatus,
) -> bool {
    painted_frame
        || pending_surface_retry
        || (has_surface_frames && matches!(surface_status, HybridSurfaceStatus::Initializing))
}

#[cfg_attr(not(test), allow(dead_code))]
fn map_browser_key(key: String) -> Option<sparsha_input::Key> {
    WebEventTranslator::new().map_key(&key)
}

#[cfg(test)]
mod tests {
    use super::*;
    use sparsha_core::{Color, Rect};
    use sparsha_layout::taffy;
    use sparsha_widgets::DrawSurface;

    struct HybridOverlayWidget {
        id: sparsha_layout::WidgetId,
        surface: DrawSurface,
    }

    impl HybridOverlayWidget {
        fn new() -> Self {
            Self {
                id: sparsha_layout::WidgetId::default(),
                surface: DrawSurface::new(|ctx| {
                    ctx.fill_rect(ctx.bounds, Color::from_hex(0x112233));
                })
                .fill(),
            }
        }
    }

    impl Widget for HybridOverlayWidget {
        fn id(&self) -> sparsha_layout::WidgetId {
            self.id
        }

        fn set_id(&mut self, id: sparsha_layout::WidgetId) {
            self.id = id;
        }

        fn style(&self) -> taffy::Style {
            taffy::Style {
                size: taffy::prelude::Size {
                    width: taffy::prelude::length(240.0),
                    height: taffy::prelude::length(140.0),
                },
                ..Default::default()
            }
        }

        fn paint(&self, ctx: &mut PaintContext) {
            ctx.fill_rect(Rect::new(12.0, 12.0, 80.0, 24.0), Color::WHITE);
        }

        fn draw_surface(&self) -> Option<&DrawSurface> {
            Some(&self.surface)
        }
    }

    #[test]
    fn hybrid_widget_keeps_surface_and_normal_paint() {
        let mut widget = HybridOverlayWidget::new();
        let mut layout_tree = LayoutTree::new();
        let root_id = layout_tree.new_leaf(widget.style());
        widget.set_id(root_id);
        layout_tree.set_root(root_id);
        layout_tree.compute_layout(240.0, 140.0);

        let focus = FocusManager::new();
        let mut draw_list = DrawList::new();
        let mut surface_frames = Vec::new();
        let mut text_system = TextSystem::new_headless();
        let mut commands = PaintCommands::default();

        paint_widget_subtree(
            &widget,
            &layout_tree,
            &focus,
            &mut draw_list,
            &mut surface_frames,
            2.0,
            &mut text_system as *mut TextSystem,
            0.0,
            &mut commands,
        );

        assert_eq!(surface_frames.len(), 1);
        assert_eq!(draw_list.len(), 1);
    }
}

#[cfg(all(test, target_arch = "wasm32"))]
mod wasm_tests {
    use super::*;
    use serde_json::json;
    use sparsha_core::Rect;
    use sparsha_widgets::AccessibilityRole;
    use wasm_bindgen_test::*;

    wasm_bindgen_test_configure!(run_in_browser);

    fn document() -> web_sys::Document {
        web_sys::window()
            .and_then(|window| window.document())
            .expect("window document")
    }

    #[wasm_bindgen_test]
    fn semantic_text_input_node_preserves_value_and_label() {
        let element = crate::platform::web::create_semantic_element(
            &document(),
            &crate::accessibility::AccessibilityNodeSnapshot {
                id: 7,
                path: vec![0],
                role: AccessibilityRole::TextInput,
                label: Some("Email".to_owned()),
                description: None,
                value: Some("hello@example.com".to_owned()),
                hidden: false,
                disabled: false,
                checked: None,
                actions: vec![AccessibilityAction::Focus, AccessibilityAction::SetValue],
                bounds: Rect::new(10.0, 20.0, 120.0, 40.0),
                children: Vec::new(),
            },
        )
        .expect("semantic element");
        let input = element.dyn_into::<HtmlInputElement>().expect("text input");
        assert_eq!(input.type_(), "text");
        assert_eq!(input.value(), "hello@example.com");
        assert_eq!(input.get_attribute("aria-label").as_deref(), Some("Email"));
    }

    #[wasm_bindgen_test]
    fn hash_sync_helper_ignores_matching_hash_paths() {
        assert!(!should_sync_external_hash("/", "#/"));
        assert!(!should_sync_external_hash("/about", "#/about"));
        assert!(should_sync_external_hash("/", "#/about"));
    }

    #[test]
    fn route_view_transition_runs_only_after_first_paint() {
        assert!(!can_start_route_view_transition(false, false));
        assert!(!can_start_route_view_transition(false, true));
        assert!(!can_start_route_view_transition(true, false));
        assert!(can_start_route_view_transition(true, true));
    }

    #[wasm_bindgen_test]
    fn start_document_view_transition_degrades_gracefully() {
        start_document_view_transition();
    }

    #[test]
    fn layer_render_decision_keeps_first_dom_paint() {
        assert!(should_render_web_layers(
            true,
            false,
            false,
            HybridSurfaceStatus::Uninitialized
        ));
        assert!(!should_render_web_layers(
            false,
            false,
            false,
            HybridSurfaceStatus::Uninitialized
        ));
    }

    #[test]
    fn browser_key_mapping_normalizes_space_to_character() {
        assert_eq!(
            map_browser_key(" ".to_owned()),
            Some(sparsha_input::Key::Character(" ".to_owned()))
        );
    }

    #[test]
    fn browser_key_mapping_keeps_named_navigation_keys() {
        assert_eq!(
            map_browser_key("Enter".to_owned()),
            Some(sparsha_input::Key::Named(sparsha_input::NamedKey::Enter))
        );
        assert_eq!(
            map_browser_key("ArrowLeft".to_owned()),
            Some(sparsha_input::Key::Named(
                sparsha_input::NamedKey::ArrowLeft
            ))
        );
    }

    #[test]
    fn web_viewport_info_uses_css_pixel_dimensions() {
        let viewport = web_viewport_info(390.0, 844.0);
        assert_eq!(viewport.width, 390.0);
        assert_eq!(viewport.height, 844.0);
        assert_eq!(viewport.class, sparsha_widgets::ViewportClass::Mobile);
    }

    #[test]
    fn text_editor_keydown_skips_plain_printable_keys_but_keeps_shortcuts() {
        assert!(!should_forward_keydown_to_widget_tree(
            true, " ", false, false, false
        ));
        assert!(!should_forward_keydown_to_widget_tree(
            true, "h", false, false, false
        ));
        assert!(should_forward_keydown_to_widget_tree(
            true, "Enter", false, false, false
        ));
        assert!(should_forward_keydown_to_widget_tree(
            true,
            "Backspace",
            false,
            false,
            false
        ));
        assert!(should_forward_keydown_to_widget_tree(
            true, "v", true, false, false
        ));
    }

    #[wasm_bindgen_test]
    fn text_editor_keydown_prevention_leaves_clipboard_shortcuts_to_browser() {
        let nav_event = sparsha_input::KeyboardEvent::key_down(
            sparsha_input::Key::Named(sparsha_input::NamedKey::Backspace),
            sparsha_input::ui_events::keyboard::Code::Backspace,
        );
        assert!(should_prevent_keydown_for_text_editor_action(
            &nav_event,
            StandardAction::Backspace
        ));
        let move_event = sparsha_input::KeyboardEvent::key_down(
            sparsha_input::Key::Named(sparsha_input::NamedKey::ArrowLeft),
            sparsha_input::ui_events::keyboard::Code::ArrowLeft,
        );
        assert!(should_prevent_keydown_for_text_editor_action(
            &move_event,
            StandardAction::MoveLeft
        ));
        let copy_event = sparsha_input::KeyboardEvent::key_down(
            sparsha_input::Key::Character("c".to_owned()),
            sparsha_input::ui_events::keyboard::Code::KeyC,
        );
        assert!(!should_prevent_keydown_for_text_editor_action(
            &copy_event,
            StandardAction::Copy
        ));
        let cut_event = sparsha_input::KeyboardEvent::key_down(
            sparsha_input::Key::Character("x".to_owned()),
            sparsha_input::ui_events::keyboard::Code::KeyX,
        );
        assert!(!should_prevent_keydown_for_text_editor_action(
            &cut_event,
            StandardAction::Cut
        ));
        let paste_event = sparsha_input::KeyboardEvent::key_down(
            sparsha_input::Key::Character("v".to_owned()),
            sparsha_input::ui_events::keyboard::Code::KeyV,
        );
        assert!(!should_prevent_keydown_for_text_editor_action(
            &paste_event,
            StandardAction::Paste
        ));
        let enter_event = sparsha_input::KeyboardEvent::key_down(
            sparsha_input::Key::Named(sparsha_input::NamedKey::Enter),
            sparsha_input::ui_events::keyboard::Code::Enter,
        );
        assert!(should_prevent_keydown_for_text_editor_action(
            &enter_event,
            StandardAction::Activate
        ));
        let space_event = sparsha_input::KeyboardEvent::key_down(
            sparsha_input::Key::Character(" ".to_owned()),
            sparsha_input::ui_events::keyboard::Code::Space,
        );
        assert!(!should_prevent_keydown_for_text_editor_action(
            &space_event,
            StandardAction::Activate
        ));
    }

    #[wasm_bindgen_test]
    fn hybrid_surface_manager_starts_initializing_when_first_surface_frame_arrives() {
        let host = document()
            .create_element("div")
            .expect("host element")
            .dyn_into::<HtmlElement>()
            .expect("html element");
        document()
            .body()
            .expect("body")
            .append_child(&host)
            .expect("mount host");

        let mut manager = HybridSurfaceManager::new(&host).expect("surface manager");
        assert_eq!(manager.status(), HybridSurfaceStatus::Uninitialized);

        let frames = vec![SurfaceFrame {
            css_bounds: Rect::new(0.0, 0.0, 32.0, 24.0),
            scale_factor: 1.0,
            elapsed_time: 0.0,
            draw_list: DrawList::new(),
        }];
        let outcome = manager.render(&frames, Color::TRANSPARENT).expect("render");

        assert_eq!(manager.status(), HybridSurfaceStatus::Initializing);
        assert!(!outcome.needs_retry);
    }

    #[wasm_bindgen_test]
    fn task_runtime_surfaces_worker_startup_failures_as_results() {
        let runtime = TaskRuntime::try_new().expect("task runtime");
        runtime.set_worker_script_url("missing-sparsha-worker.js");
        runtime.spawn("echo", json!({ "value": 1 }));

        let mut results = Vec::new();
        runtime.drain_completed(|result| results.push(result));

        assert_eq!(results.len(), 1);
        assert!(matches!(results[0].status, TaskStatus::Error(_)));
    }
}
