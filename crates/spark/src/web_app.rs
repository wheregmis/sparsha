//! Web runtime that renders Spark draw commands into the DOM.

#![cfg(target_arch = "wasm32")]

use crate::tasks::{TaskRuntime, TaskStatus};
use crate::{
    app::AppConfig,
    dom_renderer::DomRenderer,
    web_surface_manager::{HybridSurfaceManager, SurfaceFrame},
};
use spark_core::Color;
use spark_input::{FocusManager, InputEvent, PointerButton};
use spark_layout::LayoutTree;
use spark_render::DrawList;
use spark_signals::{RuntimeHandle, SubscriberKind};
use spark_text::TextSystem;
use spark_widgets::{
    BuildContext, EventCommands, EventContext, LayoutContext, PaintCommands, PaintContext, Widget,
};
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};
use wasm_bindgen::{closure::Closure, JsCast};
use web_sys::{CustomEvent, KeyboardEvent as WebKeyboardEvent, MouseEvent, WheelEvent, Window};

pub(crate) fn run_dom_app<F>(config: AppConfig, build_ui: F)
where
    F: FnOnce() -> Box<dyn Widget> + 'static,
{
    let window = web_sys::window().expect("window should be available");
    let document = window.document().expect("document should be available");
    let dom_renderer = DomRenderer::mount_to_body(&document).expect("failed to mount DOM renderer");
    let surface_manager =
        HybridSurfaceManager::new(dom_renderer.root()).expect("failed to initialize hybrid surface manager");
    let signal_runtime = RuntimeHandle::new();
    let task_runtime = TaskRuntime::new();
    task_runtime.set_worker_script_url("spark-worker.js?v=2");
    task_runtime.set_current();
    let root_widget = signal_runtime.run_with_current(build_ui);

    let mut state = WebAppState {
        config,
        dom_renderer,
        text_system: TextSystem::new_headless(),
        draw_list: DrawList::new(),
        surface_frames: Vec::new(),
        layout_tree: LayoutTree::new(),
        focus_manager: FocusManager::new(),
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
    };
    state.update_viewport();
    state.surface_manager.start_init();

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
}

struct WebAppState {
    config: AppConfig,
    dom_renderer: DomRenderer,
    text_system: TextSystem,
    draw_list: DrawList,
    surface_frames: Vec<SurfaceFrame>,
    layout_tree: LayoutTree,
    focus_manager: FocusManager,
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
            let mut surface_ctx = spark_widgets::DrawSurfaceContext {
                draw_list: &mut surface_draw_list,
                bounds: spark_core::Rect::new(
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

        match CustomEvent::new("SparkFirstPaint") {
            Ok(event) => {
                let _ = window.dispatch_event(event.as_ref());
            }
            Err(err) => {
                log::warn!("failed to emit SparkFirstPaint event: {:?}", err);
            }
        }
    }

    fn build_layout(&mut self) {
        let runtime = self.signal_runtime.clone();
        self.layout_tree = LayoutTree::new();

        runtime.with_tracking(SubscriberKind::Rebuild, || {
            fn rebuild_widget(widget: &mut dyn Widget, build_ctx: &mut BuildContext) {
                widget.rebuild(build_ctx);
                for child in widget.children_mut() {
                    rebuild_widget(child.as_mut(), build_ctx);
                }
            }

            let mut build_ctx = BuildContext::default();
            rebuild_widget(self.root_widget.as_mut(), &mut build_ctx);
        });

        fn add_to_layout(
            widget: &mut dyn Widget,
            tree: &mut LayoutTree,
            text_system: &mut TextSystem,
            in_scroll: bool,
        ) -> spark_layout::WidgetId {
            use spark_layout::taffy::Dimension;

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
            add_to_layout(
                self.root_widget.as_mut(),
                &mut self.layout_tree,
                &mut self.text_system,
                false,
            )
        });
        self.layout_tree.set_root(root_id);
        self.layout_tree
            .compute_layout(self.viewport_width.max(1.0), self.viewport_height.max(1.0));
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
        self.needs_repaint = paint_commands.request_next_frame;
    }

    fn handle_event(&mut self, event: InputEvent) {
        fn dispatch_event(
            widget: &mut dyn Widget,
            layout_tree: &LayoutTree,
            focus_id: Option<spark_layout::WidgetId>,
            event: &InputEvent,
            aggregate: &mut EventCommands,
        ) -> Option<spark_layout::WidgetId> {
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

        let runtime = self.signal_runtime.clone();
        let current_focus = self.focus_manager.focused();
        let mut commands = EventCommands::default();
        let new_focus = runtime.run_with_current(|| {
            dispatch_event(
                self.root_widget.as_mut(),
                &self.layout_tree,
                current_focus,
                &event,
                &mut commands,
            )
        });

        if let Some(fid) = new_focus {
            self.focus_manager.set_focus(fid);
        } else if current_focus.is_some() && new_focus.is_none() {
            self.focus_manager.clear_focus();
        }

        if commands.request_paint {
            self.needs_repaint = true;
        }
        if commands.request_layout {
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
        if self.needs_repaint {
            self.paint();
            if let Err(err) = self
                .dom_renderer
                .render(&self.draw_list, self.config.background)
            {
                log::error!("dom render failed: {:?}", err);
            } else {
                log::trace!(
                    "dom frame: active_nodes={} mutated_nodes={}",
                    self.dom_renderer.active_node_count(),
                    self.dom_renderer.mutated_node_count()
                );
                self.emit_first_paint_event();
            }
            if let Err(err) = self
                .surface_manager
                .render(&self.surface_frames, Color::TRANSPARENT)
            {
                log::error!("hybrid surface render failed: {:?}", err);
            }
        }
    }

    fn should_schedule_frame(&self) -> bool {
        self.needs_layout || self.needs_repaint || self.task_runtime.has_in_flight()
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
    let cb = cb_ref
        .as_ref()
        .expect("animation callback should be initialized");
    let _ = window.request_animation_frame(cb.as_ref().unchecked_ref());
}

fn install_event_listeners(
    window: &Window,
    state: &Rc<RefCell<WebAppState>>,
    pending_animation_frame: &Rc<Cell<bool>>,
    frame_cb: &Rc<RefCell<Option<Closure<dyn FnMut(f64)>>>>,
) {
    let root = state.borrow().dom_renderer.root().clone();

    {
        let state = Rc::clone(state);
        let pending_animation_frame = Rc::clone(pending_animation_frame);
        let frame_cb = Rc::clone(frame_cb);
        let window = window.clone();
        let target = root.clone();
        let root_for_event = target.clone();
        let on_move = Closure::wrap(Box::new(move |event: MouseEvent| {
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
        let on_up = Closure::wrap(Box::new(move |event: MouseEvent| {
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
                let code = spark_input::ui_events::keyboard::Code::Unidentified;
                let kb_event = spark_input::KeyboardEvent::key_down(key, code);
                state
                    .borrow_mut()
                    .handle_event(InputEvent::KeyDown { event: kb_event });
            }

            if should_emit_text(&event) {
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
                let code = spark_input::ui_events::keyboard::Code::Unidentified;
                let kb_event = spark_input::KeyboardEvent::key_up(key, code);
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
        let state = Rc::clone(state);
        let pending_animation_frame = Rc::clone(pending_animation_frame);
        let frame_cb = Rc::clone(frame_cb);
        let window = window.clone();
        let on_focus = Closure::wrap(Box::new(move || {
            state.borrow_mut().handle_event(InputEvent::FocusGained);
            if state.borrow().should_schedule_frame() {
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
            state.borrow_mut().handle_event(InputEvent::FocusLost);
            if state.borrow().should_schedule_frame() {
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
        let window_for_resize = window.clone();
        let on_resize = Closure::wrap(Box::new(move || {
            let mut state_ref = state.borrow_mut();
            state_ref.update_viewport();
            state_ref.needs_layout = true;
            let should_schedule = state_ref.should_schedule_frame();
            drop(state_ref);
            if should_schedule {
                schedule_animation_frame(
                    &window_for_resize,
                    &pending_animation_frame,
                    &frame_cb,
                );
            }
        }) as Box<dyn FnMut()>);
        let _ =
            window.add_event_listener_with_callback("resize", on_resize.as_ref().unchecked_ref());
        on_resize.forget();
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

fn should_emit_text(event: &WebKeyboardEvent) -> bool {
    let key = event.key();
    key.chars().count() == 1 && !event.ctrl_key() && !event.alt_key() && !event.meta_key()
}

fn map_browser_key(key: String) -> Option<spark_input::Key> {
    use spark_input::{Key, NamedKey};
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
    use spark_core::{Color, Rect};
    use spark_layout::taffy;
    use spark_widgets::DrawSurface;

    struct HybridOverlayWidget {
        id: spark_layout::WidgetId,
        surface: DrawSurface,
    }

    impl HybridOverlayWidget {
        fn new() -> Self {
            Self {
                id: spark_layout::WidgetId::default(),
                surface: DrawSurface::new(|ctx| {
                    ctx.fill_rect(ctx.bounds, Color::from_hex(0x112233));
                })
                .fill(),
            }
        }
    }

    impl Widget for HybridOverlayWidget {
        fn id(&self) -> spark_layout::WidgetId {
            self.id
        }

        fn set_id(&mut self, id: spark_layout::WidgetId) {
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
