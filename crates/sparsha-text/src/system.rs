//! Text shaping and layout system using Parley.

use crate::atlas::{CachedGlyph, GlyphAtlas, GlyphBitmap, GlyphKey};
use crate::metrics_backend::{default_text_metrics_backend, TextMetricsBackend};
#[cfg(not(target_arch = "wasm32"))]
use parley::fontique::Blob;
use parley::{
    layout::{Alignment, GlyphRun, PositionedLayoutItem},
    style::{
        FontFamily, FontStack, FontStyle, FontWeight, GenericFamily, LineHeight, OverflowWrap,
        StyleProperty, TextWrapMode, WordBreakStrength,
    },
    FontContext, Layout, LayoutContext,
};
use sparsha_core::{Color, GlyphInstance};
use std::collections::HashMap;
use std::ops::Range;
use swash::{
    scale::{Render, ScaleContext, Source, StrikeWith},
    zeno::{Format, Vector},
    FontRef,
};
use wgpu::{Device, Queue};

/// Horizontal alignment for constrained text layout.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum TextLayoutAlignment {
    #[default]
    Start,
    Center,
    End,
}

impl From<TextLayoutAlignment> for Alignment {
    fn from(value: TextLayoutAlignment) -> Self {
        match value {
            TextLayoutAlignment::Start => Alignment::Start,
            TextLayoutAlignment::Center => Alignment::Center,
            TextLayoutAlignment::End => Alignment::End,
        }
    }
}

/// Wrapping behavior for constrained text layout.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum TextWrap {
    NoWrap,
    #[default]
    Word,
    Anywhere,
}

/// Additional word-breaking policy for constrained text layout.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum TextBreakMode {
    #[default]
    Normal,
    BreakWord,
    BreakAll,
}

/// Layout options for shaped or measured text.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct TextLayoutOptions {
    pub max_width: Option<f32>,
    pub alignment: TextLayoutAlignment,
    pub max_lines: Option<usize>,
    pub wrap: TextWrap,
    pub break_mode: TextBreakMode,
}

/// A single visual line in a constrained text layout.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct TextLayoutLine {
    pub text_range: Range<usize>,
    pub offset: f32,
    pub baseline: f32,
    pub advance: f32,
    pub line_height: f32,
    pub min_coord: f32,
    pub max_coord: f32,
}

/// Paragraph layout information for constrained text.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct TextLayoutInfo {
    pub width: f32,
    pub height: f32,
    pub lines: Vec<TextLayoutLine>,
}

impl TextLayoutOptions {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_max_width(mut self, max_width: Option<f32>) -> Self {
        self.max_width = max_width;
        self
    }

    pub fn with_alignment(mut self, alignment: TextLayoutAlignment) -> Self {
        self.alignment = alignment;
        self
    }

    pub fn with_max_lines(mut self, max_lines: Option<usize>) -> Self {
        self.max_lines = max_lines;
        self
    }

    pub fn with_wrap(mut self, wrap: TextWrap) -> Self {
        self.wrap = wrap;
        self
    }

    pub fn with_break_mode(mut self, break_mode: TextBreakMode) -> Self {
        self.break_mode = break_mode;
        self
    }
}

// Embed Inter on native platforms only to keep web WASM payload smaller.
#[cfg(not(target_arch = "wasm32"))]
static INTER_REGULAR: &[u8] = include_bytes!("../../../assets/fonts/Inter-Regular.ttf");
#[cfg(not(target_arch = "wasm32"))]
static INTER_BOLD: &[u8] = include_bytes!("../../../assets/fonts/Inter-Bold.ttf");

/// Text style configuration.
#[derive(Clone, Debug)]
pub struct TextStyle {
    /// Font family name.
    pub family: String,
    /// Font size in pixels.
    pub font_size: f32,
    /// Line height multiplier.
    pub line_height: f32,
    /// Text color.
    pub color: Color,
    /// Whether the text is bold.
    pub bold: bool,
    /// Whether the text is italic.
    pub italic: bool,
}

impl Default for TextStyle {
    fn default() -> Self {
        Self {
            family: String::from("system-ui"),
            font_size: 16.0,
            line_height: 1.2,
            color: Color::BLACK,
            bold: false,
            italic: false,
        }
    }
}

impl TextStyle {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_size(mut self, size: f32) -> Self {
        self.font_size = size;
        self
    }

    pub fn with_color(mut self, color: Color) -> Self {
        self.color = color;
        self
    }

    pub fn with_line_height(mut self, line_height: f32) -> Self {
        self.line_height = line_height;
        self
    }

    pub fn with_family(mut self, family: impl Into<String>) -> Self {
        self.family = family.into();
        self
    }

    pub fn bold(mut self) -> Self {
        self.bold = true;
        self
    }

    pub fn italic(mut self) -> Self {
        self.italic = true;
        self
    }
}

/// Result of text shaping - positioned glyphs ready for rendering.
#[derive(Clone, Debug, Default)]
pub struct ShapedText {
    /// Glyph instances ready for GPU rendering.
    pub glyphs: Vec<GlyphInstance>,
    /// Total width of the shaped text.
    pub width: f32,
    /// Total height of the shaped text.
    pub height: f32,
}

impl ShapedText {
    /// Check if the shaped text has any glyphs.
    pub fn is_empty(&self) -> bool {
        self.glyphs.is_empty()
    }
}

/// The text system manages fonts, shaping, and glyph caching.
pub struct TextSystem {
    font_cx: FontContext,
    layout_cx: LayoutContext<[u8; 4]>,
    scale_cx: ScaleContext,
    atlas: Option<GlyphAtlas>,
    measure_cache: HashMap<TextCacheKey, (f32, f32)>,
    layout_cache: HashMap<TextCacheKey, TextLayoutInfo>,
    shape_cache: HashMap<TextCacheKey, ShapedText>,
    metrics_backend: Box<dyn TextMetricsBackend>,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
struct TextCacheKey {
    text: String,
    family: String,
    font_size_bits: u32,
    line_height_bits: u32,
    color_bits: [u32; 4],
    bold: bool,
    italic: bool,
    max_width_bits: Option<u32>,
    alignment: TextLayoutAlignment,
    max_lines: Option<usize>,
    wrap: TextWrap,
    break_mode: TextBreakMode,
}

impl TextCacheKey {
    fn new(text: &str, style: &TextStyle, options: TextLayoutOptions) -> Self {
        Self {
            text: text.to_owned(),
            family: style.family.clone(),
            font_size_bits: style.font_size.to_bits(),
            line_height_bits: style.line_height.to_bits(),
            color_bits: [
                style.color.r.to_bits(),
                style.color.g.to_bits(),
                style.color.b.to_bits(),
                style.color.a.to_bits(),
            ],
            bold: style.bold,
            italic: style.italic,
            max_width_bits: options.max_width.map(f32::to_bits),
            alignment: options.alignment,
            max_lines: options.max_lines,
            wrap: options.wrap,
            break_mode: options.break_mode,
        }
    }
}

impl TextSystem {
    /// Create a new text system.
    pub fn new(device: &Device) -> Self {
        let mut system = Self::new_headless();
        system.ensure_atlas(device);
        system
    }

    /// Create a text system that can measure text without initializing GPU resources.
    pub fn new_headless() -> Self {
        #[cfg(not(target_arch = "wasm32"))]
        let mut font_cx = FontContext::new();
        #[cfg(target_arch = "wasm32")]
        let font_cx = FontContext::new();

        #[cfg(not(target_arch = "wasm32"))]
        {
            let regular_blob = Blob::new(std::sync::Arc::new(INTER_REGULAR.to_vec()));
            let bold_blob = Blob::new(std::sync::Arc::new(INTER_BOLD.to_vec()));
            font_cx.collection.register_fonts(regular_blob, None);
            font_cx.collection.register_fonts(bold_blob, None);
        }

        let layout_cx = LayoutContext::new();
        let scale_cx = ScaleContext::new();

        Self {
            font_cx,
            layout_cx,
            scale_cx,
            atlas: None,
            measure_cache: HashMap::new(),
            layout_cache: HashMap::new(),
            shape_cache: HashMap::new(),
            metrics_backend: default_text_metrics_backend(),
        }
    }

    /// Get a reference to the font context.
    pub fn font_context(&self) -> &FontContext {
        &self.font_cx
    }

    /// Get a mutable reference to the font context.
    pub fn font_context_mut(&mut self) -> &mut FontContext {
        &mut self.font_cx
    }

    /// Get the glyph atlas.
    pub fn atlas(&self) -> Option<&GlyphAtlas> {
        self.atlas.as_ref()
    }

    fn ensure_atlas(&mut self, device: &Device) {
        if self.atlas.is_none() {
            self.atlas = Some(GlyphAtlas::new(device, 1024, 1024));
            self.shape_cache.clear();
        }
    }

    fn build_layout(
        &mut self,
        text: &str,
        style: &TextStyle,
        options: TextLayoutOptions,
    ) -> Layout<[u8; 4]> {
        let mut builder = self
            .layout_cx
            .ranged_builder(&mut self.font_cx, text, 1.0, true);

        builder.push_default(StyleProperty::FontSize(style.font_size));
        builder.push_default(StyleProperty::LineHeight(LineHeight::FontSizeRelative(
            style.line_height,
        )));

        #[cfg(target_arch = "wasm32")]
        builder.push_default(StyleProperty::FontStack(FontStack::List(
            vec![FontFamily::Generic(GenericFamily::SansSerif)].into(),
        )));

        #[cfg(not(target_arch = "wasm32"))]
        builder.push_default(StyleProperty::FontStack(FontStack::List(
            vec![
                FontFamily::Named("Inter".into()),
                FontFamily::Generic(GenericFamily::SansSerif),
            ]
            .into(),
        )));

        let (text_wrap_mode, mut overflow_wrap) = match options.wrap {
            TextWrap::NoWrap => (TextWrapMode::NoWrap, OverflowWrap::Normal),
            TextWrap::Word => (TextWrapMode::Wrap, OverflowWrap::Normal),
            TextWrap::Anywhere => (TextWrapMode::Wrap, OverflowWrap::Anywhere),
        };
        let word_break = match options.break_mode {
            TextBreakMode::Normal => WordBreakStrength::Normal,
            TextBreakMode::BreakWord => {
                if options.wrap != TextWrap::NoWrap {
                    overflow_wrap = OverflowWrap::BreakWord;
                }
                WordBreakStrength::Normal
            }
            TextBreakMode::BreakAll => WordBreakStrength::BreakAll,
        };

        builder.push_default(StyleProperty::TextWrapMode(text_wrap_mode));
        builder.push_default(StyleProperty::OverflowWrap(overflow_wrap));
        builder.push_default(StyleProperty::WordBreak(word_break));

        if style.bold {
            builder.push_default(StyleProperty::FontWeight(FontWeight::BOLD));
        }
        if style.italic {
            builder.push_default(StyleProperty::FontStyle(FontStyle::Italic));
        }

        let color_arr = style.color.to_u8_array();
        builder.push_default(StyleProperty::Brush(color_arr));

        let mut layout: Layout<[u8; 4]> = builder.build(text);
        layout.break_all_lines(options.max_width);
        layout.align(
            options.max_width,
            options.alignment.into(),
            parley::layout::AlignmentOptions {
                align_when_overflowing: options.alignment != TextLayoutAlignment::Start,
            },
        );
        layout
    }

    /// Compute paragraph layout information using explicit width/alignment/clamp options.
    pub fn layout_info(
        &mut self,
        text: &str,
        style: &TextStyle,
        options: TextLayoutOptions,
    ) -> TextLayoutInfo {
        if text.is_empty() {
            return TextLayoutInfo::default();
        }

        let cache_key = TextCacheKey::new(text, style, options);
        if let Some(cached) = self.layout_cache.get(&cache_key) {
            return cached.clone();
        }

        let layout = self.build_layout(text, style, options);
        let info = TextLayoutInfo {
            width: layout_width(&layout, options.max_width, options.max_lines),
            height: layout_height(&layout, style, options.max_lines),
            lines: layout
                .lines()
                .enumerate()
                .take_while(|(line_index, _)| {
                    options.max_lines.is_none_or(|limit| *line_index < limit)
                })
                .map(|(_, line)| {
                    let metrics = line.metrics();
                    TextLayoutLine {
                        text_range: line.text_range(),
                        offset: metrics.offset,
                        baseline: metrics.baseline,
                        advance: metrics.advance,
                        line_height: metrics.line_height,
                        min_coord: metrics.min_coord,
                        max_coord: metrics.max_coord,
                    }
                })
                .collect(),
        };

        cache_insert(&mut self.layout_cache, cache_key, info.clone(), 512);
        info
    }

    /// Shape and position text for rendering.
    pub fn shape(
        &mut self,
        device: &Device,
        queue: &Queue,
        text: &str,
        style: &TextStyle,
        max_width: Option<f32>,
    ) -> ShapedText {
        self.shape_with_options(
            device,
            queue,
            text,
            style,
            TextLayoutOptions::new().with_max_width(max_width),
        )
    }

    /// Shape and position text for rendering with explicit layout options.
    pub fn shape_with_options(
        &mut self,
        device: &Device,
        queue: &Queue,
        text: &str,
        style: &TextStyle,
        options: TextLayoutOptions,
    ) -> ShapedText {
        if text.is_empty() {
            return ShapedText::default();
        }
        let cache_key = TextCacheKey::new(text, style, options);
        if let Some(cached) = self.shape_cache.get(&cache_key) {
            return cached.clone();
        }
        self.ensure_atlas(device);

        let layout = self.build_layout(text, style, options);

        // Collect glyph instances
        let mut glyphs = Vec::new();
        let mut min_y: f32 = f32::MAX;
        let mut max_y: f32 = f32::MIN;

        for (line_index, line) in layout.lines().enumerate() {
            if options.max_lines.is_some_and(|limit| line_index >= limit) {
                break;
            }
            for item in line.items() {
                if let PositionedLayoutItem::GlyphRun(glyph_run) = item {
                    let mut reset_atlas = false;
                    self.render_glyph_run(
                        queue,
                        &glyph_run,
                        &mut glyphs,
                        &mut min_y,
                        &mut max_y,
                        &mut reset_atlas,
                    );
                    if reset_atlas {
                        self.atlas = Some(GlyphAtlas::new(device, 2048, 2048));
                        self.shape_cache.clear();
                    }
                }
            }
        }

        // Normalize Y positions so all glyphs start at y >= 0
        if min_y < f32::MAX && min_y != 0.0 {
            let offset = -min_y.min(0.0);
            for glyph in &mut glyphs {
                glyph.pos[1] += offset;
            }
            if max_y > f32::MIN {
                max_y += offset;
            }
        }

        let total_height = if glyphs.is_empty() {
            layout_height(&layout, style, options.max_lines)
        } else if max_y > f32::MIN {
            max_y
        } else {
            layout_height(&layout, style, options.max_lines)
        };

        let shaped = ShapedText {
            glyphs,
            width: layout_width(&layout, options.max_width, options.max_lines),
            height: total_height,
        };
        cache_insert(&mut self.shape_cache, cache_key, shaped.clone(), 512);
        shaped
    }

    fn render_glyph_run(
        &mut self,
        queue: &Queue,
        glyph_run: &GlyphRun<'_, [u8; 4]>,
        glyphs: &mut Vec<GlyphInstance>,
        min_y: &mut f32,
        max_y: &mut f32,
        reset_atlas: &mut bool,
    ) {
        let run = glyph_run.run();
        let font = run.font();
        let font_size = run.font_size();

        // Convert brush color from [u8; 4] back to [f32; 4] for GlyphInstance
        let brush = glyph_run.style().brush;
        let color = [
            brush[0] as f32 / 255.0,
            brush[1] as f32 / 255.0,
            brush[2] as f32 / 255.0,
            brush[3] as f32 / 255.0,
        ];
        let run_x = glyph_run.offset();
        let run_y = glyph_run.baseline();

        // Get font data for swash
        let font_data = font.data.as_ref();
        let font_ref = match FontRef::from_index(font_data, font.index as usize) {
            Some(f) => f,
            None => return,
        };

        // Create a hash from font data pointer for caching
        let font_hash = font_data.as_ptr() as u64;

        // Get normalized coordinates for variable fonts - convert to swash Setting format
        let normalized_coords = run.normalized_coords();

        // Track cursor position - glyph.x is for kerning adjustments, we need to accumulate advances
        let mut cursor_x = run_x;

        for glyph in glyph_run.glyphs() {
            let glyph_id = glyph.id;
            // glyph.x contains kerning/positioning adjustments, add to cursor
            let x = cursor_x + glyph.x;
            let y = run_y - glyph.y;

            // Create glyph key for caching
            let key = GlyphKey::new(font_hash, glyph_id, font_size);

            let Some(atlas) = self.atlas.as_mut() else {
                log::warn!("glyph atlas missing during shaping; skipping glyph run");
                return;
            };
            let cached = if let Some(cached) = atlas.get(&key) {
                *cached
            } else {
                let glyph_id_u16 = match u16::try_from(glyph_id) {
                    Ok(id) => id,
                    Err(_) => {
                        cursor_x += glyph.advance;
                        continue;
                    }
                };
                // Rasterize the glyph using swash
                let mut scaler = self
                    .scale_cx
                    .builder(font_ref)
                    .size(font_size)
                    .hint(true)
                    .normalized_coords(normalized_coords)
                    .build();

                let image = Render::new(&[
                    Source::ColorOutline(0),
                    Source::ColorBitmap(StrikeWith::BestFit),
                    Source::Outline,
                ])
                .format(Format::Alpha)
                .offset(Vector::new(0.0, 0.0))
                .render(&mut scaler, glyph_id_u16);

                match image {
                    Some(img) => {
                        let cached = atlas.insert(
                            queue,
                            key,
                            GlyphBitmap {
                                width: img.placement.width,
                                height: img.placement.height,
                                offset_x: img.placement.left,
                                offset_y: img.placement.top,
                                data: &img.data,
                            },
                        );

                        match cached {
                            Some(c) => c,
                            None => {
                                // Atlas full, clear and retry with larger atlas
                                atlas.clear();
                                *reset_atlas = true;
                                continue;
                            }
                        }
                    }
                    None => {
                        // Create empty glyph for spaces and other non-rendering glyphs
                        CachedGlyph {
                            uv_x: 0.0,
                            uv_y: 0.0,
                            uv_width: 0.0,
                            uv_height: 0.0,
                            width: 0,
                            height: 0,
                            offset_x: 0,
                            offset_y: 0,
                        }
                    }
                }
            };

            // Skip empty glyphs but still advance cursor
            if cached.width == 0 || cached.height == 0 {
                cursor_x += glyph.advance;
                continue;
            }

            let glyph_x = x + cached.offset_x as f32;
            let glyph_y = y - cached.offset_y as f32;

            *min_y = min_y.min(glyph_y);
            *max_y = max_y.max(glyph_y + cached.height as f32);

            glyphs.push(GlyphInstance {
                pos: [glyph_x, glyph_y],
                size: [cached.width as f32, cached.height as f32],
                uv_pos: [cached.uv_x, cached.uv_y],
                uv_size: [cached.uv_width, cached.uv_height],
                color,
            });

            // Advance cursor by glyph width
            cursor_x += glyph.advance;
        }
    }

    /// Measure text without rasterizing (faster for layout).
    /// Returns (width, height) where height is based on line metrics.
    pub fn measure(&mut self, text: &str, style: &TextStyle, max_width: Option<f32>) -> (f32, f32) {
        self.measure_with_options(
            text,
            style,
            TextLayoutOptions::new().with_max_width(max_width),
        )
    }

    /// Measure text without rasterizing using explicit layout options.
    pub fn measure_with_options(
        &mut self,
        text: &str,
        style: &TextStyle,
        options: TextLayoutOptions,
    ) -> (f32, f32) {
        if text.is_empty() {
            return (0.0, style.font_size * style.line_height);
        }
        let cache_key = TextCacheKey::new(text, style, options);
        if let Some(cached) = self.measure_cache.get(&cache_key) {
            return *cached;
        }

        if options.max_width.is_none() {
            if let Some((width, height)) = self.metrics_backend.measure_inline(text, style) {
                let measured = (
                    width.max(0.0),
                    height.max(style.font_size * style.line_height),
                );
                cache_insert(&mut self.measure_cache, cache_key, measured, 1024);
                return measured;
            }
        }

        let layout = self.build_layout(text, style, options);

        let measured_width = layout_width(&layout, options.max_width, options.max_lines);
        let measured_height = layout_height(&layout, style, options.max_lines);

        // On web builds without embedded fonts, the shaping backend can occasionally return
        // zero metrics while browser CSS text still renders. Guard against that so layout
        // does not collapse and overlap adjacent text widgets.
        let fallback_height = style.font_size * style.line_height;
        let height = if measured_height.is_finite() && measured_height > 0.0 {
            measured_height.max(fallback_height)
        } else {
            fallback_height
        };

        let width = if measured_width.is_finite() && measured_width > 0.0 {
            measured_width
        } else {
            let approx = text.chars().count() as f32 * style.font_size * 0.55;
            match options.max_width {
                Some(limit) if limit.is_finite() && limit > 0.0 => approx.min(limit),
                _ => approx,
            }
        };

        let measured = (width, height);
        cache_insert(&mut self.measure_cache, cache_key, measured, 1024);
        measured
    }

    /// Truncate text to the longest prefix that fits the given layout options, appending an
    /// ellipsis when truncation is required.
    pub fn ellipsize_with_options(
        &mut self,
        text: &str,
        style: &TextStyle,
        options: TextLayoutOptions,
    ) -> String {
        if text.is_empty() {
            return String::new();
        }

        let max_lines = options.max_lines.unwrap_or(1).max(1);
        let options = options.with_max_lines(Some(max_lines));

        if text_fits_layout(self, text, style, options) {
            return text.to_owned();
        }

        let ellipsis = "\u{2026}";
        if text_fits_layout(self, ellipsis, style, options) {
            let mut boundaries: Vec<usize> = text.char_indices().map(|(index, _)| index).collect();
            boundaries.push(text.len());

            let mut best = ellipsis.to_owned();
            let mut low = 0usize;
            let mut high = boundaries.len() - 1;

            while low <= high {
                let mid = low + (high - low) / 2;
                let prefix_end = boundaries[mid];
                let candidate = ellipsized_candidate(text, prefix_end);

                if text_fits_layout(self, &candidate, style, options) {
                    best = candidate;
                    low = mid.saturating_add(1);
                } else if mid == 0 {
                    break;
                } else {
                    high = mid - 1;
                }
            }

            return best;
        }

        ellipsis.to_owned()
    }
}

fn text_fits_layout(
    system: &mut TextSystem,
    text: &str,
    style: &TextStyle,
    options: TextLayoutOptions,
) -> bool {
    if text.is_empty() {
        return true;
    }

    let info = system.layout_info(text, style, options);
    let consumed_all_text = info
        .lines
        .last()
        .map(|line| line.text_range.end >= text.len())
        .unwrap_or(false);
    if !consumed_all_text {
        return false;
    }

    match options.max_width {
        Some(limit) if limit.is_finite() && limit > 0.0 => {
            info.lines.iter().all(|line| line.advance <= limit + 1e-3)
        }
        _ => true,
    }
}

fn ellipsized_candidate(text: &str, prefix_end: usize) -> String {
    let trimmed = text[..prefix_end].trim_end();
    if trimmed.is_empty() {
        "\u{2026}".to_owned()
    } else {
        format!("{trimmed}\u{2026}")
    }
}

fn layout_width(
    layout: &Layout<[u8; 4]>,
    max_width_constraint: Option<f32>,
    max_lines: Option<usize>,
) -> f32 {
    let mut widest_line: f32 = 0.0;

    for (line_index, line) in layout.lines().enumerate() {
        if max_lines.is_some_and(|limit| line_index >= limit) {
            break;
        }
        let mut line_width: f32 = 0.0;
        for item in line.items() {
            if let PositionedLayoutItem::GlyphRun(glyph_run) = item {
                let mut cursor_x = glyph_run.offset();
                for glyph in glyph_run.glyphs() {
                    cursor_x += glyph.advance;
                }
                line_width = line_width.max(cursor_x);
            }
        }
        widest_line = widest_line.max(line_width);
    }

    match max_width_constraint {
        Some(limit) if limit.is_finite() && limit > 0.0 => widest_line.min(limit),
        _ => widest_line,
    }
}

fn layout_height(layout: &Layout<[u8; 4]>, style: &TextStyle, max_lines: Option<usize>) -> f32 {
    let fallback_height = style.font_size * style.line_height;
    let limited_height = max_lines
        .and_then(|limit| layout.lines().take(limit).last())
        .map(|line| line.metrics().max_coord)
        .unwrap_or_else(|| layout.height());

    if limited_height.is_finite() && limited_height > 0.0 {
        limited_height.max(fallback_height)
    } else {
        fallback_height
    }
}

fn cache_insert<V: Clone>(
    cache: &mut HashMap<TextCacheKey, V>,
    key: TextCacheKey,
    value: V,
    max: usize,
) {
    if cache.len() >= max {
        cache.clear();
    }
    cache.insert(key, value);
}

#[cfg(test)]
mod tests {
    use super::*;
    use sparsha_core::GlyphInstance;
    use std::time::Instant;

    #[test]
    fn text_style_default() {
        let style = TextStyle::default();
        assert_eq!(style.family, "system-ui");
        assert_eq!(style.font_size, 16.0);
        assert!((style.line_height - 1.2).abs() < 1e-5);
        assert_eq!(style.color, Color::BLACK);
        assert!(!style.bold);
        assert!(!style.italic);
    }

    #[test]
    fn text_style_builder() {
        let style = TextStyle::default()
            .with_size(24.0)
            .with_color(Color::RED)
            .with_family("Inter")
            .bold()
            .italic();
        assert_eq!(style.font_size, 24.0);
        assert_eq!(style.color, Color::RED);
        assert_eq!(style.family, "Inter");
        assert!(style.bold);
        assert!(style.italic);
    }

    #[test]
    fn shaped_text_default_and_is_empty() {
        let st = ShapedText::default();
        assert!(st.glyphs.is_empty());
        assert_eq!(st.width, 0.0);
        assert_eq!(st.height, 0.0);
        assert!(st.is_empty());
    }

    #[test]
    fn shaped_text_with_glyph_not_empty() {
        let mut st = ShapedText::default();
        st.glyphs.push(GlyphInstance {
            pos: [0.0, 0.0],
            size: [10.0, 12.0],
            uv_pos: [0.0, 0.0],
            uv_size: [0.1, 0.1],
            color: [0.0, 0.0, 0.0, 1.0],
        });
        st.width = 10.0;
        st.height = 12.0;
        assert!(!st.is_empty());
        assert_eq!(st.glyphs.len(), 1);
    }

    #[test]
    #[ignore = "perf smoke"]
    fn perf_smoke_measurement_batch() {
        let start = Instant::now();
        let mut system = TextSystem::new_headless();
        let style = TextStyle::default()
            .with_family("Inter")
            .with_size(16.0)
            .with_color(Color::from_hex(0x1F2937));
        let texts: Vec<String> = (0..300)
            .map(|index| {
                format!(
                    "Perf smoke paragraph {index}: Sparsha shapes and measures repeated text for layout verification."
                )
            })
            .collect();
        let mut non_zero = 0usize;

        for round in 0..12 {
            for (index, text) in texts.iter().enumerate() {
                let max_width = Some(240.0 + ((index + round) % 4) as f32 * 80.0);
                let (width, height) = system.measure(text, &style, max_width);
                if width > 0.0 && height > 0.0 {
                    non_zero += 1;
                }
            }
        }

        let elapsed = start.elapsed();
        println!(
            "text perf smoke: measured {} text layouts in {:?}",
            texts.len() * 12,
            elapsed
        );
        assert_eq!(non_zero, texts.len() * 12);
    }

    #[test]
    fn measure_includes_trailing_space_advance() {
        let mut system = TextSystem::new_headless();
        let style = TextStyle::default().with_family("Inter").with_size(16.0);

        let without_space = system.measure("h", &style, None).0;
        let with_trailing_space = system.measure("h ", &style, None).0;

        assert!(
            with_trailing_space > without_space,
            "expected trailing space width to advance the measured width: {without_space} -> {with_trailing_space}"
        );
    }

    #[test]
    fn measure_includes_multiple_trailing_spaces() {
        let mut system = TextSystem::new_headless();
        let style = TextStyle::default().with_family("Inter").with_size(16.0);

        let single_space = system.measure("h ", &style, None).0;
        let double_space = system.measure("h  ", &style, None).0;

        assert!(
            double_space > single_space,
            "expected each trailing space to advance width: {single_space} -> {double_space}"
        );
    }

    #[test]
    fn measure_preserves_trailing_space_on_each_line() {
        let mut system = TextSystem::new_headless();
        let style = TextStyle::default().with_family("Inter").with_size(16.0);

        let without_space = system.measure("hello\nworld", &style, None).0;
        let with_space = system.measure("hello \nworld ", &style, None).0;

        assert!(
            with_space > without_space,
            "expected multiline trailing spaces to affect measured width: {without_space} -> {with_space}"
        );
    }

    #[test]
    fn no_wrap_keeps_constrained_text_on_a_single_line() {
        let mut system = TextSystem::new_headless();
        let style = TextStyle::default().with_family("Inter").with_size(16.0);
        let text = "Sparsha can keep this label on one visual line.";
        let constrained = Some(120.0);

        let (_, wrapped_height) = system.measure_with_options(
            text,
            &style,
            TextLayoutOptions::new()
                .with_max_width(constrained)
                .with_wrap(TextWrap::Word),
        );
        let (_, no_wrap_height) = system.measure_with_options(
            text,
            &style,
            TextLayoutOptions::new()
                .with_max_width(constrained)
                .with_wrap(TextWrap::NoWrap),
        );

        assert!(
            no_wrap_height < wrapped_height,
            "expected no-wrap text to stay on one line under width constraint: {wrapped_height} vs {no_wrap_height}"
        );
    }

    #[test]
    fn anywhere_wrap_breaks_long_words_more_aggressively() {
        let mut system = TextSystem::new_headless();
        let style = TextStyle::default().with_family("Inter").with_size(16.0);
        let text = "Antidisestablishmentarianism";
        let constrained = Some(72.0);

        let (_, word_height) = system.measure_with_options(
            text,
            &style,
            TextLayoutOptions::new()
                .with_max_width(constrained)
                .with_wrap(TextWrap::Word),
        );
        let (_, anywhere_height) = system.measure_with_options(
            text,
            &style,
            TextLayoutOptions::new()
                .with_max_width(constrained)
                .with_wrap(TextWrap::Anywhere),
        );

        assert!(
            anywhere_height > word_height,
            "expected anywhere-wrap to break a long word into more lines: {word_height} vs {anywhere_height}"
        );
    }

    #[test]
    fn break_word_wraps_long_words_more_aggressively_than_normal() {
        let mut system = TextSystem::new_headless();
        let style = TextStyle::default().with_family("Inter").with_size(16.0);
        let text = "very-long-identifier-without-natural-breaks";
        let constrained = Some(96.0);

        let (_, normal_height) = system.measure_with_options(
            text,
            &style,
            TextLayoutOptions::new()
                .with_max_width(constrained)
                .with_wrap(TextWrap::Word)
                .with_break_mode(TextBreakMode::Normal),
        );
        let (_, break_word_height) = system.measure_with_options(
            text,
            &style,
            TextLayoutOptions::new()
                .with_max_width(constrained)
                .with_wrap(TextWrap::Word)
                .with_break_mode(TextBreakMode::BreakWord),
        );

        assert!(
            break_word_height > normal_height,
            "expected break-word to create more wrapped height: {normal_height} vs {break_word_height}"
        );
    }

    #[test]
    fn break_all_wraps_dense_identifiers_more_aggressively() {
        let mut system = TextSystem::new_headless();
        let style = TextStyle::default().with_family("Inter").with_size(16.0);
        let text = "sparshaTextBreakModeDemonstration";
        let constrained = Some(88.0);

        let (_, break_word_height) = system.measure_with_options(
            text,
            &style,
            TextLayoutOptions::new()
                .with_max_width(constrained)
                .with_wrap(TextWrap::Word)
                .with_break_mode(TextBreakMode::BreakWord),
        );
        let (_, break_all_height) = system.measure_with_options(
            text,
            &style,
            TextLayoutOptions::new()
                .with_max_width(constrained)
                .with_wrap(TextWrap::Word)
                .with_break_mode(TextBreakMode::BreakAll),
        );

        assert!(
            break_all_height >= break_word_height,
            "expected break-all to be at least as aggressive as break-word: {break_word_height} vs {break_all_height}"
        );
    }

    #[test]
    fn ellipsize_with_options_truncates_single_line_text() {
        let mut system = TextSystem::new_headless();
        let style = TextStyle::default().with_family("Inter").with_size(16.0);
        let source = "Sparsha ellipsizes long labels cleanly.";
        let options = TextLayoutOptions::new()
            .with_max_width(Some(120.0))
            .with_alignment(TextLayoutAlignment::Center)
            .with_max_lines(Some(1));

        let truncated = system.ellipsize_with_options(source, &style, options);
        assert!(truncated.ends_with('\u{2026}'));
        assert_ne!(truncated, source);
        assert!(text_fits_layout(&mut system, &truncated, &style, options));
    }

    #[test]
    fn ellipsize_with_options_respects_multiline_clamp() {
        let mut system = TextSystem::new_headless();
        let style = TextStyle::default().with_family("Inter").with_size(16.0);
        let source = "First paragraph line that wraps.\nSecond paragraph line that also wraps.";
        let options = TextLayoutOptions::new()
            .with_max_width(Some(180.0))
            .with_alignment(TextLayoutAlignment::Start)
            .with_max_lines(Some(2));

        let truncated = system.ellipsize_with_options(source, &style, options);
        let layout = system.layout_info(&truncated, &style, options);

        assert!(truncated.ends_with('\u{2026}'));
        assert!(layout.lines.len() <= 2);
        assert!(text_fits_layout(&mut system, &truncated, &style, options));
    }
}
