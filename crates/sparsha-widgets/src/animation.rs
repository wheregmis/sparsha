//! Animation primitives for implicit and explicit UI animation.

use sparsha_core::Color;

/// Easing functions supported by the built-in animation helpers.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AnimationEasing {
    Linear,
    EaseIn,
    EaseOut,
    EaseInOut,
}

impl AnimationEasing {
    pub fn apply(self, t: f32) -> f32 {
        let t = t.clamp(0.0, 1.0);
        match self {
            Self::Linear => t,
            Self::EaseIn => t * t,
            Self::EaseOut => 1.0 - (1.0 - t) * (1.0 - t),
            Self::EaseInOut => {
                if t < 0.5 {
                    2.0 * t * t
                } else {
                    1.0 - ((-2.0 * t + 2.0).powi(2) / 2.0)
                }
            }
        }
    }
}

/// Explicit time-based tween for a scalar value.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Tween {
    from: f32,
    to: f32,
    start_time: f32,
    duration: f32,
    easing: AnimationEasing,
}

impl Tween {
    pub fn new(
        from: f32,
        to: f32,
        start_time: f32,
        duration: f32,
        easing: AnimationEasing,
    ) -> Self {
        Self {
            from,
            to,
            start_time,
            duration: duration.max(0.000_001),
            easing,
        }
    }

    pub fn value_at(&self, now: f32) -> f32 {
        let progress = ((now - self.start_time) / self.duration).clamp(0.0, 1.0);
        let eased = self.easing.apply(progress);
        self.from + (self.to - self.from) * eased
    }

    pub fn is_finished_at(&self, now: f32) -> bool {
        now >= self.start_time + self.duration
    }
}

/// Implicit animation helper for values that animate toward new targets.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ImplicitAnimation {
    current: f32,
    target: f32,
    tween: Option<Tween>,
}

impl ImplicitAnimation {
    pub fn new(initial: f32) -> Self {
        Self {
            current: initial,
            target: initial,
            tween: None,
        }
    }

    pub fn current(&self) -> f32 {
        self.current
    }

    pub fn target(&self) -> f32 {
        self.target
    }

    pub fn is_animating(&self) -> bool {
        self.tween.is_some()
    }

    pub fn set_target(&mut self, target: f32, now: f32, duration: f32, easing: AnimationEasing) {
        self.current = self.sample(now);
        self.target = target;
        if (self.current - target).abs() <= f32::EPSILON {
            self.current = target;
            self.tween = None;
            return;
        }
        self.tween = Some(Tween::new(self.current, target, now, duration, easing));
    }

    pub fn sample(&mut self, now: f32) -> f32 {
        let Some(tween) = self.tween else {
            return self.current;
        };
        self.current = tween.value_at(now);
        if tween.is_finished_at(now) {
            self.current = self.target;
            self.tween = None;
        }
        self.current
    }
}

/// Interpolate between two colors in linear space.
pub fn lerp_color(from: Color, to: Color, t: f32) -> Color {
    let t = t.clamp(0.0, 1.0);
    Color::rgba(
        from.r + (to.r - from.r) * t,
        from.g + (to.g - from.g) * t,
        from.b + (to.b - from.b) * t,
        from.a + (to.a - from.a) * t,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tween_interpolates_and_completes() {
        let tween = Tween::new(0.0, 100.0, 1.0, 2.0, AnimationEasing::Linear);
        assert!((tween.value_at(1.0) - 0.0).abs() < 1e-6);
        assert!((tween.value_at(2.0) - 50.0).abs() < 1e-6);
        assert!((tween.value_at(3.0) - 100.0).abs() < 1e-6);
        assert!(!tween.is_finished_at(2.9));
        assert!(tween.is_finished_at(3.0));
    }

    #[test]
    fn implicit_animation_tracks_target_changes() {
        let mut anim = ImplicitAnimation::new(0.0);
        anim.set_target(10.0, 0.0, 1.0, AnimationEasing::EaseInOut);
        assert!(anim.is_animating());
        let mid = anim.sample(0.5);
        assert!(mid > 0.0 && mid < 10.0);
        let end = anim.sample(1.0);
        assert!((end - 10.0).abs() < 1e-5);
        assert!(!anim.is_animating());
    }

    #[test]
    fn color_lerp_interpolates_rgba_channels() {
        let from = Color::rgba(0.0, 0.25, 0.5, 0.2);
        let to = Color::rgba(1.0, 0.75, 1.0, 1.0);
        let mixed = lerp_color(from, to, 0.5);
        assert!((mixed.r - 0.5).abs() < 1e-6);
        assert!((mixed.g - 0.5).abs() < 1e-6);
        assert!((mixed.b - 0.75).abs() < 1e-6);
        assert!((mixed.a - 0.6).abs() < 1e-6);
    }
}
