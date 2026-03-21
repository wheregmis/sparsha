//! Shared helpers for keeping web DOM text metrics aligned with text measurement.

/// Normalize the CSS `font-family` used for DOM text rendering so it stays compatible with the
/// wasm text measurement path.
pub(crate) fn normalize_dom_font_family(family: &str) -> &str {
    if family.trim().is_empty() || family == "system-ui" {
        "sans-serif"
    } else {
        family
    }
}

#[cfg(test)]
mod tests {
    use super::normalize_dom_font_family;

    #[test]
    fn normalizes_system_ui_to_sans_serif() {
        assert_eq!(normalize_dom_font_family("system-ui"), "sans-serif");
    }

    #[test]
    fn normalizes_empty_family_to_sans_serif() {
        assert_eq!(normalize_dom_font_family(""), "sans-serif");
        assert_eq!(normalize_dom_font_family("   "), "sans-serif");
    }

    #[test]
    fn keeps_explicit_family_intact() {
        assert_eq!(normalize_dom_font_family("Inter"), "Inter");
        assert_eq!(
            normalize_dom_font_family("ui-sans-serif, system-ui"),
            "ui-sans-serif, system-ui"
        );
    }
}
