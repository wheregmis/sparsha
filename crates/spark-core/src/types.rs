//! Common types used throughout the framework.

use bytemuck::{Pod, Zeroable};
pub use glam::{Mat4, Vec2, Vec3, Vec4};

/// RGBA color with f32 components (0.0 - 1.0).
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Pod, Zeroable)]
pub struct Color {
    pub r: f32,
    pub g: f32,
    pub b: f32,
    pub a: f32,
}

impl Color {
    pub const WHITE: Self = Self::rgb(1.0, 1.0, 1.0);
    pub const BLACK: Self = Self::rgb(0.0, 0.0, 0.0);
    pub const RED: Self = Self::rgb(1.0, 0.0, 0.0);
    pub const GREEN: Self = Self::rgb(0.0, 1.0, 0.0);
    pub const BLUE: Self = Self::rgb(0.0, 0.0, 1.0);
    pub const TRANSPARENT: Self = Self::rgba(0.0, 0.0, 0.0, 0.0);

    pub const fn rgb(r: f32, g: f32, b: f32) -> Self {
        Self { r, g, b, a: 1.0 }
    }

    pub const fn rgba(r: f32, g: f32, b: f32, a: f32) -> Self {
        Self { r, g, b, a }
    }

    /// Create from hex color (e.g., 0xFF5500 for orange).
    pub fn from_hex(hex: u32) -> Self {
        let r = ((hex >> 16) & 0xFF) as f32 / 255.0;
        let g = ((hex >> 8) & 0xFF) as f32 / 255.0;
        let b = (hex & 0xFF) as f32 / 255.0;
        Self::rgb(r, g, b)
    }

    /// Create from hex color with alpha (e.g., 0xFF550080 for semi-transparent orange).
    pub fn from_hex_alpha(hex: u32) -> Self {
        let r = ((hex >> 24) & 0xFF) as f32 / 255.0;
        let g = ((hex >> 16) & 0xFF) as f32 / 255.0;
        let b = ((hex >> 8) & 0xFF) as f32 / 255.0;
        let a = (hex & 0xFF) as f32 / 255.0;
        Self::rgba(r, g, b, a)
    }

    pub fn to_array(self) -> [f32; 4] {
        [self.r, self.g, self.b, self.a]
    }

    /// Convert to u8 array (0-255 range).
    pub fn to_u8_array(self) -> [u8; 4] {
        [
            (self.r * 255.0) as u8,
            (self.g * 255.0) as u8,
            (self.b * 255.0) as u8,
            (self.a * 255.0) as u8,
        ]
    }

    pub fn with_alpha(self, a: f32) -> Self {
        Self { a, ..self }
    }
}

impl From<[f32; 4]> for Color {
    fn from(arr: [f32; 4]) -> Self {
        Self {
            r: arr[0],
            g: arr[1],
            b: arr[2],
            a: arr[3],
        }
    }
}

impl From<Color> for [f32; 4] {
    fn from(c: Color) -> Self {
        c.to_array()
    }
}

/// A 2D rectangle defined by position and size.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Rect {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

impl Rect {
    pub const ZERO: Self = Self::new(0.0, 0.0, 0.0, 0.0);

    pub const fn new(x: f32, y: f32, width: f32, height: f32) -> Self {
        Self { x, y, width, height }
    }

    pub fn from_pos_size(pos: Vec2, size: Vec2) -> Self {
        Self {
            x: pos.x,
            y: pos.y,
            width: size.x,
            height: size.y,
        }
    }

    pub fn pos(&self) -> Vec2 {
        Vec2::new(self.x, self.y)
    }

    pub fn size(&self) -> Vec2 {
        Vec2::new(self.width, self.height)
    }

    pub fn min(&self) -> Vec2 {
        self.pos()
    }

    pub fn max(&self) -> Vec2 {
        Vec2::new(self.x + self.width, self.y + self.height)
    }

    pub fn center(&self) -> Vec2 {
        Vec2::new(self.x + self.width * 0.5, self.y + self.height * 0.5)
    }

    pub fn contains(&self, point: Vec2) -> bool {
        point.x >= self.x
            && point.x <= self.x + self.width
            && point.y >= self.y
            && point.y <= self.y + self.height
    }

    pub fn intersects(&self, other: &Rect) -> bool {
        self.x < other.x + other.width
            && self.x + self.width > other.x
            && self.y < other.y + other.height
            && self.y + self.height > other.y
    }

    pub fn intersection(&self, other: &Rect) -> Option<Rect> {
        let x = self.x.max(other.x);
        let y = self.y.max(other.y);
        let max_x = (self.x + self.width).min(other.x + other.width);
        let max_y = (self.y + self.height).min(other.y + other.height);

        if max_x > x && max_y > y {
            Some(Rect::new(x, y, max_x - x, max_y - y))
        } else {
            None
        }
    }

    pub fn translate(&self, offset: Vec2) -> Self {
        Self {
            x: self.x + offset.x,
            y: self.y + offset.y,
            ..*self
        }
    }

    pub fn inset(&self, amount: f32) -> Self {
        Self {
            x: self.x + amount,
            y: self.y + amount,
            width: (self.width - amount * 2.0).max(0.0),
            height: (self.height - amount * 2.0).max(0.0),
        }
    }
}

/// A 2D point (alias for Vec2 for clarity).
pub type Point = Vec2;

/// Global uniforms passed to all shaders.
#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub struct GlobalUniforms {
    /// Viewport size in pixels.
    pub viewport_size: [f32; 2],
    /// Scale factor (for HiDPI).
    pub scale_factor: f32,
    /// Time since app start in seconds.
    pub time: f32,
}

impl Default for GlobalUniforms {
    fn default() -> Self {
        Self {
            viewport_size: [800.0, 600.0],
            scale_factor: 1.0,
            time: 0.0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn color_constants() {
        assert_eq!(Color::WHITE.r, 1.0);
        assert_eq!(Color::WHITE.g, 1.0);
        assert_eq!(Color::WHITE.b, 1.0);
        assert_eq!(Color::WHITE.a, 1.0);
        assert_eq!(Color::BLACK, Color::rgb(0.0, 0.0, 0.0));
        assert_eq!(Color::TRANSPARENT.a, 0.0);
        assert_eq!(Color::RED, Color::rgb(1.0, 0.0, 0.0));
        assert_eq!(Color::GREEN, Color::rgb(0.0, 1.0, 0.0));
        assert_eq!(Color::BLUE, Color::rgb(0.0, 0.0, 1.0));
    }

    #[test]
    fn color_from_hex() {
        let red = Color::from_hex(0xFF0000);
        assert!((red.r - 1.0).abs() < 1e-5);
        assert!(red.g.abs() < 1e-5);
        assert!(red.b.abs() < 1e-5);
        assert_eq!(red.a, 1.0);

        let green = Color::from_hex(0x00FF00);
        assert!(green.r.abs() < 1e-5);
        assert!((green.g - 1.0).abs() < 1e-5);
        assert!(green.b.abs() < 1e-5);
    }

    #[test]
    fn color_from_hex_alpha() {
        let semi = Color::from_hex_alpha(0xFF000080);
        assert!((semi.r - 1.0).abs() < 1e-5);
        assert!((semi.a - 0.5).abs() < 0.01);
    }

    #[test]
    fn color_to_array_with_alpha() {
        let c = Color::rgba(0.5, 0.25, 0.75, 0.5);
        assert_eq!(c.to_array(), [0.5, 0.25, 0.75, 0.5]);
        assert_eq!(c.to_u8_array(), [127, 63, 191, 127]);
        let with_a = Color::WHITE.with_alpha(0.5);
        assert_eq!(with_a.a, 0.5);
    }

    #[test]
    fn color_from_into_array() {
        let arr = [0.1, 0.2, 0.3, 0.4];
        let c: Color = arr.into();
        assert_eq!(c.r, 0.1);
        assert_eq!(c.g, 0.2);
        assert_eq!(c.b, 0.3);
        assert_eq!(c.a, 0.4);
        let back: [f32; 4] = c.into();
        assert_eq!(back, arr);
    }

    #[test]
    fn rect_new_pos_size() {
        let r = Rect::new(10.0, 20.0, 100.0, 50.0);
        assert_eq!(r.x, 10.0);
        assert_eq!(r.y, 20.0);
        assert_eq!(r.width, 100.0);
        assert_eq!(r.height, 50.0);
        assert_eq!(Rect::ZERO, Rect::new(0.0, 0.0, 0.0, 0.0));
        let r2 = Rect::from_pos_size(Vec2::new(5.0, 5.0), Vec2::new(20.0, 30.0));
        assert_eq!(r2.pos(), Vec2::new(5.0, 5.0));
        assert_eq!(r2.size(), Vec2::new(20.0, 30.0));
        assert_eq!(r2.min(), Vec2::new(5.0, 5.0));
        assert_eq!(r2.max(), Vec2::new(25.0, 35.0));
        assert_eq!(r2.center(), Vec2::new(15.0, 20.0));
    }

    #[test]
    fn rect_contains() {
        let r = Rect::new(0.0, 0.0, 100.0, 50.0);
        assert!(r.contains(Vec2::new(50.0, 25.0)));
        assert!(r.contains(Vec2::new(0.0, 0.0)));
        assert!(r.contains(Vec2::new(100.0, 50.0)));
        assert!(!r.contains(Vec2::new(101.0, 25.0)));
        assert!(!r.contains(Vec2::new(50.0, 51.0)));
    }

    #[test]
    fn rect_intersects_intersection() {
        let a = Rect::new(0.0, 0.0, 100.0, 100.0);
        let b = Rect::new(50.0, 50.0, 100.0, 100.0);
        assert!(a.intersects(&b));
        let inter = a.intersection(&b).unwrap();
        assert_eq!(inter, Rect::new(50.0, 50.0, 50.0, 50.0));

        let c = Rect::new(200.0, 200.0, 50.0, 50.0);
        assert!(!a.intersects(&c));
        assert!(a.intersection(&c).is_none());

        let d = Rect::new(80.0, 10.0, 50.0, 30.0);
        assert!(a.intersects(&d));
        let inter_ad = a.intersection(&d).unwrap();
        assert_eq!(inter_ad, Rect::new(80.0, 10.0, 20.0, 30.0));
    }

    #[test]
    fn rect_translate_inset() {
        let r = Rect::new(10.0, 20.0, 100.0, 50.0);
        let t = r.translate(Vec2::new(5.0, -5.0));
        assert_eq!(t.x, 15.0);
        assert_eq!(t.y, 15.0);
        assert_eq!(t.width, 100.0);
        assert_eq!(t.height, 50.0);

        let i = r.inset(10.0);
        assert_eq!(i.x, 20.0);
        assert_eq!(i.y, 30.0);
        assert_eq!(i.width, 80.0);
        assert_eq!(i.height, 30.0);

        let over = r.inset(60.0);
        assert!(over.width >= 0.0);
        assert!(over.height >= 0.0);
    }
}

