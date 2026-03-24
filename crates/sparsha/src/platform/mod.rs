use crate::accessibility::AccessibilityTreeSnapshot;
use sparsha_input::ShortcutProfile;
use sparsha_widgets::TextEditorState;

pub(crate) mod events;
#[cfg(not(target_arch = "wasm32"))]
pub(crate) mod native;
#[cfg(target_arch = "wasm32")]
pub(crate) mod web;

#[cfg(not(target_arch = "wasm32"))]
pub(crate) use native::NativePlatform;
#[cfg(target_arch = "wasm32")]
pub(crate) use web::WebPlatform;

#[allow(dead_code)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) enum PlatformId {
    MacOs,
    Windows,
    Linux,
    Web,
}

#[allow(dead_code)]
impl PlatformId {
    pub(crate) const ALL: [Self; 4] = [Self::MacOs, Self::Windows, Self::Linux, Self::Web];

    pub(crate) const fn current_native() -> Self {
        #[cfg(target_os = "macos")]
        {
            Self::MacOs
        }

        #[cfg(target_os = "windows")]
        {
            Self::Windows
        }

        #[cfg(all(not(target_os = "macos"), not(target_os = "windows")))]
        {
            Self::Linux
        }
    }

    pub(crate) const fn shortcut_profile(self) -> ShortcutProfile {
        match self {
            Self::MacOs | Self::Web => ShortcutProfile::CommandPrimary,
            Self::Windows | Self::Linux => ShortcutProfile::ControlPrimary,
        }
    }
}

#[allow(dead_code)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) enum PlatformFeature {
    ShortcutProfile,
    PointerCapture,
    ClipboardRead,
    ClipboardWrite,
    ImeComposition,
    TextInputBridge,
    AccessibilityTree,
    SemanticDom,
    NativeAccessibility,
    TextMetricsFallback,
    BackgroundTasks,
    HybridSurface,
    ViewportSync,
    ScrollNormalization,
    TouchInput,
}

#[allow(dead_code)]
impl PlatformFeature {
    pub(crate) const ALL: [Self; 15] = [
        Self::ShortcutProfile,
        Self::PointerCapture,
        Self::ClipboardRead,
        Self::ClipboardWrite,
        Self::ImeComposition,
        Self::TextInputBridge,
        Self::AccessibilityTree,
        Self::SemanticDom,
        Self::NativeAccessibility,
        Self::TextMetricsFallback,
        Self::BackgroundTasks,
        Self::HybridSurface,
        Self::ViewportSync,
        Self::ScrollNormalization,
        Self::TouchInput,
    ];
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) enum SupportLevel {
    Native,
    Emulated,
    Partial,
    Unsupported,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) enum FallbackPolicy {
    NoOpWarn,
    Degrade,
    HardError,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) struct FeatureSupport {
    pub(crate) support: SupportLevel,
    pub(crate) fallback: FallbackPolicy,
    pub(crate) rationale: &'static str,
}

impl FeatureSupport {
    const fn native(rationale: &'static str) -> Self {
        Self {
            support: SupportLevel::Native,
            fallback: FallbackPolicy::HardError,
            rationale,
        }
    }

    const fn emulated(rationale: &'static str) -> Self {
        Self {
            support: SupportLevel::Emulated,
            fallback: FallbackPolicy::Degrade,
            rationale,
        }
    }

    const fn partial(rationale: &'static str) -> Self {
        Self {
            support: SupportLevel::Partial,
            fallback: FallbackPolicy::Degrade,
            rationale,
        }
    }

    const fn unsupported(rationale: &'static str) -> Self {
        Self {
            support: SupportLevel::Unsupported,
            fallback: FallbackPolicy::NoOpWarn,
            rationale,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) struct PlatformCapabilities {
    platform: PlatformId,
}

#[allow(dead_code)]
impl PlatformCapabilities {
    pub(crate) const fn new(platform: PlatformId) -> Self {
        Self { platform }
    }

    pub(crate) const fn platform(self) -> PlatformId {
        self.platform
    }

    pub(crate) const fn shortcut_profile(self) -> ShortcutProfile {
        self.platform.shortcut_profile()
    }

    pub(crate) const fn support(self, feature: PlatformFeature) -> FeatureSupport {
        use PlatformFeature as Feature;
        use PlatformId as Platform;

        match (self.platform, feature) {
            (Platform::MacOs, Feature::ShortcutProfile)
            | (Platform::Windows, Feature::ShortcutProfile)
            | (Platform::Linux, Feature::ShortcutProfile)
            | (Platform::Web, Feature::ShortcutProfile) => {
                FeatureSupport::native("shortcut mapping is defined by an explicit profile")
            }

            (Platform::MacOs, Feature::PointerCapture)
            | (Platform::Windows, Feature::PointerCapture)
            | (Platform::Linux, Feature::PointerCapture)
            | (Platform::Web, Feature::PointerCapture) => {
                FeatureSupport::emulated("pointer capture is tracked in the runtime dispatch layer")
            }

            (Platform::MacOs, Feature::ClipboardRead)
            | (Platform::Windows, Feature::ClipboardRead)
            | (Platform::Linux, Feature::ClipboardRead) => {
                FeatureSupport::native("desktop runtimes can read the system clipboard directly")
            }
            (Platform::Web, Feature::ClipboardRead) => FeatureSupport::partial(
                "web relies on browser paste events instead of direct clipboard reads",
            ),

            (Platform::MacOs, Feature::ClipboardWrite)
            | (Platform::Windows, Feature::ClipboardWrite)
            | (Platform::Linux, Feature::ClipboardWrite) => {
                FeatureSupport::native("desktop runtimes can write the system clipboard directly")
            }
            (Platform::Web, Feature::ClipboardWrite) => {
                FeatureSupport::emulated("web writes through browser clipboard events")
            }

            (Platform::MacOs, Feature::ImeComposition)
            | (Platform::Windows, Feature::ImeComposition)
            | (Platform::Linux, Feature::ImeComposition)
            | (Platform::Web, Feature::ImeComposition) => {
                FeatureSupport::native("native and browser runtimes expose composition events")
            }

            (Platform::Web, Feature::TextInputBridge) => {
                FeatureSupport::emulated("web uses a hidden textarea bridge to align browser editing")
            }
            (Platform::MacOs, Feature::TextInputBridge)
            | (Platform::Windows, Feature::TextInputBridge)
            | (Platform::Linux, Feature::TextInputBridge) => FeatureSupport::unsupported(
                "desktop runtimes use the window IME path and do not need a DOM text bridge",
            ),

            (Platform::MacOs, Feature::AccessibilityTree)
            | (Platform::Windows, Feature::AccessibilityTree)
            | (Platform::Linux, Feature::AccessibilityTree)
            | (Platform::Web, Feature::AccessibilityTree) => {
                FeatureSupport::native("all runtimes build the same accessibility snapshot model")
            }

            (Platform::Web, Feature::SemanticDom) => {
                FeatureSupport::emulated("web mirrors the accessibility tree into a semantic DOM layer")
            }
            (Platform::MacOs, Feature::SemanticDom)
            | (Platform::Windows, Feature::SemanticDom)
            | (Platform::Linux, Feature::SemanticDom) => {
                FeatureSupport::unsupported("semantic DOM is only relevant on the web runtime")
            }

            (Platform::MacOs, Feature::NativeAccessibility)
            | (Platform::Windows, Feature::NativeAccessibility)
            | (Platform::Linux, Feature::NativeAccessibility) => {
                FeatureSupport::native("desktop runtimes bridge the shared tree into AccessKit")
            }
            (Platform::Web, Feature::NativeAccessibility) => {
                FeatureSupport::unsupported("the web runtime exposes accessibility through semantic DOM")
            }

            (Platform::Web, Feature::TextMetricsFallback) => FeatureSupport::emulated(
                "web uses a DOM-backed measurement fallback when headless shaping is not sufficient",
            ),
            (Platform::MacOs, Feature::TextMetricsFallback)
            | (Platform::Windows, Feature::TextMetricsFallback)
            | (Platform::Linux, Feature::TextMetricsFallback) => FeatureSupport::unsupported(
                "desktop text measurement uses embedded fonts and Parley directly",
            ),

            (Platform::MacOs, Feature::BackgroundTasks)
            | (Platform::Windows, Feature::BackgroundTasks)
            | (Platform::Linux, Feature::BackgroundTasks) => {
                FeatureSupport::native("desktop runtimes use Tokio worker threads")
            }
            (Platform::Web, Feature::BackgroundTasks) => {
                FeatureSupport::emulated("web runtime uses a Web Worker pool")
            }

            (Platform::Web, Feature::HybridSurface) => {
                FeatureSupport::partial("hybrid surfaces are available through DrawSurface only")
            }
            (Platform::MacOs, Feature::HybridSurface)
            | (Platform::Windows, Feature::HybridSurface)
            | (Platform::Linux, Feature::HybridSurface) => {
                FeatureSupport::unsupported("desktop apps already render through the native GPU surface")
            }

            (Platform::MacOs, Feature::ViewportSync)
            | (Platform::Windows, Feature::ViewportSync)
            | (Platform::Linux, Feature::ViewportSync)
            | (Platform::Web, Feature::ViewportSync) => {
                FeatureSupport::native("all runtimes resync viewport state from the host platform")
            }

            (Platform::MacOs, Feature::ScrollNormalization)
            | (Platform::Windows, Feature::ScrollNormalization)
            | (Platform::Linux, Feature::ScrollNormalization)
            | (Platform::Web, Feature::ScrollNormalization) => FeatureSupport::emulated(
                "runtimes normalize wheel and gesture deltas into shared scroll events",
            ),

            (Platform::Web, Feature::TouchInput) => {
                FeatureSupport::native("web runtime maps touch events into shared pointer input")
            }
            (Platform::MacOs, Feature::TouchInput)
            | (Platform::Windows, Feature::TouchInput)
            | (Platform::Linux, Feature::TouchInput) => FeatureSupport::partial(
                "desktop runner currently prioritizes pointer/mouse input over dedicated touch handling",
            ),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum PlatformEffect {
    SyncTextInput,
    SyncPointerCapture,
    SyncAccessibility,
    WriteClipboard(String),
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct PlatformEffects {
    effects: Vec<PlatformEffect>,
}

#[allow(dead_code)]
impl PlatformEffects {
    pub(crate) fn push(&mut self, effect: PlatformEffect) {
        self.effects.push(effect);
    }

    pub(crate) fn extend(&mut self, other: PlatformEffects) {
        self.effects.extend(other.effects);
    }

    pub(crate) fn iter(&self) -> impl Iterator<Item = &PlatformEffect> {
        self.effects.iter()
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.effects.is_empty()
    }
}

#[allow(dead_code)]
pub(crate) trait ClipboardService {
    fn read_text(&mut self) -> Option<String>;
    fn write_text(&mut self, text: &str);
}

#[allow(dead_code)]
pub(crate) trait TextInputService {
    fn sync_editor_state(&mut self, editor_state: Option<&TextEditorState>, suppress_bridge: bool);
}

#[allow(dead_code)]
pub(crate) trait PointerCaptureService {
    fn sync_capture(&mut self, has_capture: bool);
}

#[allow(dead_code)]
pub(crate) trait AccessibilityBackend {
    fn update_accessibility(&mut self, title: &str, snapshot: &AccessibilityTreeSnapshot);
}

#[allow(dead_code)]
pub(crate) trait SurfaceBackend {
    fn request_redraw(&mut self);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn capability_matrix_covers_every_platform_feature_pair() {
        for platform in PlatformId::ALL {
            let capabilities = PlatformCapabilities::new(platform);
            for feature in PlatformFeature::ALL {
                let support = capabilities.support(feature);
                assert!(
                    !support.rationale.trim().is_empty(),
                    "missing rationale for {platform:?} {feature:?}"
                );
            }
        }
    }

    #[test]
    fn web_uses_command_primary_shortcuts() {
        let capabilities = PlatformCapabilities::new(PlatformId::Web);
        assert_eq!(
            capabilities.shortcut_profile(),
            ShortcutProfile::CommandPrimary
        );
    }

    #[test]
    fn native_hybrid_surface_support_is_explicitly_missing() {
        let capabilities = PlatformCapabilities::new(PlatformId::Linux);
        let support = capabilities.support(PlatformFeature::HybridSurface);
        assert_eq!(support.support, SupportLevel::Unsupported);
        assert_eq!(support.fallback, FallbackPolicy::NoOpWarn);
    }
}
