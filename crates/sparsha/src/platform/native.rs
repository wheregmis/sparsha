use crate::accessibility::AccessibilityTreeSnapshot;
use crate::platform::events::NativeEventTranslator;
use crate::platform::{
    AccessibilityBackend, ClipboardService, FeatureSupport, PlatformCapabilities, PlatformEffect,
    PlatformEffects, PlatformFeature, PlatformId, PointerCaptureService, SupportLevel,
    TextInputService,
};
use sparsha_input::ShortcutProfile;
use sparsha_widgets::TextEditorState;
use std::sync::{Arc, Mutex};

pub(crate) struct NativePlatform {
    platform_id: PlatformId,
    capabilities: PlatformCapabilities,
    event_translator: NativeEventTranslator,
    clipboard: Option<arboard::Clipboard>,
    accessibility_snapshot: Arc<Mutex<AccessibilityTreeSnapshot>>,
    accessibility_adapter: Option<accesskit_winit::Adapter>,
}

impl NativePlatform {
    pub(crate) fn new(platform_id: PlatformId) -> Self {
        let clipboard = match arboard::Clipboard::new() {
            Ok(clipboard) => Some(clipboard),
            Err(err) => {
                log::warn!("native clipboard unavailable: {err}");
                None
            }
        };

        Self {
            platform_id,
            capabilities: PlatformCapabilities::new(platform_id),
            event_translator: NativeEventTranslator::new(platform_id),
            clipboard,
            accessibility_snapshot: Arc::new(Mutex::new(AccessibilityTreeSnapshot::default())),
            accessibility_adapter: None,
        }
    }

    pub(crate) fn shortcut_profile(&self) -> ShortcutProfile {
        self.capabilities.shortcut_profile()
    }

    pub(crate) fn event_translator(&self) -> &NativeEventTranslator {
        &self.event_translator
    }

    pub(crate) fn read_clipboard_text(&mut self) -> Option<String> {
        let mut clipboard = NativeClipboardBridge {
            clipboard: &mut self.clipboard,
        };
        clipboard.read_text()
    }

    pub(crate) fn activation_handler(&self, title: String) -> NativeAccessibilityActivationHandler {
        NativeAccessibilityActivationHandler {
            title,
            snapshot: Arc::clone(&self.accessibility_snapshot),
        }
    }

    pub(crate) fn set_accessibility_adapter(&mut self, adapter: accesskit_winit::Adapter) {
        self.accessibility_adapter = Some(adapter);
    }

    pub(crate) fn accessibility_adapter_mut(&mut self) -> Option<&mut accesskit_winit::Adapter> {
        self.accessibility_adapter.as_mut()
    }

    pub(crate) fn apply_effects(
        &mut self,
        window: &winit::window::Window,
        title: &str,
        effects: &PlatformEffects,
        focused_editor_state: Option<&TextEditorState>,
        has_capture: bool,
        snapshot: &AccessibilityTreeSnapshot,
    ) {
        let mut clipboard = NativeClipboardBridge {
            clipboard: &mut self.clipboard,
        };
        let mut text_input = NativeTextInputBridge { window };
        let mut pointer_capture = NativePointerCaptureBridge;
        let mut accessibility = NativeAccessibilityBridge {
            snapshot: &self.accessibility_snapshot,
            adapter: self.accessibility_adapter.as_mut(),
        };

        apply_effects_with_services(
            self.platform_id,
            self.capabilities,
            title,
            effects,
            focused_editor_state,
            has_capture,
            snapshot,
            &mut clipboard,
            &mut text_input,
            &mut pointer_capture,
            &mut accessibility,
        );
    }
}

pub(crate) struct NativeAccessibilityActivationHandler {
    title: String,
    snapshot: Arc<Mutex<AccessibilityTreeSnapshot>>,
}

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

struct NativeClipboardBridge<'a> {
    clipboard: &'a mut Option<arboard::Clipboard>,
}

impl ClipboardService for NativeClipboardBridge<'_> {
    fn read_text(&mut self) -> Option<String> {
        let clipboard = self.clipboard.as_mut()?;
        match clipboard.get_text() {
            Ok(text) => Some(text),
            Err(err) => {
                log::warn!("failed to read clipboard text: {err}");
                None
            }
        }
    }

    fn write_text(&mut self, text: &str) {
        let Some(clipboard) = self.clipboard.as_mut() else {
            return;
        };
        if let Err(err) = clipboard.set_text(text.to_owned()) {
            log::warn!("failed to write clipboard text: {err}");
        }
    }
}

struct NativeTextInputBridge<'a> {
    window: &'a winit::window::Window,
}

impl TextInputService for NativeTextInputBridge<'_> {
    fn sync_editor_state(
        &mut self,
        editor_state: Option<&TextEditorState>,
        _suppress_bridge: bool,
    ) {
        set_native_ime_allowed(self.window, editor_state.is_some());
    }
}

struct NativePointerCaptureBridge;

impl PointerCaptureService for NativePointerCaptureBridge {
    fn sync_capture(&mut self, _has_capture: bool) {}
}

struct NativeAccessibilityBridge<'a> {
    snapshot: &'a Arc<Mutex<AccessibilityTreeSnapshot>>,
    adapter: Option<&'a mut accesskit_winit::Adapter>,
}

impl AccessibilityBackend for NativeAccessibilityBridge<'_> {
    fn update_accessibility(&mut self, title: &str, snapshot: &AccessibilityTreeSnapshot) {
        match self.snapshot.lock() {
            Ok(mut guard) => {
                *guard = snapshot.clone();
            }
            Err(poisoned) => {
                log::warn!("recovering from poisoned accessibility snapshot");
                *poisoned.into_inner() = snapshot.clone();
            }
        }

        if let Some(adapter) = self.adapter.as_mut() {
            adapter.update_if_active(|| snapshot.to_tree_update(title));
        }
    }
}

fn apply_effects_with_services(
    platform_id: PlatformId,
    capabilities: PlatformCapabilities,
    title: &str,
    effects: &PlatformEffects,
    focused_editor_state: Option<&TextEditorState>,
    has_capture: bool,
    snapshot: &AccessibilityTreeSnapshot,
    clipboard: &mut dyn ClipboardService,
    text_input: &mut dyn TextInputService,
    pointer_capture: &mut dyn PointerCaptureService,
    accessibility: &mut dyn AccessibilityBackend,
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
                text_input.sync_editor_state(focused_editor_state, false)
            }
            PlatformEffect::SyncPointerCapture => pointer_capture.sync_capture(has_capture),
            PlatformEffect::SyncAccessibility => {
                accessibility.update_accessibility(title, snapshot);
            }
            PlatformEffect::WriteClipboard(text) => clipboard.write_text(text),
        }
    }
}

fn feature_for_effect(effect: &PlatformEffect) -> PlatformFeature {
    match effect {
        PlatformEffect::SyncTextInput => PlatformFeature::ImeComposition,
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

#[allow(deprecated)]
fn set_native_ime_allowed(window: &winit::window::Window, allowed: bool) {
    window.set_ime_allowed(allowed);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Default)]
    struct TestClipboard {
        reads: Option<String>,
        writes: Vec<String>,
    }

    impl ClipboardService for TestClipboard {
        fn read_text(&mut self) -> Option<String> {
            self.reads.clone()
        }

        fn write_text(&mut self, text: &str) {
            self.writes.push(text.to_owned());
        }
    }

    #[derive(Default)]
    struct TestTextInput {
        sync_states: Vec<bool>,
    }

    impl TextInputService for TestTextInput {
        fn sync_editor_state(
            &mut self,
            editor_state: Option<&TextEditorState>,
            suppress_bridge: bool,
        ) {
            assert!(!suppress_bridge);
            self.sync_states.push(editor_state.is_some());
        }
    }

    #[derive(Default)]
    struct TestPointerCapture {
        capture_states: Vec<bool>,
    }

    impl PointerCaptureService for TestPointerCapture {
        fn sync_capture(&mut self, has_capture: bool) {
            self.capture_states.push(has_capture);
        }
    }

    #[derive(Default)]
    struct TestAccessibility {
        updates: Vec<(String, AccessibilityTreeSnapshot)>,
    }

    impl AccessibilityBackend for TestAccessibility {
        fn update_accessibility(&mut self, title: &str, snapshot: &AccessibilityTreeSnapshot) {
            self.updates.push((title.to_owned(), snapshot.clone()));
        }
    }

    #[test]
    fn native_effects_write_clipboard_and_sync_services() {
        let capabilities = PlatformCapabilities::new(PlatformId::Linux);
        let mut effects = PlatformEffects::default();
        effects.push(PlatformEffect::SyncTextInput);
        effects.push(PlatformEffect::SyncPointerCapture);
        effects.push(PlatformEffect::SyncAccessibility);
        effects.push(PlatformEffect::WriteClipboard("copied".to_owned()));

        let mut clipboard = TestClipboard::default();
        let mut text_input = TestTextInput::default();
        let mut pointer_capture = TestPointerCapture::default();
        let mut accessibility = TestAccessibility::default();
        let snapshot = AccessibilityTreeSnapshot::default();
        let editor_state = TextEditorState {
            text: "hello".to_owned(),
            cursor: 5,
            anchor: 5,
            multiline: false,
            composing_range: None,
        };

        apply_effects_with_services(
            PlatformId::Linux,
            capabilities,
            "Native Test",
            &effects,
            Some(&editor_state),
            true,
            &snapshot,
            &mut clipboard,
            &mut text_input,
            &mut pointer_capture,
            &mut accessibility,
        );

        assert_eq!(clipboard.writes, vec!["copied".to_owned()]);
        assert_eq!(text_input.sync_states, vec![true]);
        assert_eq!(pointer_capture.capture_states, vec![true]);
        assert_eq!(accessibility.updates.len(), 1);
        assert_eq!(accessibility.updates[0].0, "Native Test");
    }

    #[test]
    fn native_effect_support_only_warns_for_partial_or_unsupported_features() {
        let capabilities = PlatformCapabilities::new(PlatformId::Linux);
        assert!(degraded_effect_support(capabilities, &PlatformEffect::SyncTextInput).is_none());
        assert!(
            degraded_effect_support(capabilities, &PlatformEffect::SyncPointerCapture).is_none()
        );
        assert!(
            degraded_effect_support(capabilities, &PlatformEffect::SyncAccessibility).is_none()
        );
    }
}
