//! App router and navigation primitives.

use sparsha_layout::taffy::prelude::{percent, AlignItems, Display, FlexDirection, Size, Style};
use sparsha_layout::WidgetId;
use sparsha_signals::Signal;
use sparsha_widgets::{
    current_theme, current_viewport, AnimationEasing, BuildContext, Container, ImplicitAnimation,
    IntoWidget, Text, ViewportInfo, Widget, WidgetChildMode,
};
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
    sync::Arc,
};

#[derive(Clone)]
pub struct Route {
    path: String,
    builder: Arc<dyn Fn() -> Box<dyn Widget>>,
}

impl Route {
    pub fn new<W>(path: impl Into<String>, builder: impl Fn() -> W + 'static) -> Self
    where
        W: IntoWidget,
    {
        Self {
            path: normalize_path(&path.into()),
            builder: Arc::new(move || builder().into_widget()),
        }
    }

    pub fn path(&self) -> &str {
        &self.path
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct RouterTransition {
    pub duration_seconds: f32,
    pub easing: AnimationEasing,
    pub slide_distance: f32,
    pub overlay_alpha_peak: f32,
}

impl RouterTransition {
    pub fn slide_overlay() -> Self {
        Self {
            duration_seconds: 0.24,
            easing: AnimationEasing::EaseInOut,
            slide_distance: 24.0,
            overlay_alpha_peak: 0.16,
        }
    }

    fn sanitized(&self) -> Self {
        Self {
            duration_seconds: self.duration_seconds.max(0.000_001),
            easing: self.easing,
            slide_distance: self.slide_distance.max(0.0),
            overlay_alpha_peak: self.overlay_alpha_peak.clamp(0.0, 1.0),
        }
    }
}

impl Default for RouterTransition {
    fn default() -> Self {
        Self::slide_overlay()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum NavigationDirection {
    Forward,
    Backward,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct RouterState {
    history: Vec<String>,
    index: usize,
    last_direction: NavigationDirection,
}

impl RouterState {
    fn new(path: String) -> Self {
        Self {
            history: vec![path],
            index: 0,
            last_direction: NavigationDirection::Forward,
        }
    }

    fn current(&self) -> &str {
        self.history
            .get(self.index)
            .map(String::as_str)
            .unwrap_or("/")
    }

    fn push(&mut self, path: String) {
        self.last_direction = NavigationDirection::Forward;
        if self.current() == path {
            return;
        }
        self.history.truncate(self.index + 1);
        self.history.push(path);
        self.index = self.history.len() - 1;
    }

    fn replace(&mut self, path: String) {
        self.last_direction = NavigationDirection::Forward;
        if self.history.is_empty() {
            self.history.push(path);
            self.index = 0;
            return;
        }
        self.history[self.index] = path;
    }

    fn back(&mut self) -> bool {
        if self.index == 0 {
            return false;
        }
        self.index -= 1;
        self.last_direction = NavigationDirection::Backward;
        true
    }

    fn forward(&mut self) -> bool {
        if self.index + 1 >= self.history.len() {
            return false;
        }
        self.index += 1;
        self.last_direction = NavigationDirection::Forward;
        true
    }
}

#[derive(Clone)]
pub struct Router {
    routes: Vec<Route>,
    fallback_path: Option<String>,
    transition: Option<RouterTransition>,
    state: Signal<RouterState>,
}

impl Router {
    pub fn new() -> Self {
        Self {
            routes: Vec::new(),
            fallback_path: None,
            transition: None,
            state: Signal::new(RouterState::new(String::from("/"))),
        }
    }

    pub fn route<W>(self, path: impl Into<String>, builder: impl Fn() -> W + 'static) -> Self
    where
        W: IntoWidget,
    {
        self.add_route(Route::new(path, builder))
    }

    pub fn add_route(mut self, route: Route) -> Self {
        if !is_static_path(route.path()) {
            log::warn!(
                "ignoring invalid static route pattern '{}': dynamic patterns are not supported",
                route.path()
            );
            return self;
        }

        if let Some(existing) = self.routes.iter_mut().find(|it| it.path == route.path) {
            *existing = route;
        } else {
            self.routes.push(route);
        }

        self
    }

    pub fn transition(mut self, transition: RouterTransition) -> Self {
        self.transition = Some(transition.sanitized());
        self
    }

    pub fn fallback(mut self, path: impl Into<String>) -> Self {
        let path = normalize_path(&path.into());
        if !is_static_path(&path) {
            log::warn!(
                "ignoring invalid fallback route '{}': dynamic patterns are not supported",
                path
            );
            return self;
        }
        self.fallback_path = Some(path);
        self
    }

    pub fn navigator(&self) -> Navigator {
        Navigator {
            router: self.clone(),
        }
    }

    pub fn current_path(&self) -> String {
        self.state.with(|state| state.current().to_owned())
    }

    pub fn go(&self, path: impl Into<String>) {
        let path = self.resolve_path(&path.into());
        self.state.with_mut(|state| state.push(path));
    }

    pub fn push(&self, path: impl Into<String>) {
        self.go(path);
    }

    pub fn replace(&self, path: impl Into<String>) {
        let path = self.resolve_path(&path.into());
        self.state.with_mut(|state| state.replace(path));
    }

    pub fn back(&self) -> bool {
        self.state.with_mut(|state| state.back())
    }

    pub fn forward(&self) -> bool {
        self.state.with_mut(|state| state.forward())
    }

    pub(crate) fn initialize(&self, start_path: Option<&str>) {
        let requested = start_path
            .map(normalize_path)
            .unwrap_or_else(|| self.current_path());
        let resolved = self.resolve_path(&requested);
        self.state.set(RouterState::new(resolved));
    }

    pub(crate) fn build_for_current_path(&self) -> Box<dyn Widget> {
        let path = self.current_path();
        self.build_for_path(&path)
    }

    pub(crate) fn last_direction(&self) -> NavigationDirection {
        self.state.with(|state| state.last_direction)
    }

    pub(crate) fn transition_config(&self) -> Option<RouterTransition> {
        self.transition.clone()
    }

    fn build_for_path(&self, path: &str) -> Box<dyn Widget> {
        if let Some(route) = self.routes.iter().find(|route| route.path == path) {
            return (route.builder)();
        }

        let fallback = self.fallback_resolved();
        if let Some(route) = self.routes.iter().find(|route| route.path == fallback) {
            return (route.builder)();
        }

        Box::new(
            Container::new()
                .fill()
                .center()
                .child(Text::new("No routes registered")),
        )
    }

    fn resolve_path(&self, path: &str) -> String {
        let path = normalize_path(path);
        if self.routes.iter().any(|route| route.path == path) {
            return path;
        }
        self.fallback_resolved().to_owned()
    }

    fn fallback_resolved(&self) -> &str {
        if let Some(path) = &self.fallback_path {
            if self.routes.iter().any(|route| route.path == *path) {
                return path;
            }
        }

        if let Some(first) = self.routes.first() {
            return &first.path;
        }

        "/"
    }
}

impl Default for Router {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Clone)]
pub struct Navigator {
    router: Router,
}

impl Navigator {
    pub fn current_path(&self) -> String {
        self.router.current_path()
    }

    pub fn go(&self, path: impl Into<String>) {
        self.router.go(path);
    }

    pub fn push(&self, path: impl Into<String>) {
        self.router.push(path);
    }

    pub fn replace(&self, path: impl Into<String>) {
        self.router.replace(path);
    }

    pub fn back(&self) -> bool {
        self.router.back()
    }

    pub fn forward(&self) -> bool {
        self.router.forward()
    }

    #[cfg(target_arch = "wasm32")]
    pub(crate) fn sync_external_path(&self, path: &str) {
        self.router.replace(path.to_owned());
    }
}

#[derive(Default)]
struct RouteLayerState {
    offset_x: Cell<f32>,
}

struct RouteLayer {
    id: WidgetId,
    interactive: bool,
    state: Rc<RouteLayerState>,
    child: Box<dyn Widget>,
    translated: Cell<bool>,
}

impl RouteLayer {
    fn new(child: Box<dyn Widget>, state: Rc<RouteLayerState>, interactive: bool) -> Self {
        Self {
            id: WidgetId::default(),
            interactive,
            state,
            child,
            translated: Cell::new(false),
        }
    }
}

impl Widget for RouteLayer {
    fn id(&self) -> WidgetId {
        self.id
    }

    fn set_id(&mut self, id: WidgetId) {
        self.id = id;
    }

    fn style(&self) -> Style {
        Style {
            display: Display::Flex,
            flex_direction: FlexDirection::Column,
            align_items: Some(AlignItems::Stretch),
            size: Size {
                width: percent(1.0),
                height: percent(1.0),
            },
            ..Default::default()
        }
    }

    fn paint(&self, ctx: &mut sparsha_widgets::PaintContext) {
        let bounds = ctx.bounds();
        ctx.push_clip(bounds);

        let offset_x = self.state.offset_x.get();
        if offset_x.abs() > f32::EPSILON {
            ctx.push_translation((offset_x * ctx.scale_factor, 0.0));
            self.translated.set(true);
        } else {
            self.translated.set(false);
        }
    }

    fn paint_after_children(&self, ctx: &mut sparsha_widgets::PaintContext) {
        if self.translated.replace(false) {
            ctx.pop_translation();
        }
        ctx.pop_clip();
    }

    fn children(&self) -> &[Box<dyn Widget>] {
        std::slice::from_ref(&self.child)
    }

    fn children_mut(&mut self) -> &mut [Box<dyn Widget>] {
        std::slice::from_mut(&mut self.child)
    }

    fn child_mode(&self, _child_position: usize) -> WidgetChildMode {
        if self.interactive {
            WidgetChildMode::Active
        } else {
            WidgetChildMode::PaintOnly
        }
    }

    fn child_event_offset(&self) -> glam::Vec2 {
        if self.interactive {
            glam::vec2(-self.state.offset_x.get(), 0.0)
        } else {
            glam::Vec2::ZERO
        }
    }
}

struct HostTransition {
    animation: ImplicitAnimation,
    initialized: bool,
    cleanup_requested: bool,
    config: RouterTransition,
    direction: NavigationDirection,
    outgoing_state: Rc<RouteLayerState>,
    incoming_state: Rc<RouteLayerState>,
}

impl HostTransition {
    fn new(
        config: RouterTransition,
        direction: NavigationDirection,
        outgoing_state: Rc<RouteLayerState>,
        incoming_state: Rc<RouteLayerState>,
    ) -> Self {
        Self {
            animation: ImplicitAnimation::new(0.0),
            initialized: false,
            cleanup_requested: false,
            config,
            direction,
            outgoing_state,
            incoming_state,
        }
    }
}

pub(crate) struct RouterHost {
    id: WidgetId,
    router: Router,
    active_path: String,
    children: Vec<Box<dyn Widget>>,
    last_viewport: Option<ViewportInfo>,
    transition: RefCell<Option<HostTransition>>,
}

impl RouterHost {
    pub(crate) fn new(router: Router) -> Self {
        let active_path = router.resolve_path(&router.current_path());
        router.replace(active_path.clone());
        let child = build_route_layer(router.build_for_current_path(), true);
        Self {
            id: WidgetId::default(),
            router,
            active_path,
            children: vec![child],
            last_viewport: None,
            transition: RefCell::new(None),
        }
    }

    fn apply_transition_offsets(transition: &HostTransition, progress: f32) {
        let direction = match transition.direction {
            NavigationDirection::Forward => 1.0,
            NavigationDirection::Backward => -1.0,
        };
        let distance = transition.config.slide_distance;
        let incoming = direction * (1.0 - progress) * distance;
        let outgoing = -direction * progress * distance;
        transition.incoming_state.offset_x.set(incoming);
        transition.outgoing_state.offset_x.set(outgoing);
    }

    fn collapse_transition_layers(&mut self) {
        if self.children.len() == 2 {
            let active = self.children.pop().unwrap();
            self.children = vec![active];
        }
        *self.transition.get_mut() = None;
    }
}

fn build_route_layer(child: Box<dyn Widget>, interactive: bool) -> Box<dyn Widget> {
    Box::new(RouteLayer::new(
        child,
        Rc::new(RouteLayerState::default()),
        interactive,
    ))
}

fn build_transition_layers(
    router: &Router,
    from_path: &str,
    to_path: &str,
    config: RouterTransition,
    direction: NavigationDirection,
) -> (Vec<Box<dyn Widget>>, HostTransition) {
    let outgoing_state = Rc::new(RouteLayerState::default());
    let incoming_state = Rc::new(RouteLayerState::default());
    let outgoing = Box::new(RouteLayer::new(
        router.build_for_path(from_path),
        Rc::clone(&outgoing_state),
        false,
    ));
    let incoming = Box::new(RouteLayer::new(
        router.build_for_path(to_path),
        Rc::clone(&incoming_state),
        true,
    ));
    let transition = HostTransition::new(config, direction, outgoing_state, incoming_state);
    (vec![outgoing, incoming], transition)
}

impl Widget for RouterHost {
    fn id(&self) -> WidgetId {
        self.id
    }

    fn set_id(&mut self, id: WidgetId) {
        self.id = id;
    }

    fn style(&self) -> Style {
        Style {
            display: Display::Flex,
            flex_direction: FlexDirection::Column,
            align_items: Some(AlignItems::Stretch),
            size: Size {
                width: percent(1.0),
                height: percent(1.0),
            },
            ..Default::default()
        }
    }

    fn rebuild(&mut self, ctx: &mut BuildContext) {
        let path = self.router.resolve_path(&self.router.current_path());
        let viewport = ctx
            .resource::<ViewportInfo>()
            .unwrap_or_else(current_viewport);

        if self
            .transition
            .borrow()
            .as_ref()
            .is_some_and(|transition| transition.cleanup_requested)
        {
            self.collapse_transition_layers();
        }

        if self.last_viewport != Some(viewport) {
            self.last_viewport = Some(viewport);
            self.children = vec![build_route_layer(self.router.build_for_path(&path), true)];
            self.active_path = path;
            *self.transition.get_mut() = None;
            return;
        }

        if path == self.active_path && !self.children.is_empty() {
            return;
        }

        self.router.replace(path.clone());
        if self.children.is_empty() {
            self.children = vec![build_route_layer(self.router.build_for_path(&path), true)];
            self.active_path = path;
            *self.transition.get_mut() = None;
            return;
        }

        let previous_path = self.active_path.clone();
        self.active_path = path.clone();

        if let Some(config) = self.router.transition_config() {
            let direction = self.router.last_direction();
            let (children, transition) =
                build_transition_layers(&self.router, &previous_path, &path, config, direction);
            self.children = children;
            *self.transition.get_mut() = Some(transition);
        } else {
            self.children = vec![build_route_layer(self.router.build_for_path(&path), true)];
            *self.transition.get_mut() = None;
        }
    }

    fn paint(&self, ctx: &mut sparsha_widgets::PaintContext) {
        let mut transition_slot = self.transition.borrow_mut();
        let Some(transition) = transition_slot.as_mut() else {
            return;
        };

        if !transition.initialized {
            transition.animation.set_target(
                1.0,
                ctx.elapsed_time,
                transition.config.duration_seconds,
                transition.config.easing,
            );
            transition.initialized = true;
        }

        let progress = transition.animation.sample(ctx.elapsed_time);
        Self::apply_transition_offsets(transition, progress);

        let overlay_alpha =
            page_transition_overlay_alpha(progress, transition.config.overlay_alpha_peak);
        if overlay_alpha > 0.0 {
            let overlay = current_theme().colors.background.with_alpha(overlay_alpha);
            ctx.fill_rect(ctx.bounds(), overlay);
        }

        if transition.animation.is_animating() {
            ctx.request_next_frame();
        } else if !transition.cleanup_requested {
            transition.incoming_state.offset_x.set(0.0);
            transition.outgoing_state.offset_x.set(0.0);
            transition.cleanup_requested = true;
            ctx.request_layout();
        }
    }

    fn children(&self) -> &[Box<dyn Widget>] {
        &self.children
    }

    fn children_mut(&mut self) -> &mut [Box<dyn Widget>] {
        &mut self.children
    }

    fn child_path_key(&self, child_position: usize) -> usize {
        match self.children.len() {
            1 if child_position == 0 => 1,
            _ => child_position,
        }
    }

    fn child_slot_for_path_key(&self, key: usize) -> Option<usize> {
        match self.children.len() {
            1 if key == 1 => Some(0),
            1 => None,
            _ if key < self.children.len() => Some(key),
            _ => None,
        }
    }
}

pub fn path_to_hash(path: &str) -> String {
    format!("#{}", normalize_path(path))
}

pub fn hash_to_path(hash: &str) -> String {
    let raw = hash.trim();
    let raw = raw.strip_prefix('#').unwrap_or(raw);
    normalize_path(raw)
}

fn normalize_path(path: &str) -> String {
    let trimmed = path.trim();
    if trimmed.is_empty() || trimmed == "#" {
        return String::from("/");
    }

    let path = if trimmed.starts_with('/') {
        trimmed.to_owned()
    } else {
        format!("/{trimmed}")
    };

    if path.len() > 1 {
        path.trim_end_matches('/').to_owned()
    } else {
        path
    }
}

fn is_static_path(path: &str) -> bool {
    path.starts_with('/') && !path.contains(':') && !path.contains('*')
}

fn page_transition_overlay_alpha(progress: f32, peak_alpha: f32) -> f32 {
    let progress = progress.clamp(0.0, 1.0);
    let triangle = 1.0 - (progress * 2.0 - 1.0).abs();
    (triangle * peak_alpha).clamp(0.0, peak_alpha)
}

#[cfg(test)]
mod tests {
    use super::*;
    use sparsha_signals::RuntimeHandle;

    fn screen(name: &'static str) -> Container {
        Container::new().child(Text::new(name))
    }

    fn with_runtime(f: impl FnOnce()) {
        let runtime = RuntimeHandle::new();
        runtime.run_with_current(f);
    }

    #[test]
    fn router_transition_is_opt_in() {
        with_runtime(|| {
            let router = Router::new().route("/", || screen("home")).fallback("/");
            assert!(router.transition.is_none());

            let transitioned = Router::new()
                .route("/", || screen("home"))
                .transition(RouterTransition::slide_overlay())
                .fallback("/");
            assert_eq!(
                transitioned.transition,
                Some(RouterTransition::slide_overlay())
            );
        });
    }

    #[test]
    fn router_transition_sanitizes_invalid_values() {
        with_runtime(|| {
            let router = Router::new().transition(RouterTransition {
                duration_seconds: 0.0,
                easing: AnimationEasing::Linear,
                slide_distance: -12.0,
                overlay_alpha_peak: 4.0,
            });

            let transition = router.transition.unwrap();
            assert!(transition.duration_seconds > 0.0);
            assert_eq!(transition.slide_distance, 0.0);
            assert_eq!(transition.overlay_alpha_peak, 1.0);
        });
    }

    #[test]
    fn unknown_route_resolves_to_fallback() {
        with_runtime(|| {
            let router = Router::new()
                .route("/", || screen("home"))
                .route("/settings", || screen("settings"))
                .transition(RouterTransition::slide_overlay())
                .fallback("/");

            router.go("/missing");
            assert_eq!(router.current_path(), "/");
        });
    }

    #[test]
    fn navigation_stack_operations_work() {
        with_runtime(|| {
            let router = Router::new()
                .route("/", || screen("home"))
                .route("/a", || screen("a"))
                .route("/b", || screen("b"))
                .fallback("/");

            router.go("/a");
            router.push("/b");
            assert_eq!(router.current_path(), "/b");
            assert!(router.back());
            assert_eq!(router.current_path(), "/a");
            assert!(router.forward());
            assert_eq!(router.current_path(), "/b");

            router.replace("/a");
            assert_eq!(router.current_path(), "/a");
        });
    }

    #[test]
    fn navigation_direction_tracks_history_and_replacements() {
        with_runtime(|| {
            let router = Router::new()
                .route("/", || screen("home"))
                .route("/a", || screen("a"))
                .route("/b", || screen("b"))
                .fallback("/");

            router.go("/a");
            assert_eq!(router.last_direction(), NavigationDirection::Forward);

            assert!(router.back());
            assert_eq!(router.last_direction(), NavigationDirection::Backward);

            assert!(router.forward());
            assert_eq!(router.last_direction(), NavigationDirection::Forward);

            router.replace("/b");
            assert_eq!(router.last_direction(), NavigationDirection::Forward);
        });
    }

    #[test]
    fn dynamic_patterns_are_ignored() {
        with_runtime(|| {
            let router = Router::new()
                .route("/", || screen("home"))
                .route("/todos/:id", || screen("todo"))
                .fallback("/");

            router.go("/todos/1");
            assert_eq!(router.current_path(), "/");
        });
    }

    #[test]
    fn hash_path_roundtrip_helpers() {
        assert_eq!(hash_to_path("#/settings"), "/settings");
        assert_eq!(hash_to_path(""), "/");
        assert_eq!(path_to_hash("/"), "#/");
        assert_eq!(path_to_hash("settings"), "#/settings");
    }

    #[test]
    fn page_transition_alpha_peaks_midway() {
        let start = page_transition_overlay_alpha(0.0, 0.16);
        let mid = page_transition_overlay_alpha(0.5, 0.16);
        let end = page_transition_overlay_alpha(1.0, 0.16);
        assert_eq!(start, 0.0);
        assert_eq!(end, 0.0);
        assert!(mid > 0.0);
        assert!(mid > start);
        assert!(mid > end);
    }
}
