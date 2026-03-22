//! Web runtime that renders Sparsha draw commands into the DOM.

#![cfg(target_arch = "wasm32")]

use crate::tasks::{TaskRuntime, TaskStatus};
use crate::{
    accessibility::{AccessibilityTreeSnapshot, ACCESSIBILITY_ROOT_ID},
    app::{AppConfig, AppRunError, AppTheme},
    component::ComponentStateStore,
    dom_renderer::{DomFrameSnapshot, DomRenderer},
    router::{hash_to_path, path_to_hash, Navigator, Router, RouterHost},
    runtime_widget::{
        add_widget_to_layout, apply_focus_change, collect_accessibility_tree,
        dispatch_widget_event, move_focus_path, remap_path, sync_focus_manager, with_widget_mut,
        WidgetPath, WidgetRuntimeRegistry,
    },
    web_surface_manager::{HybridSurfaceManager, HybridSurfaceStatus, SurfaceFrame},
};
use sparsha_core::Color;
use sparsha_input::{
    Action, ActionMapper, FocusManager, InputEvent, Modifiers, PointerButton, StandardAction,
};
use sparsha_layout::LayoutTree;
use sparsha_render::DrawList;
use sparsha_signals::{RuntimeHandle, SubscriberKind};
use sparsha_text::TextSystem;
use sparsha_widgets::{
    set_current_theme, set_current_viewport, AccessibilityAction, AccessibilityRole, BuildContext,
    PaintCommands, PaintContext, TextEditorState, ViewportInfo, Widget,
};
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};
use wasm_bindgen::{closure::Closure, JsCast};
use web_sys::{
    ClipboardEvent, CompositionEvent as WebCompositionEvent, CustomEvent, Document, Element, Event,
    HtmlElement, HtmlInputElement, HtmlTextAreaElement, InputEvent as WebInputEvent,
    KeyboardEvent as WebKeyboardEvent, MouseEvent, TouchEvent, WheelEvent, Window,
};

fn format_js_error(error: &wasm_bindgen::JsValue) -> String {
    error.as_string().unwrap_or_else(|| format!("{error:?}"))
}

pub(crate) fn run_dom_app(
    config: AppConfig,
    theme: AppTheme,
    router: Router,
) -> Result<(), AppRunError> {
    let window = web_sys::window().ok_or(AppRunError::WebEnvironment("window"))?;
    let document = window
        .document()
        .ok_or(AppRunError::WebEnvironment("document"))?;
    let dom_renderer = DomRenderer::mount_to_body(&document)
        .map_err(|err| AppRunError::DomMount(format_js_error(&err)))?;
    let surface_manager = HybridSurfaceManager::new(dom_renderer.root())
        .map_err(|err| AppRunError::HybridSurfaceInit(format_js_error(&err)))?;
    let text_input_bridge = WebTextInputBridge::new(&document, dom_renderer.root())
        .map_err(|err| AppRunError::DomMount(format_js_error(&err)))?;
    let semantic_dom = WebSemanticDomLayer::new(&document, dom_renderer.root())
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

    let mut state = WebAppState {
        config,
        theme,
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
        scale_factor: 1.0,
        viewport_width: 0.0,
        viewport_height: 0.0,
        needs_layout: true,
        needs_repaint: true,
        first_paint_emitted: false,
        surface_manager,
        text_input_bridge,
        semantic_dom,
        pending_clipboard_write: None,
        ime_composing: false,
        accessibility_text_focus_node: None,
        pending_surface_retry: false,
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
    scale_factor: f32,
    viewport_width: f32,
    viewport_height: f32,
    needs_layout: bool,
    needs_repaint: bool,
    first_paint_emitted: bool,
    surface_manager: HybridSurfaceManager,
    text_input_bridge: WebTextInputBridge,
    semantic_dom: WebSemanticDomLayer,
    pending_clipboard_write: Option<String>,
    ime_composing: bool,
    accessibility_text_focus_node: Option<u64>,
    pending_surface_retry: bool,
}

struct WebFrameSnapshot<'a> {
    draw_list: &'a DrawList,
    surface_frames: &'a [SurfaceFrame],
    background: Color,
    viewport_width: f32,
    viewport_height: f32,
    accessibility: AccessibilityTreeSnapshot,
}

struct WebTextInputBridge {
    element: HtmlTextAreaElement,
    syncing: bool,
}

impl WebTextInputBridge {
    fn new(
        document: &web_sys::Document,
        root: &web_sys::HtmlElement,
    ) -> Result<Self, wasm_bindgen::JsValue> {
        let element = document
            .create_element("textarea")?
            .dyn_into::<HtmlTextAreaElement>()?;
        element.set_class_name("sparsha-text-editor-bridge");
        element.set_attribute("aria-hidden", "true")?;
        element.set_attribute("autocomplete", "off")?;
        element.set_attribute("autocorrect", "off")?;
        element.set_attribute("autocapitalize", "off")?;
        element.set_attribute("spellcheck", "false")?;
        let style = element.style();
        style.set_property("position", "absolute")?;
        style.set_property("left", "-10000px")?;
        style.set_property("top", "0")?;
        style.set_property("width", "1px")?;
        style.set_property("height", "1px")?;
        style.set_property("opacity", "0")?;
        style.set_property("pointer-events", "none")?;
        style.set_property("resize", "none")?;
        style.set_property("overflow", "hidden")?;
        style.set_property("white-space", "pre")?;
        root.append_child(&element)?;
        Ok(Self {
            element,
            syncing: false,
        })
    }

    fn is_syncing(&self) -> bool {
        self.syncing
    }

    fn element(&self) -> &HtmlTextAreaElement {
        &self.element
    }

    fn sync(&mut self, editor_state: Option<&sparsha_widgets::TextEditorState>) {
        self.syncing = true;
        if let Some(state) = editor_state {
            if self.element.value() != state.text {
                self.element.set_value(&state.text);
            }
            let (selection_start, selection_end) = state.selection_range();
            let _ = self
                .element
                .set_selection_start(Some(selection_start as u32));
            let _ = self.element.set_selection_end(Some(selection_end as u32));
            let _ = self.element.focus();
        } else {
            if !self.element.value().is_empty() {
                self.element.set_value("");
            }
            let _ = self.element.blur();
        }
        self.syncing = false;
    }
}

struct WebSemanticDomLayer {
    root: HtmlElement,
}

impl WebSemanticDomLayer {
    fn new(document: &Document, parent: &HtmlElement) -> Result<Self, wasm_bindgen::JsValue> {
        let root = document.create_element("div")?.dyn_into::<HtmlElement>()?;
        root.set_class_name("sparsha-semantic-root");
        let style = root.style();
        style.set_property("position", "absolute")?;
        style.set_property("inset", "0")?;
        style.set_property("pointer-events", "none")?;
        style.set_property("overflow", "visible")?;
        style.set_property("background", "transparent")?;
        style.set_property("z-index", "10")?;
        parent.append_child(&root)?;
        Ok(Self { root })
    }

    fn root(&self) -> &HtmlElement {
        &self.root
    }

    fn render(&self, snapshot: &AccessibilityTreeSnapshot) -> Result<(), wasm_bindgen::JsValue> {
        self.root.set_inner_html("");
        let Some(document) = self.root.owner_document() else {
            return Ok(());
        };

        let nodes = snapshot
            .nodes
            .iter()
            .map(|node| (node.id, node))
            .collect::<std::collections::HashMap<_, _>>();
        for node_id in &snapshot.root_children {
            self.append_node(&document, &nodes, *node_id, None)?;
        }

        if snapshot.focus != ACCESSIBILITY_ROOT_ID {
            if let Some(element) = self
                .root
                .query_selector(&format!("[data-sparsha-a11y-node=\"{}\"]", snapshot.focus))?
                .and_then(|element| element.dyn_into::<HtmlElement>().ok())
            {
                let active = document
                    .active_element()
                    .and_then(|node| node.get_attribute("data-sparsha-a11y-node"));
                if active.as_deref() != Some(&snapshot.focus.to_string()) {
                    let _ = element.focus();
                }
            }
        }

        Ok(())
    }

    fn append_node(
        &self,
        document: &Document,
        nodes: &std::collections::HashMap<u64, &crate::accessibility::AccessibilityNodeSnapshot>,
        node_id: u64,
        parent_bounds: Option<sparsha_core::Rect>,
    ) -> Result<(), wasm_bindgen::JsValue> {
        let Some(node) = nodes.get(&node_id).copied() else {
            return Ok(());
        };
        let element = create_semantic_element(document, node)?;
        apply_semantic_bounds(&element, node.bounds, parent_bounds)?;
        self.root.append_child(&element)?;
        for child_id in &node.children {
            self.append_child_node(document, &element, nodes, *child_id, Some(node.bounds))?;
        }
        Ok(())
    }

    fn append_child_node(
        &self,
        document: &Document,
        parent: &HtmlElement,
        nodes: &std::collections::HashMap<u64, &crate::accessibility::AccessibilityNodeSnapshot>,
        node_id: u64,
        parent_bounds: Option<sparsha_core::Rect>,
    ) -> Result<(), wasm_bindgen::JsValue> {
        let Some(node) = nodes.get(&node_id).copied() else {
            return Ok(());
        };
        let element = create_semantic_element(document, node)?;
        apply_semantic_bounds(&element, node.bounds, parent_bounds)?;
        parent.append_child(&element)?;
        for child_id in &node.children {
            self.append_child_node(document, &element, nodes, *child_id, Some(node.bounds))?;
        }
        Ok(())
    }
}

fn create_semantic_element(
    document: &Document,
    node: &crate::accessibility::AccessibilityNodeSnapshot,
) -> Result<HtmlElement, wasm_bindgen::JsValue> {
    let element = match node.role {
        AccessibilityRole::Button => document
            .create_element("button")?
            .dyn_into::<HtmlElement>()?,
        AccessibilityRole::CheckBox => {
            let input = document
                .create_element("input")?
                .dyn_into::<HtmlInputElement>()?;
            input.set_type("checkbox");
            input.set_checked(node.checked.unwrap_or(false));
            input.unchecked_into::<HtmlElement>()
        }
        AccessibilityRole::TextInput => {
            let input = document
                .create_element("input")?
                .dyn_into::<HtmlInputElement>()?;
            input.set_type("text");
            input.set_value(node.value.as_deref().unwrap_or_default());
            input.unchecked_into::<HtmlElement>()
        }
        AccessibilityRole::MultilineTextInput => {
            let textarea = document
                .create_element("textarea")?
                .dyn_into::<HtmlTextAreaElement>()?;
            textarea.set_value(node.value.as_deref().unwrap_or_default());
            textarea.unchecked_into::<HtmlElement>()
        }
        _ => document.create_element("div")?.dyn_into::<HtmlElement>()?,
    };

    let style = element.style();
    style.set_property("position", "absolute")?;
    style.set_property("pointer-events", "none")?;
    style.set_property("opacity", "0")?;
    style.set_property("background", "transparent")?;
    style.set_property("border", "0")?;
    style.set_property("margin", "0")?;
    style.set_property("padding", "0")?;
    style.set_property("overflow", "hidden")?;
    style.set_property("color", "transparent")?;
    style.set_property("caret-color", "transparent")?;
    style.set_property("outline", "none")?;

    element.set_attribute("data-sparsha-a11y-node", &node.id.to_string())?;
    if node.hidden {
        style.set_property("display", "none")?;
    }
    if node.disabled {
        element.set_attribute("disabled", "true")?;
        element.set_attribute("aria-disabled", "true")?;
    }
    if let Some(label) = &node.label {
        element.set_attribute("aria-label", label)?;
        if matches!(
            node.role,
            AccessibilityRole::Label | AccessibilityRole::Button
        ) {
            element.set_text_content(Some(label));
        }
    }
    if let Some(description) = &node.description {
        element.set_attribute("aria-description", description)?;
    }
    if let Some(value) = &node.value {
        match node.role {
            AccessibilityRole::Label => element.set_text_content(Some(value)),
            AccessibilityRole::ScrollView => {
                element.set_attribute("aria-valuetext", value)?;
            }
            AccessibilityRole::GenericContainer | AccessibilityRole::List => {
                element.set_attribute("aria-label", value)?;
            }
            _ => {}
        }
    }
    if let Some(checked) = node.checked {
        element.set_attribute("aria-checked", if checked { "true" } else { "false" })?;
    }

    match node.role {
        AccessibilityRole::GenericContainer => {
            element.set_attribute("role", "group")?;
        }
        AccessibilityRole::List => {
            element.set_attribute("role", "list")?;
        }
        AccessibilityRole::ScrollView => {
            element.set_attribute("role", "group")?;
            element.set_attribute("aria-roledescription", "scroll view")?;
        }
        AccessibilityRole::Label => {}
        AccessibilityRole::Button
        | AccessibilityRole::CheckBox
        | AccessibilityRole::TextInput
        | AccessibilityRole::MultilineTextInput => {}
    }

    if node.actions.contains(&AccessibilityAction::Focus)
        && !matches!(
            node.role,
            AccessibilityRole::Button
                | AccessibilityRole::CheckBox
                | AccessibilityRole::TextInput
                | AccessibilityRole::MultilineTextInput
        )
    {
        element.set_tab_index(0);
    }

    Ok(element)
}

fn apply_semantic_bounds(
    element: &HtmlElement,
    bounds: sparsha_core::Rect,
    parent_bounds: Option<sparsha_core::Rect>,
) -> Result<(), wasm_bindgen::JsValue> {
    let origin_x = parent_bounds.map(|rect| rect.x).unwrap_or(0.0);
    let origin_y = parent_bounds.map(|rect| rect.y).unwrap_or(0.0);
    let style = element.style();
    style.set_property("left", &format!("{}px", bounds.x - origin_x))?;
    style.set_property("top", &format!("{}px", bounds.y - origin_y))?;
    style.set_property("width", &format!("{}px", bounds.width.max(1.0)))?;
    style.set_property("height", &format!("{}px", bounds.height.max(1.0)))?;
    Ok(())
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

    fn focused_text_editor_state(&self) -> Option<&sparsha_widgets::TextEditorState> {
        self.focused_path
            .as_ref()
            .and_then(|path| self.widget_registry.text_editor_state_for_path(path))
    }

    fn focused_selection_text(&self) -> Option<String> {
        let state = self.focused_text_editor_state()?;
        let (start, end) = state.selection_range();
        (start < end).then(|| state.text.get(start..end).unwrap_or_default().to_owned())
    }

    fn refresh_accessibility(&mut self) {
        self.widget_registry.accessibility = collect_accessibility_tree(
            self.root_widget.as_ref(),
            &self.layout_tree,
            self.focused_path.as_ref(),
        );
    }

    fn accessibility_text_focus_matches_widget_focus(&self) -> bool {
        let Some(node_id) = self.accessibility_text_focus_node else {
            return false;
        };
        let Some(path) = self.widget_registry.path_for_accessibility_node(node_id) else {
            return false;
        };
        self.focused_path.as_deref() == Some(path)
            && self
                .widget_registry
                .text_editor_state_for_path(path)
                .is_some()
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
        if suppress_text_bridge {
            self.text_input_bridge.sync(None);
            return;
        }
        self.text_input_bridge.sync(editor_state);
    }

    fn handle_accessibility_action(
        &mut self,
        node_id: u64,
        action: AccessibilityAction,
        value: Option<String>,
    ) {
        let Some(path) = self
            .widget_registry
            .path_for_accessibility_node(node_id)
            .map(ToOwned::to_owned)
        else {
            return;
        };

        match action {
            AccessibilityAction::Focus => {
                let focus_changed = apply_focus_change(
                    self.root_widget.as_mut(),
                    &mut self.focus_manager,
                    &self.widget_registry,
                    &mut self.focused_path,
                    Some(path),
                );
                if focus_changed {
                    self.ime_composing = false;
                    self.needs_repaint = true;
                }
            }
            action => {
                let handled = with_widget_mut(self.root_widget.as_mut(), &path, |widget| {
                    widget.handle_accessibility_action(action, value.clone())
                })
                .unwrap_or(false);
                if handled {
                    if matches!(action, AccessibilityAction::SetValue) {
                        self.needs_layout = true;
                    }
                    self.needs_repaint = true;
                }
            }
        }

        self.signal_runtime.run_effects(64);
        let dirty = self.signal_runtime.take_dirty_flags();
        if dirty.rebuild || dirty.layout {
            self.needs_layout = true;
        }
        if dirty.paint {
            self.needs_repaint = true;
        }
        self.refresh_accessibility();
        self.sync_text_input_bridge();
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

    fn desired_route_hash(&self) -> String {
        path_to_hash(&self.router_navigator.current_path())
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
        let runtime = self.signal_runtime.clone();
        self.layout_tree = LayoutTree::new();
        self.component_states.begin_rebuild();

        runtime.with_tracking(SubscriberKind::Rebuild, || {
            let resolved_theme = self.theme.resolve_theme();
            let navigator = self.router_navigator.clone();
            let viewport = self.logical_viewport();
            set_current_theme(resolved_theme.clone());
            set_current_viewport(viewport);

            fn rebuild_widget(
                widget: &mut dyn Widget,
                build_ctx: &mut BuildContext,
                path: &mut Vec<usize>,
            ) {
                build_ctx.set_path(path);
                widget.rebuild(build_ctx);
                let child_keys: Vec<_> = (0..widget.children().len())
                    .map(|index| widget.child_path_key(index))
                    .collect();
                for (index, child) in widget.children_mut().iter_mut().enumerate() {
                    path.push(child_keys[index]);
                    rebuild_widget(child.as_mut(), build_ctx, path);
                    path.pop();
                }
            }

            fn persist_widget_state(
                widget: &dyn Widget,
                build_ctx: &mut BuildContext,
                path: &mut Vec<usize>,
            ) {
                build_ctx.set_path(path);
                widget.persist_build_state(build_ctx);
                let child_keys: Vec<_> = (0..widget.children().len())
                    .map(|index| widget.child_path_key(index))
                    .collect();
                for (index, child) in widget.children().iter().enumerate() {
                    path.push(child_keys[index]);
                    persist_widget_state(child.as_ref(), build_ctx, path);
                    path.pop();
                }
            }

            let mut build_ctx = BuildContext::default();
            build_ctx.set_theme(resolved_theme);
            build_ctx.insert_resource(navigator);
            build_ctx.insert_resource(self.task_runtime.clone());
            build_ctx.insert_resource(self.signal_runtime.clone());
            build_ctx.insert_resource(viewport);
            // SAFETY: the rebuild pass owns `component_states` for the entire
            // lifetime of `build_ctx` and does not alias it elsewhere.
            unsafe { build_ctx.set_state_store(&mut self.component_states) };
            let mut path = Vec::new();
            persist_widget_state(self.root_widget.as_ref(), &mut build_ctx, &mut path);
            self.component_states.begin_rebuild();
            path.clear();
            rebuild_widget(self.root_widget.as_mut(), &mut build_ctx, &mut path);
        });
        self.component_states.finish_rebuild();

        let mut widget_registry = WidgetRuntimeRegistry::default();
        let root_id = runtime.with_tracking(SubscriberKind::Layout, || {
            set_current_theme(self.theme.resolve_theme());
            set_current_viewport(self.logical_viewport());
            let mut path = Vec::new();
            add_widget_to_layout(
                self.root_widget.as_mut(),
                &mut self.layout_tree,
                &mut self.text_system,
                &mut widget_registry,
                &mut path,
                false,
                true,
            )
        });
        self.layout_tree.set_root(root_id);
        self.widget_registry = widget_registry;
        self.layout_tree
            .compute_layout(self.viewport_width.max(1.0), self.viewport_height.max(1.0));
        self.focused_path = remap_path(self.focused_path.take(), &self.widget_registry);
        self.capture_path = remap_path(self.capture_path.take(), &self.widget_registry);
        sync_focus_manager(
            &mut self.focus_manager,
            &self.widget_registry,
            self.focused_path.as_ref(),
        );
        self.refresh_accessibility();
        self.sync_text_input_bridge();
        self.needs_layout = false;
        self.needs_repaint = true;
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
        let runtime = self.signal_runtime.clone();
        let mapper = ActionMapper::new();
        let mut handled_focus_navigation = false;
        if let Some(Action::Standard(action)) = mapper.map_event(&event) {
            match action {
                StandardAction::FocusNext | StandardAction::FocusPrevious => {
                    let next_focus = move_focus_path(
                        self.focused_path.as_ref(),
                        &self.widget_registry,
                        matches!(action, StandardAction::FocusNext),
                    );
                    let focus_changed = apply_focus_change(
                        self.root_widget.as_mut(),
                        &mut self.focus_manager,
                        &self.widget_registry,
                        &mut self.focused_path,
                        next_focus,
                    );
                    if focus_changed {
                        self.ime_composing = false;
                        self.sync_text_input_bridge();
                        self.needs_repaint = true;
                    }
                    handled_focus_navigation = true;
                }
                _ => {}
            }
        }
        if handled_focus_navigation {
            self.refresh_accessibility();
            return;
        }

        let current_focus_id = self
            .focused_path
            .as_ref()
            .and_then(|path| self.widget_registry.id_for_path(path));
        let current_capture_path = self.capture_path.clone();
        let outcome = runtime.run_with_current(|| {
            dispatch_widget_event(
                self.root_widget.as_mut(),
                &self.layout_tree,
                current_focus_id,
                current_capture_path.as_ref(),
                &event,
            )
        });

        if outcome.commands.request_focus || outcome.commands.clear_focus {
            let focus_changed = apply_focus_change(
                self.root_widget.as_mut(),
                &mut self.focus_manager,
                &self.widget_registry,
                &mut self.focused_path,
                outcome.focus_path.clone(),
            );
            if focus_changed {
                self.ime_composing = false;
                self.needs_repaint = true;
            }
        }

        if outcome.commands.capture_pointer || outcome.commands.release_pointer {
            self.capture_path = outcome.capture_path;
            self.needs_repaint = true;
        }

        if outcome.commands.clipboard_write.is_some() {
            self.pending_clipboard_write = outcome.commands.clipboard_write.clone();
        }

        self.sync_text_input_bridge();

        if outcome.commands.request_paint {
            self.needs_repaint = true;
        }
        if outcome.commands.request_layout {
            self.needs_layout = true;
        }

        runtime.run_effects(64);
        let dirty = runtime.take_dirty_flags();
        if dirty.rebuild || dirty.layout {
            self.needs_layout = true;
        }
        if dirty.paint {
            self.needs_repaint = true;
        }

        self.refresh_accessibility();
    }

    fn frame(&mut self) {
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
            let desired_hash = self.desired_route_hash();
            self.sync_route_hash(&desired_hash);
            return;
        }

        let focused_editor_state = self.focused_text_editor_state().cloned();
        let suppress_text_bridge = self.accessibility_text_focus_matches_widget_focus();
        let desired_hash = self.desired_route_hash();
        self.sync_route_hash(&desired_hash);
        self.sync_text_input_bridge_with_state(focused_editor_state.as_ref(), suppress_text_bridge);

        let (dom_rendered, pending_surface_retry) = {
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

            if let Err(err) = self.semantic_dom.render(&snapshot.accessibility) {
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
    let semantic_root = state.borrow().semantic_dom.root().clone();

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
            let button = mouse_button(event.button());
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
            let Some(pos) = touch_pos(&root_for_event, &event) else {
                return;
            };
            root_for_event.focus().ok();
            let mut state_ref = state.borrow_mut();
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
        let _ =
            target.add_event_listener_with_callback("touchstart", on_touch_start.as_ref().unchecked_ref());
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
            let button = mouse_button(event.button());
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
            if state.borrow().capture_path.is_some() {
                return;
            }
            let Some(pos) = touch_pos(&root_for_event, &event) else {
                return;
            };
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
            target.add_event_listener_with_callback("touchmove", on_touch_move.as_ref().unchecked_ref());
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
            if state.borrow().capture_path.is_none() {
                return;
            }
            let Some(pos) = touch_pos(&root_for_event, &event) else {
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
        let _ =
            window.add_event_listener_with_callback("touchmove", on_move_captured.as_ref().unchecked_ref());
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
            let button = mouse_button(event.button());
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
            if state.borrow().capture_path.is_some() {
                return;
            }
            let Some(pos) = touch_pos(&root_for_event, &event) else {
                return;
            };
            let mut state_ref = state.borrow_mut();
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
            if state.borrow().capture_path.is_none() {
                return;
            }
            let Some(pos) = touch_pos(&root_for_event, &event) else {
                return;
            };
            let mut state_ref = state.borrow_mut();
            state_ref.mouse_pos = pos;
            state_ref.handle_event(InputEvent::PointerUp {
                pos,
                button: PointerButton::Primary,
            });
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
            let Some(pos) = touch_pos(&root_for_event, &event) else {
                return;
            };
            let mut state_ref = state.borrow_mut();
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
                delta_x /= 20.0;
                delta_y /= 20.0;
            }
            let mut state_ref = state.borrow_mut();
            state_ref.handle_event(InputEvent::Scroll {
                pos,
                delta: glam::Vec2::new(delta_x, delta_y),
                modifiers: browser_wheel_modifiers(&event),
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
            if let Some(key) = map_browser_key(event.key()) {
                let code = sparsha_input::ui_events::keyboard::Code::Unidentified;
                let mut kb_event = sparsha_input::KeyboardEvent::key_down(key, code);
                kb_event.modifiers = browser_modifiers(&event);
                let mapped = ActionMapper::new().map_event(&InputEvent::KeyDown {
                    event: kb_event.clone(),
                });
                let focused_text_editor = state.borrow().focused_text_editor_state().is_some();
                if focused_text_editor {
                    if let Some(Action::Standard(action)) = mapped {
                        if should_prevent_keydown_for_text_editor_action(&kb_event, action) {
                            event.prevent_default();
                        }
                    }
                } else if matches!(
                    mapped,
                    Some(Action::Standard(
                        StandardAction::FocusNext | StandardAction::FocusPrevious
                    ))
                ) {
                    event.prevent_default();
                }

                let browser_key = event.key();
                if should_forward_keydown_to_widget_tree(
                    focused_text_editor,
                    &browser_key,
                    event.ctrl_key(),
                    event.alt_key(),
                    event.meta_key(),
                ) {
                    state
                        .borrow_mut()
                        .handle_event(InputEvent::KeyDown { event: kb_event });
                }
            }

            if !state.borrow().focused_text_editor_state().is_some() && should_emit_text(&event) {
                state
                    .borrow_mut()
                    .handle_event(InputEvent::TextInput { text: event.key() });
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
            if let Some(key) = map_browser_key(event.key()) {
                let code = sparsha_input::ui_events::keyboard::Code::Unidentified;
                let mut kb_event = sparsha_input::KeyboardEvent::key_up(key, code);
                kb_event.modifiers = browser_modifiers(&event);
                state
                    .borrow_mut()
                    .handle_event(InputEvent::KeyUp { event: kb_event });
                if state.borrow().should_schedule_frame() {
                    schedule_animation_frame(&window, &pending_animation_frame, &frame_cb);
                }
            }
        }) as Box<dyn FnMut(_)>);
        let _ = root.add_event_listener_with_callback("keyup", on_key_up.as_ref().unchecked_ref());
        on_key_up.forget();
    }

    {
        let bridge = state.borrow().text_input_bridge.element().clone();
        let state = Rc::clone(state);
        let pending_animation_frame = Rc::clone(pending_animation_frame);
        let frame_cb = Rc::clone(frame_cb);
        let window = window.clone();
        let on_before_input = Closure::wrap(Box::new(move |event: WebInputEvent| {
            if state.borrow().text_input_bridge.is_syncing() {
                return;
            }
            if state.borrow().focused_text_editor_state().is_none() {
                return;
            }
            match event.input_type().as_str() {
                "insertText" => {
                    if let Some(text) = event.data() {
                        event.prevent_default();
                        state
                            .borrow_mut()
                            .handle_event(InputEvent::TextInput { text });
                    }
                }
                _ => {}
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
        let bridge = state.borrow().text_input_bridge.element().clone();
        let state = Rc::clone(state);
        let pending_animation_frame = Rc::clone(pending_animation_frame);
        let frame_cb = Rc::clone(frame_cb);
        let window = window.clone();
        let on_input = Closure::wrap(Box::new(move |_event: WebInputEvent| {
            if state.borrow().text_input_bridge.is_syncing() {
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
        let bridge = state.borrow().text_input_bridge.element().clone();
        let state = Rc::clone(state);
        let pending_animation_frame = Rc::clone(pending_animation_frame);
        let frame_cb = Rc::clone(frame_cb);
        let window = window.clone();
        let on_composition_start = Closure::wrap(Box::new(move |_event: WebCompositionEvent| {
            if state.borrow().text_input_bridge.is_syncing() {
                return;
            }
            if state.borrow().focused_text_editor_state().is_none() {
                return;
            }
            let mut state_ref = state.borrow_mut();
            state_ref.ime_composing = true;
            state_ref.handle_event(InputEvent::CompositionStart);
            let should_schedule = state_ref.should_schedule_frame();
            drop(state_ref);
            if should_schedule {
                schedule_animation_frame(&window, &pending_animation_frame, &frame_cb);
            }
        }) as Box<dyn FnMut(_)>);
        let _ = bridge.add_event_listener_with_callback(
            "compositionstart",
            on_composition_start.as_ref().unchecked_ref(),
        );
        on_composition_start.forget();
    }

    {
        let bridge = state.borrow().text_input_bridge.element().clone();
        let state = Rc::clone(state);
        let pending_animation_frame = Rc::clone(pending_animation_frame);
        let frame_cb = Rc::clone(frame_cb);
        let window = window.clone();
        let on_composition_update = Closure::wrap(Box::new(move |event: WebCompositionEvent| {
            if state.borrow().text_input_bridge.is_syncing() {
                return;
            }
            if state.borrow().focused_text_editor_state().is_none() {
                return;
            }
            let text = event.data().unwrap_or_default();
            state
                .borrow_mut()
                .handle_event(InputEvent::CompositionUpdate { text });
            if state.borrow().should_schedule_frame() {
                schedule_animation_frame(&window, &pending_animation_frame, &frame_cb);
            }
        }) as Box<dyn FnMut(_)>);
        let _ = bridge.add_event_listener_with_callback(
            "compositionupdate",
            on_composition_update.as_ref().unchecked_ref(),
        );
        on_composition_update.forget();
    }

    {
        let bridge = state.borrow().text_input_bridge.element().clone();
        let state = Rc::clone(state);
        let pending_animation_frame = Rc::clone(pending_animation_frame);
        let frame_cb = Rc::clone(frame_cb);
        let window = window.clone();
        let on_composition_end = Closure::wrap(Box::new(move |event: WebCompositionEvent| {
            if state.borrow().text_input_bridge.is_syncing() {
                return;
            }
            if state.borrow().focused_text_editor_state().is_none() {
                return;
            }
            let mut state_ref = state.borrow_mut();
            state_ref.ime_composing = false;
            state_ref.handle_event(InputEvent::CompositionEnd {
                text: event.data().unwrap_or_default(),
            });
            let should_schedule = state_ref.should_schedule_frame();
            drop(state_ref);
            if should_schedule {
                schedule_animation_frame(&window, &pending_animation_frame, &frame_cb);
            }
        }) as Box<dyn FnMut(_)>);
        let _ = bridge.add_event_listener_with_callback(
            "compositionend",
            on_composition_end.as_ref().unchecked_ref(),
        );
        on_composition_end.forget();
    }

    {
        let bridge = state.borrow().text_input_bridge.element().clone();
        let state = Rc::clone(state);
        let pending_animation_frame = Rc::clone(pending_animation_frame);
        let frame_cb = Rc::clone(frame_cb);
        let window = window.clone();
        let on_paste = Closure::wrap(Box::new(move |event: ClipboardEvent| {
            if state.borrow().focused_text_editor_state().is_none() {
                return;
            }
            let Some(clipboard) = event.clipboard_data() else {
                return;
            };
            let Ok(text) = clipboard.get_data("text/plain") else {
                return;
            };
            event.prevent_default();
            state.borrow_mut().handle_event(InputEvent::Paste { text });
            if state.borrow().should_schedule_frame() {
                schedule_animation_frame(&window, &pending_animation_frame, &frame_cb);
            }
        }) as Box<dyn FnMut(_)>);
        let _ = bridge.add_event_listener_with_callback("paste", on_paste.as_ref().unchecked_ref());
        on_paste.forget();
    }

    {
        let bridge = state.borrow().text_input_bridge.element().clone();
        let state = Rc::clone(state);
        let on_copy = Closure::wrap(Box::new(move |event: ClipboardEvent| {
            if state.borrow().focused_text_editor_state().is_none() {
                return;
            }
            let text = {
                let mut state_ref = state.borrow_mut();
                state_ref
                    .pending_clipboard_write
                    .take()
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
        let bridge = state.borrow().text_input_bridge.element().clone();
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
                state_ref.pending_clipboard_write.take()
            };

            if clipboard_text.is_none() && fallback_text.is_some() {
                let mut synthetic = sparsha_input::KeyboardEvent::key_down(
                    sparsha_input::Key::Character("x".to_owned()),
                    sparsha_input::ui_events::keyboard::Code::Unidentified,
                );
                synthetic.modifiers = primary_shortcut_modifiers();
                state
                    .borrow_mut()
                    .handle_event(InputEvent::KeyDown { event: synthetic });
                clipboard_text = state.borrow_mut().pending_clipboard_write.take();
                if clipboard_text.is_none() {
                    let mut ctrl_synthetic = sparsha_input::KeyboardEvent::key_down(
                        sparsha_input::Key::Character("x".to_owned()),
                        sparsha_input::ui_events::keyboard::Code::Unidentified,
                    );
                    ctrl_synthetic.modifiers = ctrl_shortcut_modifiers();
                    state.borrow_mut().handle_event(InputEvent::KeyDown {
                        event: ctrl_synthetic,
                    });
                    clipboard_text = state.borrow_mut().pending_clipboard_write.take();
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
            state_ref.accessibility_text_focus_node = is_text.then_some(node_id);
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
            state_ref.accessibility_text_focus_node = None;
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
            state_ref.accessibility_text_focus_node = Some(node_id);
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

fn mouse_button(button: i16) -> PointerButton {
    match button {
        0 => PointerButton::Primary,
        1 => PointerButton::Auxiliary,
        2 => PointerButton::Secondary,
        _ => PointerButton::Primary,
    }
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

fn touch_pos(root: &web_sys::HtmlElement, event: &TouchEvent) -> Option<glam::Vec2> {
    // Prefer changed_touches for end/cancel events where active touches can already be empty.
    // Fall back to touches/target_touches for start/move where the active contact is still present.
    let touch = event
        .changed_touches()
        .item(0)
        .or_else(|| event.touches().item(0))
        .or_else(|| event.target_touches().item(0))?;
    let rect = root.get_bounding_client_rect();
    Some(glam::Vec2::new(
        touch.client_x() as f32 - rect.left() as f32,
        touch.client_y() as f32 - rect.top() as f32,
    ))
}

fn browser_modifiers(event: &WebKeyboardEvent) -> Modifiers {
    browser_modifiers_from_flags(
        event.shift_key(),
        event.ctrl_key(),
        event.alt_key(),
        event.meta_key(),
    )
}

fn browser_wheel_modifiers(event: &WheelEvent) -> Modifiers {
    browser_modifiers_from_flags(
        event.shift_key(),
        event.ctrl_key(),
        event.alt_key(),
        event.meta_key(),
    )
}

fn browser_modifiers_from_flags(shift: bool, ctrl: bool, alt: bool, meta: bool) -> Modifiers {
    let mut modifiers = Modifiers::empty();
    if shift {
        modifiers |= Modifiers::SHIFT;
    }
    if ctrl {
        modifiers |= Modifiers::CONTROL;
    }
    if alt {
        modifiers |= Modifiers::ALT;
    }
    if meta {
        modifiers |= Modifiers::META;
    }
    modifiers
}

fn primary_shortcut_modifiers() -> Modifiers {
    Modifiers::META
}

fn ctrl_shortcut_modifiers() -> Modifiers {
    Modifiers::CONTROL
}

fn is_plain_printable_key(key: &str, ctrl: bool, alt: bool, meta: bool) -> bool {
    key.chars().count() == 1 && !ctrl && !alt && !meta
}

fn should_prevent_keydown_for_text_editor_action(
    event: &sparsha_input::KeyboardEvent,
    action: StandardAction,
) -> bool {
    matches!(
        action,
        StandardAction::FocusNext
            | StandardAction::FocusPrevious
            | StandardAction::SelectAll
            | StandardAction::Undo
            | StandardAction::Redo
            | StandardAction::Backspace
            | StandardAction::Delete
            | StandardAction::MoveLeft
            | StandardAction::MoveRight
            | StandardAction::MoveUp
            | StandardAction::MoveDown
            | StandardAction::MoveWordLeft
            | StandardAction::MoveWordRight
            | StandardAction::MoveToStart
            | StandardAction::MoveToEnd
            | StandardAction::SelectLeft
            | StandardAction::SelectRight
            | StandardAction::SelectUp
            | StandardAction::SelectDown
            | StandardAction::SelectWordLeft
            | StandardAction::SelectWordRight
            | StandardAction::SelectToStart
            | StandardAction::SelectToEnd
            | StandardAction::Cancel
    ) || matches!(action, StandardAction::Activate)
        && !matches!(&event.key, sparsha_input::Key::Character(value) if value == " ")
}

fn should_emit_text(event: &WebKeyboardEvent) -> bool {
    is_plain_printable_key(
        &event.key(),
        event.ctrl_key(),
        event.alt_key(),
        event.meta_key(),
    )
}

fn should_forward_keydown_to_widget_tree(
    focused_text_editor: bool,
    key: &str,
    ctrl: bool,
    alt: bool,
    meta: bool,
) -> bool {
    !focused_text_editor || !is_plain_printable_key(key, ctrl, alt, meta)
}

fn should_sync_external_hash(current_path: &str, hash: &str) -> bool {
    hash_to_path(hash) != current_path
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

fn map_browser_key(key: String) -> Option<sparsha_input::Key> {
    use sparsha_input::{Key, NamedKey};
    Some(match key.as_str() {
        "Enter" => Key::Named(NamedKey::Enter),
        "Tab" => Key::Named(NamedKey::Tab),
        "Backspace" => Key::Named(NamedKey::Backspace),
        "Delete" => Key::Named(NamedKey::Delete),
        "Escape" => Key::Named(NamedKey::Escape),
        "ArrowUp" => Key::Named(NamedKey::ArrowUp),
        "ArrowDown" => Key::Named(NamedKey::ArrowDown),
        "ArrowLeft" => Key::Named(NamedKey::ArrowLeft),
        "ArrowRight" => Key::Named(NamedKey::ArrowRight),
        "Home" => Key::Named(NamedKey::Home),
        "End" => Key::Named(NamedKey::End),
        "PageUp" => Key::Named(NamedKey::PageUp),
        "PageDown" => Key::Named(NamedKey::PageDown),
        value if value.chars().count() == 1 => Key::Character(value.to_string()),
        _ => return None,
    })
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
    use wasm_bindgen_test::*;

    wasm_bindgen_test_configure!(run_in_browser);

    fn document() -> Document {
        web_sys::window()
            .and_then(|window| window.document())
            .expect("window document")
    }

    #[wasm_bindgen_test]
    fn semantic_text_input_node_preserves_value_and_label() {
        let element = create_semantic_element(
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
