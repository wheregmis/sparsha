#![cfg(target_arch = "wasm32")]

use crate::accessibility::{
    AccessibilityNodeSnapshot, AccessibilityTreeSnapshot, ACCESSIBILITY_ROOT_ID,
};
use crate::platform::events::WebEventTranslator;
use crate::platform::{
    ClipboardService, FeatureSupport, PlatformCapabilities, PlatformEffect, PlatformEffects,
    PlatformFeature, PlatformId, PointerCaptureService, SupportLevel, TextInputService,
};
use sparsha_input::ShortcutProfile;
use sparsha_widgets::{AccessibilityAction, AccessibilityRole, TextEditorState};
use wasm_bindgen::JsCast;
use web_sys::{Document, HtmlElement, HtmlInputElement, HtmlTextAreaElement};

pub(crate) struct WebPlatform {
    platform_id: PlatformId,
    capabilities: PlatformCapabilities,
    event_translator: WebEventTranslator,
    text_input_bridge: WebTextInputBridge,
    semantic_dom: WebSemanticDomLayer,
    pending_clipboard_write: Option<String>,
    accessibility_text_focus_node: Option<u64>,
}

impl WebPlatform {
    pub(crate) fn new(
        document: &Document,
        root: &HtmlElement,
    ) -> Result<Self, wasm_bindgen::JsValue> {
        let platform_id = PlatformId::Web;
        Ok(Self {
            platform_id,
            capabilities: PlatformCapabilities::new(platform_id),
            event_translator: WebEventTranslator::new(),
            text_input_bridge: WebTextInputBridge::new(document, root)?,
            semantic_dom: WebSemanticDomLayer::new(document, root)?,
            pending_clipboard_write: None,
            accessibility_text_focus_node: None,
        })
    }

    pub(crate) fn shortcut_profile(&self) -> ShortcutProfile {
        self.capabilities.shortcut_profile()
    }

    pub(crate) fn event_translator(&self) -> &WebEventTranslator {
        &self.event_translator
    }

    pub(crate) fn text_input_element(&self) -> &HtmlTextAreaElement {
        self.text_input_bridge.element()
    }

    pub(crate) fn text_input_is_syncing(&self) -> bool {
        self.text_input_bridge.is_syncing()
    }

    pub(crate) fn semantic_root(&self) -> &HtmlElement {
        self.semantic_dom.root()
    }

    pub(crate) fn render_semantic_dom(
        &self,
        snapshot: &AccessibilityTreeSnapshot,
    ) -> Result<(), wasm_bindgen::JsValue> {
        self.semantic_dom.render(snapshot)
    }

    pub(crate) fn accessibility_text_focus_matches_widget_focus(
        &self,
        widget_registry: &crate::runtime_widget::WidgetRuntimeRegistry,
        focused_path: Option<&[usize]>,
    ) -> bool {
        let Some(node_id) = self.accessibility_text_focus_node else {
            return false;
        };
        let Some(path) = widget_registry.path_for_accessibility_node(node_id) else {
            return false;
        };
        focused_path == Some(path) && widget_registry.text_editor_state_for_path(path).is_some()
    }

    pub(crate) fn set_accessibility_text_focus_node(&mut self, node_id: Option<u64>) {
        self.accessibility_text_focus_node = node_id;
    }

    pub(crate) fn apply_effects(
        &mut self,
        effects: &PlatformEffects,
        focused_editor_state: Option<&TextEditorState>,
        has_capture: bool,
        suppress_bridge: bool,
    ) {
        let mut clipboard = WebClipboardBridge {
            pending_write: &mut self.pending_clipboard_write,
        };
        let mut text_input = WebTextInputService {
            bridge: &mut self.text_input_bridge,
        };
        let mut pointer_capture = WebPointerCaptureBridge;

        apply_effects_with_services(
            self.platform_id,
            self.capabilities,
            effects,
            focused_editor_state,
            has_capture,
            suppress_bridge,
            &mut clipboard,
            &mut text_input,
            &mut pointer_capture,
        );
    }

    pub(crate) fn sync_text_input_bridge(
        &mut self,
        editor_state: Option<&TextEditorState>,
        suppress_bridge: bool,
    ) {
        if suppress_bridge {
            self.text_input_bridge.sync(None);
        } else {
            self.text_input_bridge.sync(editor_state);
        }
    }

    pub(crate) fn take_pending_clipboard_write(&mut self) -> Option<String> {
        self.pending_clipboard_write.take()
    }
}

struct WebClipboardBridge<'a> {
    pending_write: &'a mut Option<String>,
}

impl ClipboardService for WebClipboardBridge<'_> {
    fn read_text(&mut self) -> Option<String> {
        None
    }

    fn write_text(&mut self, text: &str) {
        *self.pending_write = Some(text.to_owned());
    }
}

struct WebTextInputService<'a> {
    bridge: &'a mut WebTextInputBridge,
}

impl TextInputService for WebTextInputService<'_> {
    fn sync_editor_state(&mut self, editor_state: Option<&TextEditorState>, suppress_bridge: bool) {
        if suppress_bridge {
            self.bridge.sync(None);
        } else {
            self.bridge.sync(editor_state);
        }
    }
}

struct WebPointerCaptureBridge;

impl PointerCaptureService for WebPointerCaptureBridge {
    fn sync_capture(&mut self, _has_capture: bool) {}
}

fn apply_effects_with_services(
    platform_id: PlatformId,
    capabilities: PlatformCapabilities,
    effects: &PlatformEffects,
    focused_editor_state: Option<&TextEditorState>,
    has_capture: bool,
    suppress_bridge: bool,
    clipboard: &mut dyn ClipboardService,
    text_input: &mut dyn TextInputService,
    pointer_capture: &mut dyn PointerCaptureService,
) {
    for effect in effects.iter() {
        if let Some(support) = degraded_effect_support(capabilities, effect) {
            log::warn!(
                "platform {:?} handling {:?} via {:?}: {}",
                platform_id,
                feature_for_effect(effect),
                support.fallback,
                support.rationale
            );
        }
    }

    for effect in effects.iter() {
        match effect {
            PlatformEffect::SyncTextInput => {
                text_input.sync_editor_state(focused_editor_state, suppress_bridge)
            }
            PlatformEffect::SyncPointerCapture => pointer_capture.sync_capture(has_capture),
            PlatformEffect::SyncAccessibility => {}
            PlatformEffect::WriteClipboard(text) => clipboard.write_text(text),
        }
    }
}

fn feature_for_effect(effect: &PlatformEffect) -> PlatformFeature {
    match effect {
        PlatformEffect::SyncTextInput => PlatformFeature::TextInputBridge,
        PlatformEffect::SyncPointerCapture => PlatformFeature::PointerCapture,
        PlatformEffect::SyncAccessibility => PlatformFeature::AccessibilityTree,
        PlatformEffect::WriteClipboard(_) => PlatformFeature::ClipboardWrite,
    }
}

fn degraded_effect_support(
    capabilities: PlatformCapabilities,
    effect: &PlatformEffect,
) -> Option<FeatureSupport> {
    let support = capabilities.support(feature_for_effect(effect));
    matches!(
        support.support,
        SupportLevel::Partial | SupportLevel::Unsupported
    )
    .then_some(support)
}

pub(crate) struct WebTextInputBridge {
    element: HtmlTextAreaElement,
    syncing: bool,
}

impl WebTextInputBridge {
    fn new(document: &Document, root: &HtmlElement) -> Result<Self, wasm_bindgen::JsValue> {
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

    fn sync(&mut self, editor_state: Option<&TextEditorState>) {
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

pub(crate) struct WebSemanticDomLayer {
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
        nodes: &std::collections::HashMap<u64, &AccessibilityNodeSnapshot>,
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
        nodes: &std::collections::HashMap<u64, &AccessibilityNodeSnapshot>,
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

pub(crate) fn create_semantic_element(
    document: &Document,
    node: &AccessibilityNodeSnapshot,
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

#[cfg(all(test, target_arch = "wasm32"))]
mod wasm_tests {
    use super::*;
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
            &AccessibilityNodeSnapshot {
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
    fn web_platform_tracks_text_focus_suppression() {
        let host = document()
            .create_element("div")
            .expect("host")
            .dyn_into::<HtmlElement>()
            .expect("html");
        document()
            .body()
            .expect("body")
            .append_child(&host)
            .expect("mount");

        let mut platform = WebPlatform::new(&document(), &host).expect("platform");
        platform.set_accessibility_text_focus_node(Some(42));
        let registry = crate::runtime_widget::WidgetRuntimeRegistry::default();

        assert!(!platform.accessibility_text_focus_matches_widget_focus(&registry, None));
        platform.sync_text_input_bridge(None, true);
        assert!(!platform.text_input_is_syncing());
    }

    #[test]
    fn web_effect_support_only_warns_for_partial_or_unsupported_features() {
        let capabilities = PlatformCapabilities::new(PlatformId::Web);
        assert!(degraded_effect_support(capabilities, &PlatformEffect::SyncTextInput).is_none());
        assert!(
            degraded_effect_support(capabilities, &PlatformEffect::SyncPointerCapture).is_none()
        );
        assert!(
            degraded_effect_support(capabilities, &PlatformEffect::SyncAccessibility).is_none()
        );
    }
}
