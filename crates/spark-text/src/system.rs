//! Text shaping and layout system using Parley.

use crate::atlas::{CachedGlyph, GlyphAtlas, GlyphBitmap, GlyphKey};
use parley::{
    fontique::Blob,
    layout::{Alignment, GlyphRun, PositionedLayoutItem},
    style::{FontFamily, FontStack, FontStyle, FontWeight, GenericFamily, LineHeight, StyleProperty},
    FontContext, Layout, LayoutContext,
};
use spark_core::{Color, GlyphInstance};
use swash::{
    scale::{Render, ScaleContext, Source, StrikeWith},
    zeno::{Format, Vector},
    FontRef,
};
use wgpu::{Device, Queue};

// Embed the Inter font at compile time
static INTER_REGULAR: &[u8] = include_bytes!("../../../assets/fonts/Inter-Regular.ttf");
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
    atlas: GlyphAtlas,
}

impl TextSystem {
    /// Create a new text system.
    pub fn new(device: &Device) -> Self {
        let mut font_cx = FontContext::new();
        
        // Register embedded Inter fonts
        let regular_blob = Blob::new(std::sync::Arc::new(INTER_REGULAR.to_vec()));
        let bold_blob = Blob::new(std::sync::Arc::new(INTER_BOLD.to_vec()));
        
        font_cx.collection.register_fonts(regular_blob, None);
        font_cx.collection.register_fonts(bold_blob, None);
        
        
        let layout_cx = LayoutContext::new();
        let scale_cx = ScaleContext::new();
        let atlas = GlyphAtlas::new(device, 1024, 1024);

        Self {
            font_cx,
            layout_cx,
            scale_cx,
            atlas,
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
    pub fn atlas(&self) -> &GlyphAtlas {
        &self.atlas
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
        if text.is_empty() {
            return ShapedText::default();
        }

        // Build layout with Parley
        let mut builder = self
            .layout_cx
            .ranged_builder(&mut self.font_cx, text, 1.0, true);

        // Apply default styles
        builder.push_default(StyleProperty::FontSize(style.font_size));
        builder.push_default(StyleProperty::LineHeight(LineHeight::FontSizeRelative(
            style.line_height,
        )));
        
        // Use embedded Inter font with fallback to system sans-serif
        builder.push_default(StyleProperty::FontStack(FontStack::List(
            vec![
                FontFamily::Named("Inter".into()),
                FontFamily::Generic(GenericFamily::SansSerif),
            ]
            .into(),
        )));

        // Apply weight and style
        if style.bold {
            builder.push_default(StyleProperty::FontWeight(FontWeight::BOLD));
        }
        if style.italic {
            builder.push_default(StyleProperty::FontStyle(FontStyle::Italic));
        }

        // Set brush color (Parley uses [u8; 4] for colors)
        let color_arr = style.color.to_u8_array();
        builder.push_default(StyleProperty::Brush(color_arr));

        // Build the layout
        let mut layout: Layout<[u8; 4]> = builder.build(text);

        // Perform line breaking
        layout.break_all_lines(max_width);
        layout.align(max_width, Alignment::Start, Default::default());

        // Collect glyph instances
        let mut glyphs = Vec::new();
        let mut min_y: f32 = f32::MAX;
        let mut max_y: f32 = f32::MIN;

        for line in layout.lines() {
            for item in line.items() {
                if let PositionedLayoutItem::GlyphRun(glyph_run) = item {
                    self.render_glyph_run(
                        device,
                        queue,
                        &glyph_run,
                        &mut glyphs,
                        &mut min_y,
                        &mut max_y,
                    );
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
            style.font_size * style.line_height
        } else if max_y > f32::MIN {
            max_y
        } else {
            style.font_size * style.line_height
        };

        ShapedText {
            glyphs,
            width: layout.width(),
            height: total_height,
        }
    }

    fn render_glyph_run(
        &mut self,
        device: &Device,
        queue: &Queue,
        glyph_run: &GlyphRun<'_, [u8; 4]>,
        glyphs: &mut Vec<GlyphInstance>,
        min_y: &mut f32,
        max_y: &mut f32,
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

            let cached = if let Some(cached) = self.atlas.get(&key) {
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
                        let cached = self.atlas.insert(
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
                                self.atlas.clear();
                                self.atlas = GlyphAtlas::new(device, 2048, 2048);
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
        if text.is_empty() {
            return (0.0, style.font_size * style.line_height);
        }

        // Build layout with Parley
        let mut builder = self
            .layout_cx
            .ranged_builder(&mut self.font_cx, text, 1.0, true);

        // Apply styles
        builder.push_default(StyleProperty::FontSize(style.font_size));
        builder.push_default(StyleProperty::LineHeight(LineHeight::FontSizeRelative(
            style.line_height,
        )));
        
        // Use embedded Inter font with fallback to system sans-serif
        builder.push_default(StyleProperty::FontStack(FontStack::List(
            vec![
                FontFamily::Named("Inter".into()),
                FontFamily::Generic(GenericFamily::SansSerif),
            ]
            .into(),
        )));

        if style.bold {
            builder.push_default(StyleProperty::FontWeight(FontWeight::BOLD));
        }
        if style.italic {
            builder.push_default(StyleProperty::FontStyle(FontStyle::Italic));
        }

        let mut layout: Layout<[u8; 4]> = builder.build(text);

        // Perform line breaking
        layout.break_all_lines(max_width);

        (layout.width(), layout.height())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use spark_core::GlyphInstance;

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
}
