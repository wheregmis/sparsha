//! Public accessibility metadata shared by widgets and runtimes.

/// Generic accessibility roles supported by built-in widgets.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AccessibilityRole {
    GenericContainer,
    Button,
    CheckBox,
    Label,
    TextInput,
    MultilineTextInput,
    List,
    ScrollView,
}

/// Accessibility actions that runtimes or assistive technologies can request.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AccessibilityAction {
    Click,
    Focus,
    SetValue,
    ScrollUp,
    ScrollDown,
    ScrollLeft,
    ScrollRight,
}

/// Widget-provided accessibility metadata.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct AccessibilityInfo {
    pub role: Option<AccessibilityRole>,
    pub label: Option<String>,
    pub description: Option<String>,
    pub value: Option<String>,
    pub hidden: bool,
    pub disabled: bool,
    pub checked: Option<bool>,
    pub actions: Vec<AccessibilityAction>,
}

impl AccessibilityInfo {
    pub fn new(role: AccessibilityRole) -> Self {
        Self {
            role: Some(role),
            ..Default::default()
        }
    }

    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }

    pub fn description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    pub fn value(mut self, value: impl Into<String>) -> Self {
        self.value = Some(value.into());
        self
    }

    pub fn hidden(mut self, hidden: bool) -> Self {
        self.hidden = hidden;
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    pub fn checked(mut self, checked: bool) -> Self {
        self.checked = Some(checked);
        self
    }

    pub fn action(mut self, action: AccessibilityAction) -> Self {
        if !self.actions.contains(&action) {
            self.actions.push(action);
        }
        self
    }

    pub fn has_metadata(&self) -> bool {
        self.role.is_some()
            || self.label.is_some()
            || self.description.is_some()
            || self.value.is_some()
            || self.hidden
            || self.disabled
            || self.checked.is_some()
            || !self.actions.is_empty()
    }
}
