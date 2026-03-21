//! DOM renderer for wasm32 targets.

#![cfg(target_arch = "wasm32")]

use crate::web_text_metrics::normalize_dom_font_family;
use spark_core::{Color, Point, Rect};
use spark_render::{DrawCommand, DrawList};
use wasm_bindgen::JsCast;
use web_sys::{Document, HtmlElement};

/// Renders draw commands into a retained DOM layer.
pub struct DomRenderer {
    root: HtmlElement,
    pool: Vec<HtmlElement>,
}

impl DomRenderer {
    /// Create and mount the root DOM layer under `document.body`.
    pub fn mount_to_body(document: &Document) -> Result<Self, wasm_bindgen::JsValue> {
        let body = document
            .body()
            .ok_or_else(|| wasm_bindgen::JsValue::from_str("document.body is missing"))?;
        let root = document.create_element("div")?.dyn_into::<HtmlElement>()?;
        root.set_class_name("spark-dom-root");
        set_style(&root, "position", "relative")?;
        set_style(&root, "width", "100vw")?;
        set_style(&root, "height", "100vh")?;
        set_style(&root, "overflow", "hidden")?;
        set_style(&root, "user-select", "none")?;
        set_style(&root, "outline", "none")?;
        root.set_tab_index(0);
        body.append_child(&root)?;
        Ok(Self {
            root,
            pool: Vec::new(),
        })
    }

    pub fn root(&self) -> &HtmlElement {
        &self.root
    }

    /// Render a draw list into the retained DOM tree.
    pub fn render(
        &mut self,
        draw_list: &DrawList,
        background: Color,
    ) -> Result<(), wasm_bindgen::JsValue> {
        set_style(&self.root, "background-color", &color_to_css(background))?;

        let mut clip_stack: Vec<Rect> = Vec::new();
        let mut translation_stack: Vec<(f32, f32)> = vec![(0.0, 0.0)];
        let mut active = 0usize;

        for command in draw_list.commands() {
            match command {
                DrawCommand::Rect {
                    bounds,
                    color,
                    corner_radius,
                    border_width,
                    border_color,
                } => {
                    let translation = translation_stack.last().copied().unwrap_or((0.0, 0.0));
                    let translated_bounds = Rect::new(
                        bounds.x + translation.0,
                        bounds.y + translation.1,
                        bounds.width,
                        bounds.height,
                    );
                    let clipped_bounds = if let Some(clip) = clip_stack.last() {
                        match translated_bounds.intersection(clip) {
                            Some(b) => b,
                            None => continue,
                        }
                    } else {
                        translated_bounds
                    };

                    let node = self.ensure_node(active)?;
                    active += 1;
                    set_style(&node, "display", "block")?;
                    set_style(&node, "position", "absolute")?;
                    set_style(&node, "pointer-events", "none")?;
                    set_style(&node, "left", &px(clipped_bounds.x))?;
                    set_style(&node, "top", &px(clipped_bounds.y))?;
                    set_style(&node, "width", &px(clipped_bounds.width))?;
                    set_style(&node, "height", &px(clipped_bounds.height))?;
                    set_style(&node, "background-color", &color_to_css(*color))?;
                    set_style(&node, "border-radius", &px(*corner_radius))?;
                    if *border_width > 0.0 {
                        set_style(&node, "border-style", "solid")?;
                        set_style(&node, "border-width", &px(*border_width))?;
                        set_style(&node, "border-color", &color_to_css(*border_color))?;
                    } else {
                        set_style(&node, "border-width", "0px")?;
                        set_style(&node, "border-style", "none")?;
                    }
                    node.set_text_content(None);
                }
                DrawCommand::TextRun { run } => {
                    let translation = translation_stack.last().copied().unwrap_or((0.0, 0.0));
                    let x = run.position.0 + translation.0;
                    let y = run.position.1 + translation.1;
                    if let Some(clip) = clip_stack.last() {
                        if !clip.contains(Point::new(x, y)) {
                            continue;
                        }
                    }

                    let node = self.ensure_node(active)?;
                    active += 1;
                    set_style(&node, "display", "block")?;
                    set_style(&node, "position", "absolute")?;
                    set_style(&node, "pointer-events", "none")?;
                    set_style(&node, "left", &px(x))?;
                    set_style(&node, "top", &px(y))?;
                    set_style(&node, "color", &color_to_css(run.style.color))?;
                    set_style(&node, "font-size", &px(run.style.font_size))?;
                    // Keep DOM text metrics aligned with wasm text measurement, which currently
                    // uses a generic sans-serif stack on the shaping side.
                    let family = normalize_dom_font_family(&run.style.family);
                    set_style(&node, "font-family", family)?;
                    set_style(
                        &node,
                        "font-style",
                        if run.style.italic { "italic" } else { "normal" },
                    )?;
                    set_style(
                        &node,
                        "font-weight",
                        if run.style.bold { "700" } else { "400" },
                    )?;
                    set_style(&node, "line-height", &run.style.line_height.to_string())?;
                    set_style(&node, "white-space", "pre")?;
                    set_style(&node, "background-color", "transparent")?;
                    set_style(&node, "border", "none")?;
                    node.set_text_content(Some(&run.text));
                }
                DrawCommand::Text { .. } => {
                    // Legacy command path is intentionally ignored by the DOM renderer.
                }
                DrawCommand::PushClip { bounds } => {
                    let translation = translation_stack.last().copied().unwrap_or((0.0, 0.0));
                    let translated_bounds = Rect::new(
                        bounds.x + translation.0,
                        bounds.y + translation.1,
                        bounds.width,
                        bounds.height,
                    );
                    let new_clip = if let Some(current) = clip_stack.last() {
                        translated_bounds
                            .intersection(current)
                            .unwrap_or(Rect::ZERO)
                    } else {
                        translated_bounds
                    };
                    clip_stack.push(new_clip);
                }
                DrawCommand::PopClip => {
                    clip_stack.pop();
                }
                DrawCommand::PushTranslation { offset } => {
                    let current = translation_stack.last().copied().unwrap_or((0.0, 0.0));
                    translation_stack.push((current.0 + offset.0, current.1 + offset.1));
                }
                DrawCommand::PopTranslation => {
                    if translation_stack.len() > 1 {
                        translation_stack.pop();
                    }
                }
            }
        }

        for idx in active..self.pool.len() {
            set_style(&self.pool[idx], "display", "none")?;
        }

        Ok(())
    }

    fn ensure_node(&mut self, index: usize) -> Result<HtmlElement, wasm_bindgen::JsValue> {
        if let Some(existing) = self.pool.get(index) {
            return Ok(existing.clone());
        }
        let document = self
            .root
            .owner_document()
            .ok_or_else(|| wasm_bindgen::JsValue::from_str("missing owner document"))?;
        let node = document.create_element("div")?.dyn_into::<HtmlElement>()?;
        self.root.append_child(&node)?;
        self.pool.push(node.clone());
        Ok(node)
    }
}

fn set_style(node: &HtmlElement, key: &str, value: &str) -> Result<(), wasm_bindgen::JsValue> {
    node.style().set_property(key, value)
}

fn px(value: f32) -> String {
    format!("{value}px")
}

fn color_to_css(color: Color) -> String {
    let r = (linear_to_srgb(color.r).clamp(0.0, 1.0) * 255.0).round() as u8;
    let g = (linear_to_srgb(color.g).clamp(0.0, 1.0) * 255.0).round() as u8;
    let b = (linear_to_srgb(color.b).clamp(0.0, 1.0) * 255.0).round() as u8;
    format!("rgba({r}, {g}, {b}, {})", color.a.clamp(0.0, 1.0))
}

fn linear_to_srgb(channel: f32) -> f32 {
    if channel <= 0.003_130_8 {
        channel * 12.92
    } else {
        1.055 * channel.powf(1.0 / 2.4) - 0.055
    }
}
