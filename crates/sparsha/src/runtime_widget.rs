use crate::accessibility::{
    accessibility_node_id, AccessibilityNodeSnapshot, AccessibilityTreeSnapshot,
    ACCESSIBILITY_ROOT_ID,
};
use sparsha_input::{FocusManager, InputEvent};
use sparsha_layout::{taffy::Dimension, LayoutTree, WidgetId};
use sparsha_text::TextSystem;
use sparsha_widgets::{
    AccessibilityInfo, AccessibilityRole, EventCommands, EventContext, LayoutContext,
    TextEditorState, Widget, WidgetChildMode,
};
use std::collections::{HashMap, HashSet};

pub(crate) type WidgetPath = Vec<usize>;

#[derive(Clone, Debug, Default)]
pub(crate) struct WidgetRuntimeRegistry {
    active_paths: HashSet<WidgetPath>,
    focus_order: Vec<WidgetPath>,
    path_to_id: HashMap<WidgetPath, WidgetId>,
    id_to_path: HashMap<WidgetId, WidgetPath>,
    text_editors: HashMap<WidgetPath, TextEditorState>,
    pub(crate) accessibility: AccessibilityTreeSnapshot,
}

impl WidgetRuntimeRegistry {
    pub(crate) fn focus_order(&self) -> &[WidgetPath] {
        &self.focus_order
    }

    pub(crate) fn id_for_path(&self, path: &[usize]) -> Option<WidgetId> {
        self.path_to_id.get(path).copied()
    }

    pub(crate) fn text_editor_state_for_path(&self, path: &[usize]) -> Option<&TextEditorState> {
        self.text_editors.get(path)
    }

    pub(crate) fn accessibility_tree(&self) -> &AccessibilityTreeSnapshot {
        &self.accessibility
    }

    pub(crate) fn path_for_accessibility_node(&self, node_id: u64) -> Option<&[usize]> {
        self.accessibility.path_for_node(node_id)
    }

    pub(crate) fn is_active_path(&self, path: &[usize]) -> bool {
        self.active_paths.contains(path)
    }
}

#[derive(Clone, Debug, Default)]
pub(crate) struct DispatchOutcome {
    pub commands: EventCommands,
    pub focus_path: Option<WidgetPath>,
    pub capture_path: Option<WidgetPath>,
}

pub(crate) fn add_widget_to_layout(
    widget: &mut dyn Widget,
    tree: &mut LayoutTree,
    text_system: &mut TextSystem,
    registry: &mut WidgetRuntimeRegistry,
    path: &mut WidgetPath,
    in_scroll: bool,
    runtime_active: bool,
) -> WidgetId {
    let mut style = widget.style();
    if in_scroll {
        style.flex_shrink = 0.0;
    }

    let is_scroll = widget.is_scroll_container();
    let child_keys: Vec<_> = (0..widget.children().len())
        .map(|index| widget.child_path_key(index))
        .collect();
    let child_modes: Vec<_> = (0..widget.children().len())
        .map(|index| widget.child_mode(index))
        .collect();
    let children_ids: Vec<_> = widget
        .children_mut()
        .iter_mut()
        .enumerate()
        .map(|(index, child)| {
            path.push(child_keys[index]);
            let child_runtime_active =
                runtime_active && child_modes[index] == WidgetChildMode::Active;
            let id = add_widget_to_layout(
                child.as_mut(),
                tree,
                text_system,
                registry,
                path,
                in_scroll || is_scroll,
                child_runtime_active,
            );
            path.pop();
            id
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
    let widget_path = path.clone();
    if runtime_active {
        registry.active_paths.insert(widget_path.clone());
    }
    registry.path_to_id.insert(widget_path.clone(), id);
    registry.id_to_path.insert(id, widget_path.clone());
    if runtime_active && widget.focusable() {
        registry.focus_order.push(widget_path.clone());
    }
    if runtime_active {
        if let Some(editor_state) = widget.text_editor_state() {
            registry.text_editors.insert(widget_path, editor_state);
        }
    }
    id
}

pub(crate) fn sync_focus_manager(
    focus_manager: &mut FocusManager,
    registry: &WidgetRuntimeRegistry,
    focused_path: Option<&WidgetPath>,
) {
    focus_manager.clear_focusable();
    for path in registry.focus_order() {
        if let Some(id) = registry.id_for_path(path) {
            focus_manager.register_focusable(id);
        }
    }

    if let Some(path) =
        focused_path.filter(|path| registry.focus_order.iter().any(|entry| entry == *path))
    {
        if let Some(id) = registry.id_for_path(path) {
            focus_manager.set_focus(id);
            return;
        }
    }
    focus_manager.clear_focus();
}

pub(crate) fn remap_path(
    path: Option<WidgetPath>,
    registry: &WidgetRuntimeRegistry,
) -> Option<WidgetPath> {
    path.filter(|candidate| registry.is_active_path(candidate))
}

pub(crate) fn collect_accessibility_tree(
    root_widget: &dyn Widget,
    layout_tree: &LayoutTree,
    focused_path: Option<&WidgetPath>,
) -> AccessibilityTreeSnapshot {
    #[derive(Default)]
    struct SubtreeResult {
        root_nodes: Vec<u64>,
        first_descendant: Option<u64>,
        focused_node: Option<u64>,
    }

    struct AccessibilityCollector<'a> {
        layout_tree: &'a LayoutTree,
        focused_path: Option<&'a WidgetPath>,
        nodes: Vec<AccessibilityNodeSnapshot>,
        node_paths: HashMap<u64, WidgetPath>,
        node_indices: HashMap<u64, usize>,
    }

    impl<'a> AccessibilityCollector<'a> {
        fn build_snapshot(
            &self,
            widget: &dyn Widget,
            path: &[usize],
            info: AccessibilityInfo,
            children: Vec<u64>,
        ) -> AccessibilityNodeSnapshot {
            let bounds = self
                .layout_tree
                .get_absolute_layout(widget.id())
                .map(|layout| layout.bounds)
                .unwrap_or(sparsha_core::Rect::ZERO);
            AccessibilityNodeSnapshot {
                id: accessibility_node_id(path),
                path: path.to_vec(),
                role: info.role.unwrap_or(AccessibilityRole::GenericContainer),
                label: info.label,
                description: info.description,
                value: info.value,
                hidden: info.hidden,
                disabled: info.disabled,
                checked: info.checked,
                actions: info.actions,
                bounds,
                children,
            }
        }

        fn visit(
            &mut self,
            widget: &dyn Widget,
            runtime_active: bool,
            path: &mut WidgetPath,
        ) -> SubtreeResult {
            if !runtime_active {
                return SubtreeResult::default();
            }

            let mut root_nodes = Vec::new();
            let mut first_descendant = None;
            let mut focused_node = None;

            for (index, child) in widget.children().iter().enumerate() {
                path.push(widget.child_path_key(index));
                let child_result = self.visit(
                    child.as_ref(),
                    widget.child_mode(index) == WidgetChildMode::Active,
                    path,
                );
                path.pop();
                root_nodes.extend(child_result.root_nodes);
                if first_descendant.is_none() {
                    first_descendant = child_result.first_descendant;
                }
                if focused_node.is_none() {
                    focused_node = child_result.focused_node;
                }
            }

            let Some(info) = widget.accessibility_info() else {
                return SubtreeResult {
                    root_nodes,
                    first_descendant,
                    focused_node,
                };
            };

            if widget.accessibility_merge_descendant() {
                if let Some(target_id) = first_descendant {
                    if let Some(index) = self.node_indices.get(&target_id).copied() {
                        self.nodes[index].apply_overrides(info);
                    }
                    return SubtreeResult {
                        root_nodes,
                        first_descendant: Some(target_id),
                        focused_node,
                    };
                }

                if !info.has_metadata() {
                    return SubtreeResult {
                        root_nodes,
                        first_descendant,
                        focused_node,
                    };
                }
            }

            let snapshot = self.build_snapshot(widget, path, info, root_nodes.clone());
            let node_id = snapshot.id;
            self.node_paths.insert(node_id, path.clone());
            self.node_indices.insert(node_id, self.nodes.len());
            self.nodes.push(snapshot);

            let focused_node = if self.focused_path == Some(path) {
                Some(node_id)
            } else {
                focused_node
            };

            SubtreeResult {
                root_nodes: vec![node_id],
                first_descendant: Some(node_id),
                focused_node,
            }
        }
    }

    let mut collector = AccessibilityCollector {
        layout_tree,
        focused_path,
        nodes: Vec::new(),
        node_paths: HashMap::new(),
        node_indices: HashMap::new(),
    };
    let result = collector.visit(root_widget, true, &mut Vec::new());
    AccessibilityTreeSnapshot {
        nodes: collector.nodes,
        root_children: result.root_nodes,
        focus: result.focused_node.unwrap_or(ACCESSIBILITY_ROOT_ID),
        node_paths: collector.node_paths,
    }
}

pub(crate) fn move_focus_path(
    current: Option<&WidgetPath>,
    registry: &WidgetRuntimeRegistry,
    forward: bool,
) -> Option<WidgetPath> {
    if registry.focus_order.is_empty() {
        return None;
    }

    let next_index = match current
        .and_then(|path| registry.focus_order.iter().position(|entry| entry == path))
    {
        Some(index) if forward => (index + 1) % registry.focus_order.len(),
        Some(index) => {
            if index == 0 {
                registry.focus_order.len() - 1
            } else {
                index - 1
            }
        }
        None if forward => 0,
        None => registry.focus_order.len() - 1,
    };
    registry.focus_order.get(next_index).cloned()
}

pub(crate) fn apply_focus_change(
    root_widget: &mut dyn Widget,
    focus_manager: &mut FocusManager,
    registry: &WidgetRuntimeRegistry,
    focused_path: &mut Option<WidgetPath>,
    next_focus_path: Option<WidgetPath>,
) -> bool {
    if focused_path.as_ref() == next_focus_path.as_ref() {
        sync_focus_manager(focus_manager, registry, focused_path.as_ref());
        return false;
    }

    let previous_focus = focused_path.clone();
    if let Some(path) = previous_focus.as_ref() {
        let _ = with_widget_mut(root_widget, path, |widget| widget.on_blur());
    }

    *focused_path = next_focus_path;
    sync_focus_manager(focus_manager, registry, focused_path.as_ref());

    if let Some(path) = focused_path.as_ref() {
        let _ = with_widget_mut(root_widget, path, |widget| widget.on_focus());
    }

    true
}

pub(crate) fn dispatch_widget_event(
    root_widget: &mut dyn Widget,
    layout_tree: &LayoutTree,
    focused_id: Option<WidgetId>,
    capture_path: Option<&WidgetPath>,
    event: &InputEvent,
) -> DispatchOutcome {
    let mut outcome = DispatchOutcome {
        commands: EventCommands::default(),
        focus_path: None,
        capture_path: capture_path.cloned(),
    };

    match event {
        InputEvent::PointerMove { .. } | InputEvent::PointerUp { .. } => {
            if let Some(path) = capture_path {
                let _ = dispatch_widget_event_at_path(
                    root_widget,
                    layout_tree,
                    focused_id,
                    Some(path),
                    event,
                    path,
                    &mut Vec::new(),
                    &mut outcome,
                );
                return outcome;
            }
        }
        _ => {}
    }

    dispatch_widget_event_recursive(
        root_widget,
        layout_tree,
        focused_id,
        capture_path,
        event,
        &mut Vec::new(),
        &mut outcome,
    );
    outcome
}

fn transform_event_for_children(event: &InputEvent, offset: glam::Vec2) -> InputEvent {
    match event {
        InputEvent::PointerMove { pos } => InputEvent::PointerMove { pos: *pos + offset },
        InputEvent::PointerDown { pos, button } => InputEvent::PointerDown {
            pos: *pos + offset,
            button: *button,
        },
        InputEvent::PointerUp { pos, button } => InputEvent::PointerUp {
            pos: *pos + offset,
            button: *button,
        },
        InputEvent::Scroll {
            pos,
            delta,
            modifiers,
        } => InputEvent::Scroll {
            pos: *pos + offset,
            delta: *delta,
            modifiers: *modifiers,
        },
        _ => event.clone(),
    }
}

fn scroll_container_prehandles_event(event: &InputEvent) -> bool {
    matches!(
        event,
        InputEvent::PointerMove { .. }
            | InputEvent::PointerDown { .. }
            | InputEvent::PointerUp { .. }
    )
}

fn dispatch_widget_event_recursive(
    widget: &mut dyn Widget,
    layout_tree: &LayoutTree,
    focused_id: Option<WidgetId>,
    capture_path: Option<&WidgetPath>,
    event: &InputEvent,
    path: &mut WidgetPath,
    outcome: &mut DispatchOutcome,
) {
    let id = widget.id();
    let Some(layout) = layout_tree.get_absolute_layout(id) else {
        return;
    };

    let should_descend = if widget.is_scroll_container() && event.is_pointer_event() {
        event.pos().is_some_and(|pos| layout.bounds.contains(pos))
    } else {
        true
    };

    let child_offset = widget.child_event_offset();
    let transformed_event =
        if should_descend && child_offset != glam::Vec2::ZERO && event.is_pointer_event() {
            Some(transform_event_for_children(event, child_offset))
        } else {
            None
        };
    let child_event = transformed_event.as_ref().unwrap_or(event);
    let child_keys: Vec<_> = (0..widget.children().len())
        .map(|index| widget.child_path_key(index))
        .collect();
    let child_modes: Vec<_> = (0..widget.children().len())
        .map(|index| widget.child_mode(index))
        .collect();

    if widget.is_scroll_container() && scroll_container_prehandles_event(event) {
        dispatch_widget_event_here(
            widget,
            layout_tree,
            focused_id,
            capture_path,
            event,
            path,
            outcome,
        );
        if outcome.commands.stop_propagation {
            return;
        }
    }

    if should_descend {
        for (index, child) in widget.children_mut().iter_mut().enumerate() {
            if child_modes[index] != WidgetChildMode::Active {
                continue;
            }
            path.push(child_keys[index]);
            dispatch_widget_event_recursive(
                child.as_mut(),
                layout_tree,
                focused_id,
                capture_path,
                child_event,
                path,
                outcome,
            );
            path.pop();
            if outcome.commands.stop_propagation {
                return;
            }
        }
    }

    if !(widget.is_scroll_container() && scroll_container_prehandles_event(event)) {
        dispatch_widget_event_here(
            widget,
            layout_tree,
            focused_id,
            capture_path,
            event,
            path,
            outcome,
        );
    }
}

#[allow(clippy::too_many_arguments)]
fn dispatch_widget_event_at_path(
    widget: &mut dyn Widget,
    layout_tree: &LayoutTree,
    focused_id: Option<WidgetId>,
    capture_path: Option<&WidgetPath>,
    event: &InputEvent,
    target_path: &[usize],
    current_path: &mut WidgetPath,
    outcome: &mut DispatchOutcome,
) -> bool {
    if target_path.is_empty() {
        dispatch_widget_event_here(
            widget,
            layout_tree,
            focused_id,
            capture_path,
            event,
            current_path,
            outcome,
        );
        return true;
    }

    let child_offset = widget.child_event_offset();
    let transformed_event = if child_offset != glam::Vec2::ZERO && event.is_pointer_event() {
        Some(transform_event_for_children(event, child_offset))
    } else {
        None
    };
    let child_event = transformed_event.as_ref().unwrap_or(event);

    let Some(child_slot) = widget.child_slot_for_path_key(target_path[0]) else {
        return false;
    };

    if widget.child_mode(child_slot) != WidgetChildMode::Active {
        return false;
    }

    let Some(child) = widget.children_mut().get_mut(child_slot) else {
        return false;
    };

    current_path.push(target_path[0]);
    let found = dispatch_widget_event_at_path(
        child.as_mut(),
        layout_tree,
        focused_id,
        capture_path,
        child_event,
        &target_path[1..],
        current_path,
        outcome,
    );
    current_path.pop();
    if found {
        return true;
    }
    false
}

fn dispatch_widget_event_here(
    widget: &mut dyn Widget,
    layout_tree: &LayoutTree,
    focused_id: Option<WidgetId>,
    capture_path: Option<&WidgetPath>,
    event: &InputEvent,
    path: &mut WidgetPath,
    outcome: &mut DispatchOutcome,
) {
    let id = widget.id();
    let Some(layout) = layout_tree.get_absolute_layout(id) else {
        return;
    };

    let mut temp_focus = FocusManager::new();
    if let Some(id) = focused_id {
        temp_focus.set_focus(id);
    }

    let mut ctx = EventContext {
        layout,
        layout_tree,
        focus: &mut temp_focus,
        widget_id: id,
        has_capture: capture_path == Some(path),
        commands: EventCommands::default(),
    };
    widget.event(&mut ctx, event);

    if ctx.commands.request_focus {
        outcome.focus_path = Some(path.clone());
    } else if ctx.commands.clear_focus && focused_id == Some(id) {
        outcome.focus_path = None;
    }

    if ctx.commands.capture_pointer {
        outcome.capture_path = Some(path.clone());
    } else if ctx.commands.release_pointer && capture_path == Some(path) {
        outcome.capture_path = None;
    }

    outcome.commands.merge(ctx.commands);
}

pub(crate) fn with_widget_mut<R>(
    widget: &mut dyn Widget,
    path: &[usize],
    f: impl FnOnce(&mut dyn Widget) -> R,
) -> Option<R> {
    with_widget_mut_inner(widget, path, Some(f))
}

fn with_widget_mut_inner<R>(
    widget: &mut dyn Widget,
    path: &[usize],
    mut f: Option<impl FnOnce(&mut dyn Widget) -> R>,
) -> Option<R> {
    if path.is_empty() {
        return f.take().map(|apply| apply(widget));
    }

    let child_slot = widget.child_slot_for_path_key(path[0])?;
    let child = widget.children_mut().get_mut(child_slot)?;
    with_widget_mut_inner(child.as_mut(), &path[1..], f)
}

#[cfg(test)]
mod tests {
    use super::*;
    use sparsha_input::{InputEvent, PointerButton};
    use sparsha_layout::taffy::{self, prelude::*};
    use sparsha_render::DrawList;
    use sparsha_widgets::{
        Button, Container, PaintCommands, PaintContext, Semantics, TextInput, WidgetChildMode,
    };
    use std::sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    };

    #[derive(Default)]
    struct FocusProbe {
        id: WidgetId,
        focusable: bool,
        focus_count: Arc<AtomicUsize>,
        blur_count: Arc<AtomicUsize>,
        capture_on_down: bool,
        seen_capture_moves: Arc<AtomicUsize>,
    }

    impl FocusProbe {
        fn new(focusable: bool) -> (Self, Arc<AtomicUsize>, Arc<AtomicUsize>) {
            let focus_count = Arc::new(AtomicUsize::new(0));
            let blur_count = Arc::new(AtomicUsize::new(0));
            (
                Self {
                    focusable,
                    focus_count: Arc::clone(&focus_count),
                    blur_count: Arc::clone(&blur_count),
                    ..Default::default()
                },
                focus_count,
                blur_count,
            )
        }

        fn capture_probe() -> (Self, Arc<AtomicUsize>) {
            let seen_capture_moves = Arc::new(AtomicUsize::new(0));
            (
                Self {
                    focusable: true,
                    capture_on_down: true,
                    seen_capture_moves: Arc::clone(&seen_capture_moves),
                    ..Default::default()
                },
                seen_capture_moves,
            )
        }
    }

    struct OffsetScrollProbe {
        id: WidgetId,
        offset: glam::Vec2,
        child: Box<dyn Widget>,
    }

    impl OffsetScrollProbe {
        fn new(offset: glam::Vec2, child: impl Widget + 'static) -> Self {
            Self {
                id: WidgetId::default(),
                offset,
                child: Box::new(child),
            }
        }
    }

    impl Widget for OffsetScrollProbe {
        fn id(&self) -> WidgetId {
            self.id
        }

        fn set_id(&mut self, id: WidgetId) {
            self.id = id;
        }

        fn style(&self) -> taffy::Style {
            taffy::Style {
                size: Size {
                    width: length(200.0),
                    height: length(100.0),
                },
                display: Display::Flex,
                flex_direction: FlexDirection::Column,
                ..Default::default()
            }
        }

        fn paint(&self, _ctx: &mut PaintContext) {}

        fn is_scroll_container(&self) -> bool {
            true
        }

        fn child_event_offset(&self) -> glam::Vec2 {
            self.offset
        }

        fn children(&self) -> &[Box<dyn Widget>] {
            std::slice::from_ref(&self.child)
        }

        fn children_mut(&mut self) -> &mut [Box<dyn Widget>] {
            std::slice::from_mut(&mut self.child)
        }
    }

    struct ScrollHitProbe {
        id: WidgetId,
        seen_scroll: Arc<AtomicUsize>,
        size: (f32, f32),
    }

    impl ScrollHitProbe {
        fn new(size: (f32, f32)) -> (Self, Arc<AtomicUsize>) {
            let seen_scroll = Arc::new(AtomicUsize::new(0));
            (
                Self {
                    id: WidgetId::default(),
                    seen_scroll: Arc::clone(&seen_scroll),
                    size,
                },
                seen_scroll,
            )
        }
    }

    impl Widget for ScrollHitProbe {
        fn id(&self) -> WidgetId {
            self.id
        }

        fn set_id(&mut self, id: WidgetId) {
            self.id = id;
        }

        fn style(&self) -> taffy::Style {
            taffy::Style {
                size: Size {
                    width: length(self.size.0),
                    height: length(self.size.1),
                },
                ..Default::default()
            }
        }

        fn paint(&self, _ctx: &mut PaintContext) {}

        fn event(&mut self, ctx: &mut EventContext, event: &InputEvent) {
            if let InputEvent::Scroll { pos, .. } = event {
                if ctx.contains(*pos) {
                    self.seen_scroll.fetch_add(1, Ordering::SeqCst);
                    ctx.stop_propagation();
                }
            }
        }
    }

    struct ScrollContainerProbe {
        id: WidgetId,
        seen_scroll: Arc<AtomicUsize>,
        size: (f32, f32),
        children: Vec<Box<dyn Widget>>,
    }

    impl ScrollContainerProbe {
        fn new(size: (f32, f32)) -> (Self, Arc<AtomicUsize>) {
            let seen_scroll = Arc::new(AtomicUsize::new(0));
            (
                Self {
                    id: WidgetId::default(),
                    seen_scroll: Arc::clone(&seen_scroll),
                    size,
                    children: Vec::new(),
                },
                seen_scroll,
            )
        }

        fn child(mut self, child: impl Widget + 'static) -> Self {
            self.children.push(Box::new(child));
            self
        }
    }

    struct ChildModeWrapper {
        id: WidgetId,
        mode: WidgetChildMode,
        child: Box<dyn Widget>,
    }

    impl ChildModeWrapper {
        fn new(mode: WidgetChildMode, child: impl Widget + 'static) -> Self {
            Self {
                id: WidgetId::default(),
                mode,
                child: Box::new(child),
            }
        }
    }

    impl Widget for ChildModeWrapper {
        fn id(&self) -> WidgetId {
            self.id
        }

        fn set_id(&mut self, id: WidgetId) {
            self.id = id;
        }

        fn style(&self) -> taffy::Style {
            taffy::Style {
                size: Size {
                    width: length(200.0),
                    height: length(60.0),
                },
                display: Display::Flex,
                flex_direction: FlexDirection::Column,
                ..Default::default()
            }
        }

        fn paint(&self, _ctx: &mut PaintContext) {}

        fn children(&self) -> &[Box<dyn Widget>] {
            std::slice::from_ref(&self.child)
        }

        fn children_mut(&mut self) -> &mut [Box<dyn Widget>] {
            std::slice::from_mut(&mut self.child)
        }

        fn child_mode(&self, _child_position: usize) -> WidgetChildMode {
            self.mode
        }
    }

    struct EventProbe {
        id: WidgetId,
        hits: Arc<AtomicUsize>,
    }

    impl EventProbe {
        fn new() -> (Self, Arc<AtomicUsize>) {
            let hits = Arc::new(AtomicUsize::new(0));
            (
                Self {
                    id: WidgetId::default(),
                    hits: Arc::clone(&hits),
                },
                hits,
            )
        }
    }

    impl Widget for EventProbe {
        fn id(&self) -> WidgetId {
            self.id
        }

        fn set_id(&mut self, id: WidgetId) {
            self.id = id;
        }

        fn style(&self) -> taffy::Style {
            taffy::Style {
                size: Size {
                    width: length(120.0),
                    height: length(40.0),
                },
                ..Default::default()
            }
        }

        fn paint(&self, _ctx: &mut PaintContext) {}

        fn event(&mut self, ctx: &mut EventContext, event: &InputEvent) {
            if let InputEvent::PointerDown { pos, .. } = event {
                if ctx.contains(*pos) {
                    self.hits.fetch_add(1, Ordering::SeqCst);
                    ctx.stop_propagation();
                }
            }
        }
    }

    struct PaintProbe {
        id: WidgetId,
        paints: Arc<AtomicUsize>,
    }

    impl PaintProbe {
        fn new() -> (Self, Arc<AtomicUsize>) {
            let paints = Arc::new(AtomicUsize::new(0));
            (
                Self {
                    id: WidgetId::default(),
                    paints: Arc::clone(&paints),
                },
                paints,
            )
        }
    }

    impl Widget for PaintProbe {
        fn id(&self) -> WidgetId {
            self.id
        }

        fn set_id(&mut self, id: WidgetId) {
            self.id = id;
        }

        fn style(&self) -> taffy::Style {
            taffy::Style {
                size: Size {
                    width: length(120.0),
                    height: length(40.0),
                },
                ..Default::default()
            }
        }

        fn paint(&self, ctx: &mut PaintContext) {
            self.paints.fetch_add(1, Ordering::SeqCst);
            ctx.fill_rect(ctx.bounds(), sparsha_core::Color::from_hex(0x3366FF));
        }
    }

    impl Widget for ScrollContainerProbe {
        fn id(&self) -> WidgetId {
            self.id
        }

        fn set_id(&mut self, id: WidgetId) {
            self.id = id;
        }

        fn style(&self) -> taffy::Style {
            taffy::Style {
                size: Size {
                    width: length(self.size.0),
                    height: length(self.size.1),
                },
                display: Display::Flex,
                flex_direction: FlexDirection::Column,
                ..Default::default()
            }
        }

        fn paint(&self, _ctx: &mut PaintContext) {}

        fn is_scroll_container(&self) -> bool {
            true
        }

        fn event(&mut self, ctx: &mut EventContext, event: &InputEvent) {
            if let InputEvent::Scroll { pos, .. } = event {
                if ctx.contains(*pos) {
                    self.seen_scroll.fetch_add(1, Ordering::SeqCst);
                    ctx.stop_propagation();
                }
            }
        }

        fn children(&self) -> &[Box<dyn Widget>] {
            &self.children
        }

        fn children_mut(&mut self) -> &mut [Box<dyn Widget>] {
            &mut self.children
        }
    }

    impl Widget for FocusProbe {
        fn id(&self) -> WidgetId {
            self.id
        }

        fn set_id(&mut self, id: WidgetId) {
            self.id = id;
        }

        fn style(&self) -> taffy::Style {
            taffy::Style {
                size: Size {
                    width: length(80.0),
                    height: length(24.0),
                },
                ..Default::default()
            }
        }

        fn paint(&self, _ctx: &mut PaintContext) {}

        fn focusable(&self) -> bool {
            self.focusable
        }

        fn on_focus(&mut self) {
            self.focus_count.fetch_add(1, Ordering::SeqCst);
        }

        fn on_blur(&mut self) {
            self.blur_count.fetch_add(1, Ordering::SeqCst);
        }

        fn event(&mut self, ctx: &mut EventContext, event: &InputEvent) {
            match event {
                InputEvent::PointerDown { .. } if self.capture_on_down => ctx.capture_pointer(),
                InputEvent::PointerMove { .. } if ctx.has_capture => {
                    self.seen_capture_moves.fetch_add(1, Ordering::SeqCst);
                    ctx.stop_propagation();
                }
                InputEvent::PointerUp { .. } if ctx.has_capture => ctx.release_pointer(),
                _ => {}
            }
        }
    }

    fn build_registry(root: &mut dyn Widget) -> (LayoutTree, WidgetRuntimeRegistry) {
        let mut tree = LayoutTree::new();
        let mut registry = WidgetRuntimeRegistry::default();
        let mut text = TextSystem::new_headless();
        let mut path = Vec::new();
        let root_id = add_widget_to_layout(
            root,
            &mut tree,
            &mut text,
            &mut registry,
            &mut path,
            false,
            true,
        );
        tree.set_root(root_id);
        tree.compute_layout(480.0, 320.0);
        (tree, registry)
    }

    fn paint_widget_subtree(root: &dyn Widget, layout_tree: &LayoutTree) -> DrawList {
        fn paint_recursive(
            widget: &dyn Widget,
            layout_tree: &LayoutTree,
            focus: &FocusManager,
            draw_list: &mut DrawList,
            text_system: &mut TextSystem,
        ) {
            let Some(layout) = layout_tree.get_absolute_layout(widget.id()) else {
                return;
            };

            let mut commands = PaintCommands::default();
            let mut ctx = PaintContext {
                draw_list,
                layout: sparsha_layout::ComputedLayout::new(layout.bounds),
                layout_tree,
                focus,
                widget_id: widget.id(),
                scale_factor: 1.0,
                text_system,
                elapsed_time: 0.0,
                commands: &mut commands,
            };
            widget.paint(&mut ctx);
            for child in widget.children() {
                paint_recursive(child.as_ref(), layout_tree, focus, draw_list, text_system);
            }
            let mut ctx = PaintContext {
                draw_list,
                layout: sparsha_layout::ComputedLayout::new(layout.bounds),
                layout_tree,
                focus,
                widget_id: widget.id(),
                scale_factor: 1.0,
                text_system,
                elapsed_time: 0.0,
                commands: &mut commands,
            };
            widget.paint_after_children(&mut ctx);
        }

        let mut draw_list = DrawList::new();
        let mut text_system = TextSystem::new_headless();
        let focus = FocusManager::new();
        paint_recursive(root, layout_tree, &focus, &mut draw_list, &mut text_system);
        draw_list
    }

    #[test]
    fn focus_order_follows_widget_paths() {
        let (button_a, _, _) = FocusProbe::new(true);
        let (button_b, _, _) = FocusProbe::new(true);
        let (label, _, _) = FocusProbe::new(false);
        let mut root = Container::column()
            .child(button_a)
            .child(label)
            .child(button_b);

        let (_, registry) = build_registry(&mut root);
        assert_eq!(registry.focus_order(), &[vec![0], vec![2]]);
    }

    #[test]
    fn paint_only_children_still_layout_and_paint() {
        let (probe, paints) = PaintProbe::new();
        let mut root = ChildModeWrapper::new(WidgetChildMode::PaintOnly, probe);

        let (layout_tree, registry) = build_registry(&mut root);
        assert!(registry.id_for_path(&[0]).is_some());

        let draw_list = paint_widget_subtree(&root, &layout_tree);
        assert_eq!(paints.load(Ordering::SeqCst), 1);
        assert!(!draw_list.is_empty());
    }

    #[test]
    fn paint_only_children_do_not_receive_events() {
        let (probe, hits) = EventProbe::new();
        let mut root = ChildModeWrapper::new(WidgetChildMode::PaintOnly, probe);
        let (layout_tree, _) = build_registry(&mut root);

        let _ = dispatch_widget_event(
            &mut root,
            &layout_tree,
            None,
            None,
            &InputEvent::PointerDown {
                pos: glam::vec2(10.0, 10.0),
                button: PointerButton::Primary,
            },
        );

        assert_eq!(hits.load(Ordering::SeqCst), 0);
    }

    #[test]
    fn paint_only_children_are_excluded_from_runtime_metadata() {
        let mut root = ChildModeWrapper::new(
            WidgetChildMode::PaintOnly,
            TextInput::builder().placeholder("Email").build(),
        );
        let (layout_tree, registry) = build_registry(&mut root);

        assert!(registry.focus_order().is_empty());
        assert!(registry.text_editor_state_for_path(&[0]).is_none());
        assert!(remap_path(Some(vec![0]), &registry).is_none());

        let tree = collect_accessibility_tree(&root, &layout_tree, None);
        assert!(tree.nodes.is_empty());
        assert!(tree.root_children.is_empty());
    }

    #[test]
    fn focus_callbacks_only_fire_on_logical_change() {
        let (first, first_focuses, first_blurs) = FocusProbe::new(true);
        let (second, second_focuses, second_blurs) = FocusProbe::new(true);
        let mut root = Container::column().child(first).child(second);
        let (_, registry) = build_registry(&mut root);
        let mut focus_manager = FocusManager::new();
        let mut focused_path = None;

        assert!(apply_focus_change(
            &mut root,
            &mut focus_manager,
            &registry,
            &mut focused_path,
            Some(vec![0]),
        ));
        assert!(!apply_focus_change(
            &mut root,
            &mut focus_manager,
            &registry,
            &mut focused_path,
            Some(vec![0]),
        ));
        assert!(apply_focus_change(
            &mut root,
            &mut focus_manager,
            &registry,
            &mut focused_path,
            Some(vec![1]),
        ));

        assert_eq!(first_focuses.load(Ordering::SeqCst), 1);
        assert_eq!(first_blurs.load(Ordering::SeqCst), 1);
        assert_eq!(second_focuses.load(Ordering::SeqCst), 1);
        assert_eq!(second_blurs.load(Ordering::SeqCst), 0);
    }

    #[test]
    fn pointer_capture_routes_move_and_up_to_owner() {
        let (capture_widget, seen_capture_moves) = FocusProbe::capture_probe();
        let (other_widget, _, _) = FocusProbe::new(true);
        let mut root = Container::column()
            .child(capture_widget)
            .child(other_widget);
        let (layout_tree, registry) = build_registry(&mut root);
        let focused_id = registry.id_for_path(&[0]).unwrap();

        let down = dispatch_widget_event(
            &mut root,
            &layout_tree,
            Some(focused_id),
            None,
            &InputEvent::PointerDown {
                pos: glam::vec2(4.0, 4.0),
                button: PointerButton::Primary,
            },
        );
        assert_eq!(down.capture_path.as_deref(), Some(&[0][..]));

        let move_event = dispatch_widget_event(
            &mut root,
            &layout_tree,
            Some(focused_id),
            down.capture_path.as_ref(),
            &InputEvent::PointerMove {
                pos: glam::vec2(40.0, 10.0),
            },
        );
        assert_eq!(seen_capture_moves.load(Ordering::SeqCst), 1);
        assert_eq!(move_event.capture_path.as_deref(), Some(&[0][..]));

        let up = dispatch_widget_event(
            &mut root,
            &layout_tree,
            Some(focused_id),
            move_event.capture_path.as_ref(),
            &InputEvent::PointerUp {
                pos: glam::vec2(40.0, 10.0),
                button: PointerButton::Primary,
            },
        );
        assert!(up.capture_path.is_none());
    }

    #[test]
    fn scroll_offsets_are_applied_to_descendant_hit_testing() {
        let spacer = ScrollHitProbe::new((200.0, 120.0)).0;
        let (target, seen_scroll) = ScrollHitProbe::new((200.0, 40.0));
        let content = Container::column().child(spacer).child(target);
        let mut root = OffsetScrollProbe::new(glam::vec2(0.0, 100.0), content);
        let (layout_tree, _) = build_registry(&mut root);

        let _ = dispatch_widget_event(
            &mut root,
            &layout_tree,
            None,
            None,
            &InputEvent::Scroll {
                pos: glam::vec2(10.0, 30.0),
                delta: glam::vec2(0.0, -1.0),
                modifiers: sparsha_input::Modifiers::default(),
            },
        );

        assert_eq!(seen_scroll.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn nested_scroll_wheel_events_reach_inner_scroll_container_first() {
        let (inner, inner_seen_scroll) = ScrollContainerProbe::new((200.0, 100.0));
        let (mut outer, outer_seen_scroll) = ScrollContainerProbe::new((200.0, 200.0));
        outer = outer.child(inner);
        let (layout_tree, _) = build_registry(&mut outer);

        let _ = dispatch_widget_event(
            &mut outer,
            &layout_tree,
            None,
            None,
            &InputEvent::Scroll {
                pos: glam::vec2(10.0, 10.0),
                delta: glam::vec2(0.0, -24.0),
                modifiers: sparsha_input::Modifiers::default(),
            },
        );

        assert_eq!(inner_seen_scroll.load(Ordering::SeqCst), 1);
        assert_eq!(outer_seen_scroll.load(Ordering::SeqCst), 0);
    }

    #[test]
    fn semantics_overrides_merge_into_descendant_accessible_node() {
        let mut root = Container::column().child(
            Semantics::new(Button::builder().label("Save").build())
                .label("Explicit accessible label"),
        );
        let (layout_tree, _) = build_registry(&mut root);
        let tree = collect_accessibility_tree(&root, &layout_tree, None);

        assert_eq!(tree.nodes.len(), 1);
        let node = &tree.nodes[0];
        assert_eq!(node.role, sparsha_widgets::AccessibilityRole::Button);
        assert_eq!(node.label.as_deref(), Some("Explicit accessible label"));
    }

    #[test]
    fn accessibility_ids_remain_stable_across_relayout() {
        let mut root = Container::column()
            .child(Button::builder().label("Primary").build())
            .child(TextInput::builder().placeholder("Email").build());
        let (layout_tree_a, _) = build_registry(&mut root);
        let tree_a = collect_accessibility_tree(&root, &layout_tree_a, Some(&vec![1]));

        let mut tree = LayoutTree::new();
        let mut registry = WidgetRuntimeRegistry::default();
        let mut text = TextSystem::new_headless();
        let mut path = Vec::new();
        let root_id = add_widget_to_layout(
            &mut root,
            &mut tree,
            &mut text,
            &mut registry,
            &mut path,
            false,
            true,
        );
        tree.set_root(root_id);
        tree.compute_layout(800.0, 600.0);
        let tree_b = collect_accessibility_tree(&root, &tree, Some(&vec![1]));

        assert_eq!(tree_a.root_children, tree_b.root_children);
        assert_eq!(tree_a.focus, tree_b.focus);
    }
}
