//! DOM renderer for wasm32 targets.

#![cfg(target_arch = "wasm32")]

use crate::web_text_metrics::normalize_dom_font_family;
use spark_core::{Color, Point, Rect};
use spark_render::{DrawCommand, DrawList};
use std::collections::HashMap;
use wasm_bindgen::JsCast;
use web_sys::{Document, HtmlElement};

/// Renders draw commands into a retained DOM layer.
pub struct DomRenderer {
    root: HtmlElement,
    pool: Vec<HtmlElement>,
    states: Vec<NodeState>,
    active_nodes: usize,
    mutated_nodes: usize,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum NodeKind {
    #[default]
    Unknown,
    Rect,
    Line,
    Text,
}

#[derive(Default)]
struct NodeState {
    kind: NodeKind,
    styles: HashMap<&'static str, String>,
    text: Option<String>,
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
            states: Vec::new(),
            active_nodes: 0,
            mutated_nodes: 0,
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
        self.mutated_nodes = 0;

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

                    let _node = self.prepare_node(active, NodeKind::Rect)?;
                    active += 1;
                    self.set_style_cached(active - 1, "display", "block")?;
                    self.set_style_cached(active - 1, "position", "absolute")?;
                    self.set_style_cached(active - 1, "pointer-events", "none")?;
                    self.set_style_cached(active - 1, "transform", "none")?;
                    self.set_style_cached(active - 1, "transform-origin", "center center")?;
                    self.set_style_cached(active - 1, "left", px(clipped_bounds.x))?;
                    self.set_style_cached(active - 1, "top", px(clipped_bounds.y))?;
                    self.set_style_cached(active - 1, "width", px(clipped_bounds.width))?;
                    self.set_style_cached(active - 1, "height", px(clipped_bounds.height))?;
                    self.set_style_cached(active - 1, "background-color", color_to_css(*color))?;
                    self.set_style_cached(active - 1, "border-radius", px(*corner_radius))?;
                    if *border_width > 0.0 {
                        self.set_style_cached(active - 1, "border-style", "solid")?;
                        self.set_style_cached(active - 1, "border-width", px(*border_width))?;
                        self.set_style_cached(active - 1, "border-color", color_to_css(*border_color))?;
                    } else {
                        self.set_style_cached(active - 1, "border-width", "0px")?;
                        self.set_style_cached(active - 1, "border-style", "none")?;
                        self.set_style_cached(active - 1, "border-color", "transparent")?;
                    }
                    self.set_text_cached(active - 1, None)?;
                }
                DrawCommand::Line {
                    start,
                    end,
                    thickness,
                    color,
                } => {
                    let translation = translation_stack.last().copied().unwrap_or((0.0, 0.0));
                    let start = Point::new(start.0 + translation.0, start.1 + translation.1);
                    let end = Point::new(end.0 + translation.0, end.1 + translation.1);
                    let line_bounds = line_bounding_box(start, end, *thickness);
                    if let Some(clip) = clip_stack.last() {
                        if line_bounds.intersection(clip).is_none() {
                            continue;
                        }
                    }

                    let delta = end - start;
                    let length = delta.length();
                    if length <= f32::EPSILON {
                        continue;
                    }
                    let center = (start + end) * 0.5;
                    let angle = delta.y.atan2(delta.x);
                    let _node = self.prepare_node(active, NodeKind::Line)?;
                    active += 1;
                    self.set_style_cached(active - 1, "display", "block")?;
                    self.set_style_cached(active - 1, "position", "absolute")?;
                    self.set_style_cached(active - 1, "pointer-events", "none")?;
                    self.set_style_cached(active - 1, "left", px(center.x - length * 0.5))?;
                    self.set_style_cached(active - 1, "top", px(center.y - *thickness * 0.5))?;
                    self.set_style_cached(active - 1, "width", px(length))?;
                    self.set_style_cached(active - 1, "height", px(*thickness))?;
                    self.set_style_cached(active - 1, "background-color", color_to_css(*color))?;
                    self.set_style_cached(active - 1, "border-radius", px(*thickness * 0.5))?;
                    self.set_style_cached(active - 1, "border-width", "0px")?;
                    self.set_style_cached(active - 1, "border-style", "none")?;
                    self.set_style_cached(active - 1, "border-color", "transparent")?;
                    self.set_style_cached(active - 1, "transform-origin", "center center")?;
                    self.set_style_cached(active - 1, "transform", format!("rotate({angle}rad)"))?;
                    self.set_text_cached(active - 1, None)?;
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

                    let _node = self.prepare_node(active, NodeKind::Text)?;
                    active += 1;
                    self.set_style_cached(active - 1, "display", "block")?;
                    self.set_style_cached(active - 1, "position", "absolute")?;
                    self.set_style_cached(active - 1, "pointer-events", "none")?;
                    self.set_style_cached(active - 1, "left", px(x))?;
                    self.set_style_cached(active - 1, "top", px(y))?;
                    self.set_style_cached(active - 1, "color", color_to_css(run.style.color))?;
                    self.set_style_cached(active - 1, "font-size", px(run.style.font_size))?;
                    // Keep DOM text metrics aligned with wasm text measurement, which currently
                    // uses a generic sans-serif stack on the shaping side.
                    let family = normalize_dom_font_family(&run.style.family);
                    self.set_style_cached(active - 1, "font-family", family)?;
                    self.set_style_cached(
                        active - 1,
                        "font-style",
                        if run.style.italic { "italic" } else { "normal" },
                    )?;
                    self.set_style_cached(
                        active - 1,
                        "font-weight",
                        if run.style.bold { "700" } else { "400" },
                    )?;
                    self.set_style_cached(active - 1, "line-height", run.style.line_height.to_string())?;
                    self.set_style_cached(active - 1, "white-space", "pre")?;
                    self.set_style_cached(active - 1, "background-color", "transparent")?;
                    self.set_style_cached(active - 1, "border", "none")?;
                    self.set_style_cached(active - 1, "transform", "none")?;
                    self.set_text_cached(active - 1, Some(run.text.clone()))?;
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
            self.set_style_cached(idx, "display", "none")?;
        }

        self.active_nodes = active;

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
        self.states.push(NodeState::default());
        Ok(node)
    }

    fn prepare_node(
        &mut self,
        index: usize,
        kind: NodeKind,
    ) -> Result<HtmlElement, wasm_bindgen::JsValue> {
        let node = self.ensure_node(index)?;
        let state = &mut self.states[index];
        if state.kind != kind {
            state.kind = kind;
            state.styles.clear();
            if state.text.is_some() {
                node.set_text_content(None);
                state.text = None;
                self.mutated_nodes += 1;
            }
        }
        Ok(node)
    }

    fn set_style_cached(
        &mut self,
        index: usize,
        key: &'static str,
        value: impl Into<String>,
    ) -> Result<(), wasm_bindgen::JsValue> {
        let value = value.into();
        let state = &mut self.states[index];
        if state.styles.get(key).map(String::as_str) == Some(value.as_str()) {
            return Ok(());
        }
        set_style(&self.pool[index], key, &value)?;
        state.styles.insert(key, value);
        self.mutated_nodes += 1;
        Ok(())
    }

    fn set_text_cached(
        &mut self,
        index: usize,
        value: Option<String>,
    ) -> Result<(), wasm_bindgen::JsValue> {
        if self.states[index].text == value {
            return Ok(());
        }
        self.pool[index].set_text_content(value.as_deref());
        self.states[index].text = value;
        self.mutated_nodes += 1;
        Ok(())
    }

    pub fn active_node_count(&self) -> usize {
        self.active_nodes
    }

    pub fn mutated_node_count(&self) -> usize {
        self.mutated_nodes
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

fn line_bounding_box(start: Point, end: Point, thickness: f32) -> Rect {
    let half = thickness * 0.5;
    let min_x = start.x.min(end.x) - half;
    let min_y = start.y.min(end.y) - half;
    let max_x = start.x.max(end.x) + half;
    let max_y = start.y.max(end.y) + half;
    Rect::new(min_x, min_y, max_x - min_x, max_y - min_y)
}
