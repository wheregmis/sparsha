//! App router and navigation primitives.

use sparsha_layout::taffy::prelude::{percent, Size, Style};
use sparsha_layout::WidgetId;
use sparsha_signals::Signal;
use sparsha_widgets::{Container, IntoWidget, Text, Widget};
use std::sync::Arc;

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

#[derive(Clone, Debug, PartialEq, Eq)]
struct RouterState {
    history: Vec<String>,
    index: usize,
}

impl RouterState {
    fn new(path: String) -> Self {
        Self {
            history: vec![path],
            index: 0,
        }
    }

    fn current(&self) -> &str {
        self.history
            .get(self.index)
            .map(String::as_str)
            .unwrap_or("/")
    }

    fn push(&mut self, path: String) {
        if self.current() == path {
            return;
        }
        self.history.truncate(self.index + 1);
        self.history.push(path);
        self.index = self.history.len() - 1;
    }

    fn replace(&mut self, path: String) {
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
        true
    }

    fn forward(&mut self) -> bool {
        if self.index + 1 >= self.history.len() {
            return false;
        }
        self.index += 1;
        true
    }
}

#[derive(Clone)]
pub struct Router {
    routes: Vec<Route>,
    fallback_path: Option<String>,
    state: Signal<RouterState>,
}

impl Router {
    pub fn new() -> Self {
        Self {
            routes: Vec::new(),
            fallback_path: None,
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

pub(crate) struct RouterHost {
    id: WidgetId,
    router: Router,
    active_path: String,
    children: Vec<Box<dyn Widget>>,
}

impl RouterHost {
    pub(crate) fn new(router: Router) -> Self {
        let active_path = router.resolve_path(&router.current_path());
        router.replace(active_path.clone());
        let child = router.build_for_current_path();
        Self {
            id: WidgetId::default(),
            router,
            active_path,
            children: vec![child],
        }
    }
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
            size: Size {
                width: percent(1.0),
                height: percent(1.0),
            },
            ..Default::default()
        }
    }

    fn rebuild(&mut self, _ctx: &mut sparsha_widgets::BuildContext) {
        let path = self.router.resolve_path(&self.router.current_path());
        if path != self.active_path || self.children.is_empty() {
            self.router.replace(path.clone());
            self.active_path = path;
            self.children = vec![self.router.build_for_current_path()];
        }
    }

    fn paint(&self, _ctx: &mut sparsha_widgets::PaintContext) {}

    fn children(&self) -> &[Box<dyn Widget>] {
        &self.children
    }

    fn children_mut(&mut self) -> &mut [Box<dyn Widget>] {
        &mut self.children
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
    fn unknown_route_resolves_to_fallback() {
        with_runtime(|| {
            let router = Router::new()
                .route("/", || screen("home"))
                .route("/settings", || screen("settings"))
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
}
