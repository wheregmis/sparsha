use crate::system::TextStyle;

pub(crate) trait TextMetricsBackend {
    fn measure_inline(&self, text: &str, style: &TextStyle) -> Option<(f32, f32)>;
}

pub(crate) fn default_text_metrics_backend() -> Box<dyn TextMetricsBackend> {
    #[cfg(target_arch = "wasm32")]
    {
        Box::new(WebDomTextMetricsBackend)
    }

    #[cfg(not(target_arch = "wasm32"))]
    {
        Box::new(NoopTextMetricsBackend)
    }
}

#[cfg_attr(target_arch = "wasm32", allow(dead_code))]
struct NoopTextMetricsBackend;

impl TextMetricsBackend for NoopTextMetricsBackend {
    fn measure_inline(&self, _text: &str, _style: &TextStyle) -> Option<(f32, f32)> {
        None
    }
}

#[cfg(target_arch = "wasm32")]
struct WebDomTextMetricsBackend;

#[cfg(target_arch = "wasm32")]
impl TextMetricsBackend for WebDomTextMetricsBackend {
    fn measure_inline(&self, text: &str, style: &TextStyle) -> Option<(f32, f32)> {
        use std::cell::RefCell;

        thread_local! {
            static TEXT_METRICS_SPAN: RefCell<Option<web_sys::Element>> = const { RefCell::new(None) };
        }

        let document = web_sys::window()?.document()?;
        let span = TEXT_METRICS_SPAN.with(|slot| {
            if let Some(existing) = slot.borrow().as_ref() {
                return Some(existing.clone());
            }

            let body = document.body()?;
            let node = document.create_element("span").ok()?;
            body.append_child(&node).ok()?;
            *slot.borrow_mut() = Some(node.clone());
            Some(node)
        })?;

        let family = if style.family.trim().is_empty() || style.family == "system-ui" {
            "sans-serif"
        } else {
            style.family.as_str()
        };

        let css = format!(
            "position:absolute;visibility:hidden;white-space:pre;left:-100000px;top:-100000px;\
             pointer-events:none;font-size:{}px;font-family:{};font-style:{};font-weight:{};\
             line-height:{};",
            style.font_size,
            family,
            if style.italic { "italic" } else { "normal" },
            if style.bold { "700" } else { "400" },
            style.line_height
        );
        span.set_attribute("style", &css).ok()?;
        span.set_text_content(Some(text));
        let rect = span.get_bounding_client_rect();
        Some((rect.width() as f32, rect.height() as f32))
    }
}
