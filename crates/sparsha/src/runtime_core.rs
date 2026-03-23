use crate::accessibility::AccessibilityTreeSnapshot;
use crate::app::AppTheme;
use crate::component::ComponentStateStore;
use crate::platform::{PlatformEffect, PlatformEffects};
use crate::runtime_widget::{
    add_widget_to_layout, apply_focus_change, collect_accessibility_tree, dispatch_widget_event,
    move_focus_path, remap_path, sync_focus_manager, with_widget_mut, WidgetPath,
    WidgetRuntimeRegistry,
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

pub(crate) fn build_layout(ctx: RuntimeCoreContext<'_>) -> PlatformEffects {
    let runtime = ctx.signal_runtime.clone();
    *ctx.layout_tree = LayoutTree::new();
    ctx.component_states.begin_rebuild();

    runtime.with_tracking(SubscriberKind::Rebuild, || {
        let resolved_theme = ctx.theme.resolve_theme();
        let navigator = ctx.navigator.clone();
        let viewport = ctx.viewport;
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
        build_ctx.insert_resource(ctx.task_runtime.clone());
        build_ctx.insert_resource(ctx.signal_runtime.clone());
        build_ctx.insert_resource(viewport);
        unsafe { build_ctx.set_state_store(ctx.component_states) };
        let mut path = Vec::new();
        persist_widget_state(ctx.root_widget, &mut build_ctx, &mut path);
        ctx.component_states.begin_rebuild();
        path.clear();
        rebuild_widget(ctx.root_widget, &mut build_ctx, &mut path);
    });
    ctx.component_states.finish_rebuild();

    let mut widget_registry = WidgetRuntimeRegistry::default();
    let root_id = runtime.with_tracking(SubscriberKind::Layout, || {
        set_current_theme(ctx.theme.resolve_theme());
        set_current_viewport(ctx.viewport);
        let mut path = Vec::new();
        add_widget_to_layout(
            ctx.root_widget,
            ctx.layout_tree,
            ctx.text_system,
            &mut widget_registry,
            &mut path,
            false,
            true,
        )
    });
    ctx.layout_tree.set_root(root_id);
    *ctx.widget_registry = widget_registry;
    ctx.layout_tree
        .compute_layout(ctx.viewport.width.max(1.0), ctx.viewport.height.max(1.0));
    *ctx.focused_path = remap_path(ctx.focused_path.take(), ctx.widget_registry);
    *ctx.capture_path = remap_path(ctx.capture_path.take(), ctx.widget_registry);
    sync_focus_manager(
        ctx.focus_manager,
        ctx.widget_registry,
        ctx.focused_path.as_ref(),
    );
    *ctx.needs_layout = false;
    *ctx.needs_repaint = true;

    let mut effects = PlatformEffects::default();
    effects.push(PlatformEffect::SyncTextInput);
    effects.push(PlatformEffect::SyncPointerCapture);
    effects
}

pub(crate) fn handle_input_event(
    ctx: RuntimeCoreContext<'_>,
    event: InputEvent,
    clipboard_text: Option<String>,
) -> PlatformEffects {
    let mapper = ActionMapper::with_shortcut_profile(ctx.shortcut_profile);
    let mut handled_focus_navigation = false;
    let mut dispatch_event = event.clone();
    let mut effects = PlatformEffects::default();

    if let Some(Action::Standard(action)) = mapper.map_event(&event) {
        match action {
            StandardAction::FocusNext | StandardAction::FocusPrevious => {
                let next_focus = move_focus_path(
                    ctx.focused_path.as_ref(),
                    ctx.widget_registry,
                    matches!(action, StandardAction::FocusNext),
                );
                let focus_changed = apply_focus_change(
                    ctx.root_widget,
                    ctx.focus_manager,
                    ctx.widget_registry,
                    ctx.focused_path,
                    next_focus,
                );
                if focus_changed {
                    *ctx.ime_composing = false;
                    *ctx.needs_repaint = true;
                    effects.push(PlatformEffect::SyncTextInput);
                }
                handled_focus_navigation = true;
            }
            StandardAction::Paste
                if focused_text_editor_state(ctx.widget_registry, ctx.focused_path.as_deref())
                    .is_some() =>
            {
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

    let current_focus_id = ctx
        .focused_path
        .as_ref()
        .and_then(|path| ctx.widget_registry.id_for_path(path));
    let current_capture_path = ctx.capture_path.clone();
    let outcome = ctx.signal_runtime.run_with_current(|| {
        with_shortcut_profile(ctx.shortcut_profile, || {
            dispatch_widget_event(
                ctx.root_widget,
                ctx.layout_tree,
                current_focus_id,
                current_capture_path.as_ref(),
                &dispatch_event,
            )
        })
    });

    if outcome.commands.request_focus || outcome.commands.clear_focus {
        let focus_changed = apply_focus_change(
            ctx.root_widget,
            ctx.focus_manager,
            ctx.widget_registry,
            ctx.focused_path,
            outcome.focus_path.clone(),
        );
        if focus_changed {
            if focused_text_editor_state(ctx.widget_registry, ctx.focused_path.as_deref()).is_none()
            {
                *ctx.ime_composing = false;
            }
            *ctx.needs_repaint = true;
            effects.push(PlatformEffect::SyncTextInput);
        }
    }

    if outcome.commands.capture_pointer || outcome.commands.release_pointer {
        *ctx.capture_path = outcome.capture_path;
        *ctx.needs_repaint = true;
        effects.push(PlatformEffect::SyncPointerCapture);
    }

    if let Some(text) = outcome.commands.clipboard_write {
        effects.push(PlatformEffect::WriteClipboard(text));
    }

    if outcome.commands.request_paint {
        *ctx.needs_repaint = true;
    }
    if outcome.commands.request_layout {
        *ctx.needs_layout = true;
    }

    run_signal_effects(&ctx.signal_runtime, ctx.needs_layout, ctx.needs_repaint);

    effects
}

pub(crate) fn handle_accessibility_action(
    ctx: RuntimeCoreContext<'_>,
    node_id: u64,
    action: AccessibilityAction,
    value: Option<String>,
) -> PlatformEffects {
    let Some(path) = ctx
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
                ctx.root_widget,
                ctx.focus_manager,
                ctx.widget_registry,
                ctx.focused_path,
                Some(path),
            );
            if focus_changed {
                *ctx.ime_composing = false;
                *ctx.needs_repaint = true;
                effects.push(PlatformEffect::SyncTextInput);
            }
        }
        action => {
            let handled = with_widget_mut(ctx.root_widget, &path, |widget| {
                widget.handle_accessibility_action(action, value.clone())
            })
            .unwrap_or(false);
            if handled {
                if matches!(action, AccessibilityAction::SetValue) {
                    *ctx.needs_layout = true;
                }
                *ctx.needs_repaint = true;
            }
        }
    }

    run_signal_effects(&ctx.signal_runtime, ctx.needs_layout, ctx.needs_repaint);

    effects
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
