//! Glyph atlas for GPU text rendering.

use rustc_hash::FxHashMap;
use wgpu::{
    Device, Extent3d, Queue, Texture, TextureDescriptor, TextureDimension, TextureFormat,
    TextureUsages, TextureView, TextureViewDescriptor,
};

/// A cached glyph in the atlas.
#[derive(Clone, Copy, Debug)]
pub struct CachedGlyph {
    /// UV position in the atlas (normalized 0-1).
    pub uv_x: f32,
    pub uv_y: f32,
    /// UV size in the atlas (normalized 0-1).
    pub uv_width: f32,
    pub uv_height: f32,
    /// Glyph dimensions in pixels.
    pub width: u32,
    pub height: u32,
    /// Offset from the baseline.
    pub offset_x: i32,
    pub offset_y: i32,
}

/// Key for looking up cached glyphs.
/// Uses a hash of the font data + glyph ID + font size for unique identification.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct GlyphKey {
    /// Hash of the font source for identification.
    pub font_hash: u64,
    /// Glyph ID.
    pub glyph_id: u32,
    /// Font size in 1/16th pixels (for sub-pixel precision).
    pub font_size_16: u32,
}

impl GlyphKey {
    pub fn new(font_hash: u64, glyph_id: u32, font_size: f32) -> Self {
        Self {
            font_hash,
            glyph_id,
            font_size_16: (font_size * 16.0) as u32,
        }
    }
}

/// Bitmap data for inserting a glyph into the atlas.
pub struct GlyphBitmap<'a> {
    pub width: u32,
    pub height: u32,
    pub offset_x: i32,
    pub offset_y: i32,
    pub data: &'a [u8],
}

/// A simple shelf-based atlas packer.
struct ShelfPacker {
    width: u32,
    height: u32,
    shelf_height: u32,
    shelf_x: u32,
    shelf_y: u32,
}

impl ShelfPacker {
    fn new(width: u32, height: u32) -> Self {
        Self {
            width,
            height,
            shelf_height: 0,
            shelf_x: 0,
            shelf_y: 0,
        }
    }

    fn allocate(&mut self, width: u32, height: u32) -> Option<(u32, u32)> {
        // Add padding
        let padded_width = width + 2;
        let padded_height = height + 2;

        // Check if we need a new shelf
        if self.shelf_x + padded_width > self.width {
            // Move to next shelf
            self.shelf_y += self.shelf_height;
            self.shelf_x = 0;
            self.shelf_height = 0;
        }

        // Check if we have vertical space
        if self.shelf_y + padded_height > self.height {
            return None;
        }

        // Update shelf height
        self.shelf_height = self.shelf_height.max(padded_height);

        // Return position (with 1px padding offset)
        let x = self.shelf_x + 1;
        let y = self.shelf_y + 1;

        self.shelf_x += padded_width;

        Some((x, y))
    }

    fn reset(&mut self) {
        self.shelf_height = 0;
        self.shelf_x = 0;
        self.shelf_y = 0;
    }
}

/// GPU texture atlas for glyph caching.
pub struct GlyphAtlas {
    texture: Texture,
    view: TextureView,
    width: u32,
    height: u32,
    packer: ShelfPacker,
    cache: FxHashMap<GlyphKey, CachedGlyph>,
    dirty: bool,
}

impl GlyphAtlas {
    /// Create a new glyph atlas with the given dimensions.
    pub fn new(device: &Device, width: u32, height: u32) -> Self {
        let texture = device.create_texture(&TextureDescriptor {
            label: Some("glyph_atlas"),
            size: Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: TextureDimension::D2,
            format: TextureFormat::R8Unorm,
            usage: TextureUsages::TEXTURE_BINDING | TextureUsages::COPY_DST,
            view_formats: &[],
        });

        let view = texture.create_view(&TextureViewDescriptor::default());

        Self {
            texture,
            view,
            width,
            height,
            packer: ShelfPacker::new(width, height),
            cache: FxHashMap::default(),
            dirty: false,
        }
    }

    /// Get the texture view for binding.
    pub fn view(&self) -> &TextureView {
        &self.view
    }

    /// Get atlas dimensions.
    pub fn size(&self) -> (u32, u32) {
        (self.width, self.height)
    }

    /// Look up a cached glyph.
    pub fn get(&self, key: &GlyphKey) -> Option<&CachedGlyph> {
        self.cache.get(key)
    }

    /// Insert a glyph into the atlas.
    pub fn insert(
        &mut self,
        queue: &Queue,
        key: GlyphKey,
        bitmap: GlyphBitmap<'_>,
    ) -> Option<CachedGlyph> {
        let GlyphBitmap {
            width,
            height,
            offset_x,
            offset_y,
            data,
        } = bitmap;

        // Skip empty glyphs (like spaces)
        if width == 0 || height == 0 {
            let glyph = CachedGlyph {
                uv_x: 0.0,
                uv_y: 0.0,
                uv_width: 0.0,
                uv_height: 0.0,
                width: 0,
                height: 0,
                offset_x,
                offset_y,
            };
            self.cache.insert(key, glyph);
            return Some(glyph);
        }

        // Try to allocate space
        let (x, y) = self.packer.allocate(width, height)?;

        // Upload to texture
        queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &self.texture,
                mip_level: 0,
                origin: wgpu::Origin3d { x, y, z: 0 },
                aspect: wgpu::TextureAspect::All,
            },
            data,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(width),
                rows_per_image: Some(height),
            },
            Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
        );

        let glyph = CachedGlyph {
            uv_x: x as f32 / self.width as f32,
            uv_y: y as f32 / self.height as f32,
            uv_width: width as f32 / self.width as f32,
            uv_height: height as f32 / self.height as f32,
            width,
            height,
            offset_x,
            offset_y,
        };

        self.cache.insert(key, glyph);
        self.dirty = true;

        Some(glyph)
    }

    /// Clear the atlas and cache.
    pub fn clear(&mut self) {
        self.cache.clear();
        self.packer.reset();
        self.dirty = true;
    }

    /// Check if any glyphs were added since last frame.
    pub fn is_dirty(&self) -> bool {
        self.dirty
    }

    /// Mark as clean (call after rendering).
    pub fn mark_clean(&mut self) {
        self.dirty = false;
    }
}
