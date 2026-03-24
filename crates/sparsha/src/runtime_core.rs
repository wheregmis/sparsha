use crate::accessibility::AccessibilityTreeSnapshot;
use crate::app::AppTheme;
use crate::component::ComponentStateStore;
use crate::platform::{PlatformEffect, PlatformEffects};
use crate::runtime_widget::{
    add_widget_to_layout, apply_focus_change, apply_post_layout_measurements,
    collect_accessibility_tree, dispatch_widget_event, move_focus_path, remap_path,
    sync_focus_manager, with_widget_mut, WidgetPath, WidgetRuntimeRegistry,
};
use sparsha_input::{
    with_shortcut_profile, Action, ActionMapper, FocusManager, InputEvent, ShortcutProfile,
    StandardAction,
};
use sparsha_layout::LayoutTree;
use sparsha_signals::{RuntimeHandle, SubscriberKind};
use sparsha_text::TextSystem;
use sparsha_widgets::{
    set_current_theme, set_current_viewport, AccessibilityAction, BuildContext, TextEditorState,
    ViewportInfo, Widget,
};

pub(crate) struct RuntimePlatformUpdate {
    pub(crate) effects: PlatformEffects,
    #[cfg_attr(target_arch = "wasm32", allow(dead_code))]
    pub(crate) accessibility: AccessibilityTreeSnapshot,
    pub(crate) focused_editor_state: Option<TextEditorState>,
    pub(crate) has_capture: bool,
}

pub(crate) struct RuntimeCoreContext<'a> {
    pub(crate) theme: &'a AppTheme,
    pub(crate) navigator: crate::router::Navigator,
    pub(crate) root_widget: &'a mut dyn Widget,
    pub(crate) layout_tree: &'a mut LayoutTree,
    pub(crate) widget_registry: &'a mut WidgetRuntimeRegistry,
    pub(crate) component_states: &'a mut ComponentStateStore,
    pub(crate) focus_manager: &'a mut FocusManager,
    pub(crate) focused_path: &'a mut Option<WidgetPath>,
    pub(crate) capture_path: &'a mut Option<WidgetPath>,
    pub(crate) signal_runtime: RuntimeHandle,
    pub(crate) task_runtime: crate::tasks::TaskRuntime,
    pub(crate) text_system: &'a mut TextSystem,
    pub(crate) viewport: ViewportInfo,
    pub(crate) shortcut_profile: ShortcutProfile,
    pub(crate) ime_composing: &'a mut bool,
    pub(crate) needs_layout: &'a mut bool,
    pub(crate) needs_repaint: &'a mut bool,
}

pub(crate) struct RuntimeHost<'a> {
    theme: &'a AppTheme,
    navigator: crate::router::Navigator,
    root_widget: &'a mut dyn Widget,
    layout_tree: &'a mut LayoutTree,
    widget_registry: &'a mut WidgetRuntimeRegistry,
    component_states: &'a mut ComponentStateStore,
    focus_manager: &'a mut FocusManager,
    focused_path: &'a mut Option<WidgetPath>,
    capture_path: &'a mut Option<WidgetPath>,
    signal_runtime: RuntimeHandle,
    task_runtime: crate::tasks::TaskRuntime,
    text_system: &'a mut TextSystem,
    viewport: ViewportInfo,
    shortcut_profile: ShortcutProfile,
    ime_composing: &'a mut bool,
    needs_layout: &'a mut bool,
    needs_repaint: &'a mut bool,
}

impl<'a> From<RuntimeCoreContext<'a>> for RuntimeHost<'a> {
    fn from(ctx: RuntimeCoreContext<'a>) -> Self {
        Self {
            theme: ctx.theme,
            navigator: ctx.navigator,
            root_widget: ctx.root_widget,
            layout_tree: ctx.layout_tree,
            widget_registry: ctx.widget_registry,
            component_states: ctx.component_states,
            focus_manager: ctx.focus_manager,
            focused_path: ctx.focused_path,
            capture_path: ctx.capture_path,
            signal_runtime: ctx.signal_runtime,
            task_runtime: ctx.task_runtime,
            text_system: ctx.text_system,
            viewport: ctx.viewport,
            shortcut_profile: ctx.shortcut_profile,
            ime_composing: ctx.ime_composing,
            needs_layout: ctx.needs_layout,
            needs_repaint: ctx.needs_repaint,
        }
    }
}

impl RuntimeHost<'_> {
    fn platform_update(&mut self, mut effects: PlatformEffects) -> RuntimePlatformUpdate {
        effects.push(PlatformEffect::SyncAccessibility);
        RuntimePlatformUpdate {
            effects,
            accessibility: self.refresh_accessibility(),
            focused_editor_state: self.focused_text_editor_state().cloned(),
            has_capture: self.has_pointer_capture(),
        }
    }

    pub(crate) fn focused_text_editor_state(&self) -> Option<&TextEditorState> {
        focused_text_editor_state(self.widget_registry, self.focused_path.as_deref())
    }

    pub(crate) fn has_pointer_capture(&self) -> bool {
        self.capture_path.is_some()
    }

    pub(crate) fn refresh_accessibility(&mut self) -> AccessibilityTreeSnapshot {
        refresh_accessibility_tree(
            self.widget_registry,
            self.root_widget,
            self.layout_tree,
            self.focused_path.as_ref(),
        )
    }

    pub(crate) fn refresh_platform_update(&mut self) -> RuntimePlatformUpdate {
        self.platform_update(PlatformEffects::default())
    }

    pub(crate) fn build_layout(&mut self) -> PlatformEffects {
        let runtime = self.signal_runtime.clone();
        *self.layout_tree = LayoutTree::new();
        self.component_states.begin_rebuild();

        runtime.with_tracking(SubscriberKind::Rebuild, || {
            let resolved_theme = self.theme.resolve_theme();
            let navigator = self.navigator.clone();
            let viewport = self.viewport;
            set_current_theme(resolved_theme.clone());
            set_current_viewport(viewport);

            fn rebuild_widget(
                widget: &mut dyn Widget,
                build_ctx: &mut BuildContext,
                path: &mut Vec<usize>,
            ) {
                build_ctx.set_path(path);
                widget.rebuild(build_ctx);
                widget.enter_build_scope(build_ctx);
                let child_keys: Vec<_> = (0..widget.children().len())
                    .map(|index| widget.child_path_key(index))
                    .collect();
                for (index, child) in widget.children_mut().iter_mut().enumerate() {
                    path.push(child_keys[index]);
                    rebuild_widget(child.as_mut(), build_ctx, path);
                    path.pop();
                }
                widget.exit_build_scope(build_ctx);
            }

            fn persist_widget_state(
                widget: &dyn Widget,
                build_ctx: &mut BuildContext,
                path: &mut Vec<usize>,
            ) {
                build_ctx.set_path(path);
                widget.persist_build_state(build_ctx);
                widget.enter_build_scope(build_ctx);
                let child_keys: Vec<_> = (0..widget.children().len())
                    .map(|index| widget.child_path_key(index))
                    .collect();
                for (index, child) in widget.children().iter().enumerate() {
                    path.push(child_keys[index]);
                    persist_widget_state(child.as_ref(), build_ctx, path);
                    path.pop();
                }
                widget.exit_build_scope(build_ctx);
            }

            let mut build_ctx = BuildContext::default();
            build_ctx.set_theme(resolved_theme);
            build_ctx.insert_resource(navigator);
            build_ctx.insert_resource(self.task_runtime.clone());
            build_ctx.insert_resource(self.signal_runtime.clone());
            build_ctx.insert_resource(viewport);
            unsafe { build_ctx.set_state_store(self.component_states) };
            let mut path = Vec::new();
            persist_widget_state(self.root_widget, &mut build_ctx, &mut path);
            self.component_states.begin_rebuild();
            path.clear();
            rebuild_widget(self.root_widget, &mut build_ctx, &mut path);
        });
        self.component_states.finish_rebuild();

        let mut widget_registry = WidgetRuntimeRegistry::default();
        let root_id = runtime.with_tracking(SubscriberKind::Layout, || {
            set_current_theme(self.theme.resolve_theme());
            set_current_viewport(self.viewport);
            let mut path = Vec::new();
            add_widget_to_layout(
                self.root_widget,
                self.layout_tree,
                self.text_system,
                &mut widget_registry,
                &mut path,
                false,
                true,
            )
        });
        self.layout_tree.set_root(root_id);
        *self.widget_registry = widget_registry;
        self.layout_tree
            .compute_layout(self.viewport.width.max(1.0), self.viewport.height.max(1.0));
        if apply_post_layout_measurements(self.root_widget, self.layout_tree, self.text_system) {
            self.layout_tree
                .compute_layout(self.viewport.width.max(1.0), self.viewport.height.max(1.0));
        }
        *self.focused_path = remap_path(self.focused_path.take(), self.widget_registry);
        *self.capture_path = remap_path(self.capture_path.take(), self.widget_registry);
        sync_focus_manager(
            self.focus_manager,
            self.widget_registry,
            self.focused_path.as_ref(),
        );
        *self.needs_layout = false;
        *self.needs_repaint = true;

        let mut effects = PlatformEffects::default();
        effects.push(PlatformEffect::SyncTextInput);
        effects.push(PlatformEffect::SyncPointerCapture);
        effects
    }

    pub(crate) fn build_layout_update(&mut self) -> RuntimePlatformUpdate {
        let effects = self.build_layout();
        self.platform_update(effects)
    }

    pub(crate) fn handle_input_event(
        &mut self,
        event: InputEvent,
        clipboard_text: Option<String>,
    ) -> PlatformEffects {
        let mapper = ActionMapper::with_shortcut_profile(self.shortcut_profile);
        let mut handled_focus_navigation = false;
        let mut dispatch_event = event.clone();
        let mut effects = PlatformEffects::default();

        if let Some(Action::Standard(action)) = mapper.map_event(&event) {
            match action {
                StandardAction::FocusNext | StandardAction::FocusPrevious => {
                    let next_focus = move_focus_path(
                        self.focused_path.as_ref(),
                        self.widget_registry,
                        matches!(action, StandardAction::FocusNext),
                    );
                    let focus_changed = apply_focus_change(
                        self.root_widget,
                        self.focus_manager,
                        self.widget_registry,
                        self.focused_path,
                        next_focus,
                    );
                    if focus_changed {
                        *self.ime_composing = false;
                        *self.needs_repaint = true;
                        effects.push(PlatformEffect::SyncTextInput);
                    }
                    handled_focus_navigation = true;
                }
                StandardAction::Paste if self.focused_text_editor_state().is_some() => {
                    let Some(text) = clipboard_text else {
                        return effects;
                    };
                    dispatch_event = InputEvent::Paste { text };
                }
                _ => {}
            }
        }

        if handled_focus_navigation {
            return effects;
        }

        let current_focus_id = self
            .focused_path
            .as_ref()
            .and_then(|path| self.widget_registry.id_for_path(path));
        let current_capture_path = self.capture_path.clone();
        let outcome = self.signal_runtime.run_with_current(|| {
            with_shortcut_profile(self.shortcut_profile, || {
                dispatch_widget_event(
                    self.root_widget,
                    self.layout_tree,
                    current_focus_id,
                    current_capture_path.as_ref(),
                    &dispatch_event,
                )
            })
        });

        if outcome.commands.request_focus || outcome.commands.clear_focus {
            let focus_changed = apply_focus_change(
                self.root_widget,
                self.focus_manager,
                self.widget_registry,
                self.focused_path,
                outcome.focus_path.clone(),
            );
            if focus_changed {
                if self.focused_text_editor_state().is_none() {
                    *self.ime_composing = false;
                }
                *self.needs_repaint = true;
                effects.push(PlatformEffect::SyncTextInput);
            }
        }

        if outcome.commands.capture_pointer || outcome.commands.release_pointer {
            *self.capture_path = outcome.capture_path;
            *self.needs_repaint = true;
            effects.push(PlatformEffect::SyncPointerCapture);
        }

        if let Some(text) = outcome.commands.clipboard_write {
            effects.push(PlatformEffect::WriteClipboard(text));
        }

        if outcome.commands.request_paint {
            *self.needs_repaint = true;
        }
        if outcome.commands.request_layout {
            *self.needs_layout = true;
        }

        run_signal_effects(&self.signal_runtime, self.needs_layout, self.needs_repaint);

        effects
    }

    pub(crate) fn handle_input_event_update(
        &mut self,
        event: InputEvent,
        clipboard_text: Option<String>,
    ) -> RuntimePlatformUpdate {
        let effects = self.handle_input_event(event, clipboard_text);
        self.platform_update(effects)
    }

    pub(crate) fn handle_accessibility_action(
        &mut self,
        node_id: u64,
        action: AccessibilityAction,
        value: Option<String>,
    ) -> PlatformEffects {
        let Some(path) = self
            .widget_registry
            .path_for_accessibility_node(node_id)
            .map(ToOwned::to_owned)
        else {
            return PlatformEffects::default();
        };

        let mut effects = PlatformEffects::default();
        match action {
            AccessibilityAction::Focus => {
                let focus_changed = apply_focus_change(
                    self.root_widget,
                    self.focus_manager,
                    self.widget_registry,
                    self.focused_path,
                    Some(path),
                );
                if focus_changed {
                    *self.ime_composing = false;
                    *self.needs_repaint = true;
                    effects.push(PlatformEffect::SyncTextInput);
                }
            }
            action => {
                let handled = with_widget_mut(self.root_widget, &path, |widget| {
                    widget.handle_accessibility_action(action, value.clone())
                })
                .unwrap_or(false);
                if handled {
                    if matches!(action, AccessibilityAction::SetValue) {
                        *self.needs_layout = true;
                    }
                    *self.needs_repaint = true;
                }
            }
        }

        run_signal_effects(&self.signal_runtime, self.needs_layout, self.needs_repaint);

        effects
    }

    pub(crate) fn handle_accessibility_action_update(
        &mut self,
        node_id: u64,
        action: AccessibilityAction,
        value: Option<String>,
    ) -> RuntimePlatformUpdate {
        let effects = self.handle_accessibility_action(node_id, action, value);
        self.platform_update(effects)
    }
}

pub(crate) fn focused_text_editor_state<'a>(
    widget_registry: &'a WidgetRuntimeRegistry,
    focused_path: Option<&[usize]>,
) -> Option<&'a TextEditorState> {
    focused_path.and_then(|path| widget_registry.text_editor_state_for_path(path))
}

pub(crate) fn refresh_accessibility_tree(
    widget_registry: &mut WidgetRuntimeRegistry,
    root_widget: &dyn Widget,
    layout_tree: &LayoutTree,
    focused_path: Option<&WidgetPath>,
) -> AccessibilityTreeSnapshot {
    widget_registry.accessibility =
        collect_accessibility_tree(root_widget, layout_tree, focused_path);
    widget_registry.accessibility_tree().clone()
}

fn run_signal_effects(
    signal_runtime: &RuntimeHandle,
    needs_layout: &mut bool,
    needs_repaint: &mut bool,
) {
    signal_runtime.run_effects(64);
    let dirty = signal_runtime.take_dirty_flags();
    if dirty.rebuild || dirty.layout {
        *needs_layout = true;
    }
    if dirty.paint {
        *needs_repaint = true;
    }
}
