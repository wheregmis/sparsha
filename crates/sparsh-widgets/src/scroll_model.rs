use sparsh_core::Rect;
use sparsh_input::Modifiers;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct ScrollAxes {
    pub horizontal: bool,
    pub vertical: bool,
}

impl ScrollAxes {
    pub(crate) fn new(horizontal: bool, vertical: bool) -> Self {
        Self {
            horizontal,
            vertical,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ScrollAxis {
    Horizontal,
    Vertical,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct Scrollbars {
    pub horizontal_thumb: Option<Rect>,
    pub vertical_thumb: Option<Rect>,
    pub horizontal_track: Option<Rect>,
    pub vertical_track: Option<Rect>,
    pub corner: Option<Rect>,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct ScrollPointerResult {
    pub changed_offset: bool,
    pub changed_visuals: bool,
    pub capture_pointer: bool,
    pub release_pointer: bool,
    pub consume: bool,
}

#[derive(Clone, Copy, Debug)]
struct DragState {
    axis: ScrollAxis,
    pointer_origin: f32,
    offset_origin: f32,
}

#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct ScrollModel {
    offset_x: f32,
    offset_y: f32,
    hover_axis: Option<ScrollAxis>,
    drag: Option<DragState>,
}

impl ScrollModel {
    pub(crate) fn offset(&self) -> (f32, f32) {
        (self.offset_x, self.offset_y)
    }

    pub(crate) fn set_offset(&mut self, x: f32, y: f32) {
        self.offset_x = x.max(0.0);
        self.offset_y = y.max(0.0);
    }

    pub(crate) fn hover_axis(&self) -> Option<ScrollAxis> {
        self.hover_axis
    }

    pub(crate) fn dragging_axis(&self) -> Option<ScrollAxis> {
        self.drag.map(|drag| drag.axis)
    }

    pub(crate) fn clamp(&mut self, viewport: Rect, content_size: (f32, f32), axes: ScrollAxes) {
        let max_x = if axes.horizontal {
            (content_size.0 - viewport.width).max(0.0)
        } else {
            0.0
        };
        let max_y = if axes.vertical {
            (content_size.1 - viewport.height).max(0.0)
        } else {
            0.0
        };
        self.offset_x = self.offset_x.clamp(0.0, max_x);
        self.offset_y = self.offset_y.clamp(0.0, max_y);
    }

    pub(crate) fn scrollbars(
        &self,
        viewport: Rect,
        content_size: (f32, f32),
        thickness: f32,
        axes: ScrollAxes,
    ) -> Scrollbars {
        let show_horizontal = axes.horizontal && content_size.0 > viewport.width;
        let show_vertical = axes.vertical && content_size.1 > viewport.height;

        let vertical_track_height = if show_horizontal {
            (viewport.height - thickness).max(0.0)
        } else {
            viewport.height
        };
        let horizontal_track_width = if show_vertical {
            (viewport.width - thickness).max(0.0)
        } else {
            viewport.width
        };

        let vertical_track = show_vertical.then(|| {
            Rect::new(
                viewport.x + viewport.width - thickness,
                viewport.y,
                thickness,
                vertical_track_height,
            )
        });
        let horizontal_track = show_horizontal.then(|| {
            Rect::new(
                viewport.x,
                viewport.y + viewport.height - thickness,
                horizontal_track_width,
                thickness,
            )
        });

        let vertical_thumb = vertical_track.map(|track| {
            let thumb_height =
                (track.height * (viewport.height / content_size.1)).clamp(20.0, track.height);
            let max_offset = (content_size.1 - viewport.height).max(0.0);
            let thumb_y = if max_offset > 0.0 {
                (self.offset_y / max_offset) * (track.height - thumb_height)
            } else {
                0.0
            };
            Rect::new(track.x, track.y + thumb_y, thickness, thumb_height)
        });

        let horizontal_thumb = horizontal_track.map(|track| {
            let thumb_width =
                (track.width * (viewport.width / content_size.0)).clamp(20.0, track.width);
            let max_offset = (content_size.0 - viewport.width).max(0.0);
            let thumb_x = if max_offset > 0.0 {
                (self.offset_x / max_offset) * (track.width - thumb_width)
            } else {
                0.0
            };
            Rect::new(track.x + thumb_x, track.y, thumb_width, thickness)
        });

        Scrollbars {
            horizontal_thumb,
            vertical_thumb,
            horizontal_track,
            vertical_track,
            corner: (show_horizontal && show_vertical).then(|| {
                Rect::new(
                    viewport.x + viewport.width - thickness,
                    viewport.y + viewport.height - thickness,
                    thickness,
                    thickness,
                )
            }),
        }
    }

    pub(crate) fn scroll_by(
        &mut self,
        viewport: Rect,
        content_size: (f32, f32),
        axes: ScrollAxes,
        delta: glam::Vec2,
        modifiers: Modifiers,
    ) -> bool {
        let mut delta_x = delta.x;
        let mut delta_y = delta.y;

        if modifiers.shift() && delta_x.abs() <= f32::EPSILON && axes.horizontal {
            delta_x = delta_y;
            if axes.vertical {
                delta_y = 0.0;
            }
        }

        let old = (self.offset_x, self.offset_y);
        if axes.horizontal {
            self.offset_x -= delta_x * 20.0;
        }
        if axes.vertical {
            self.offset_y -= delta_y * 20.0;
        }
        self.clamp(viewport, content_size, axes);
        old != (self.offset_x, self.offset_y)
    }

    pub(crate) fn pointer_move(
        &mut self,
        pos: glam::Vec2,
        viewport: Rect,
        content_size: (f32, f32),
        thickness: f32,
        axes: ScrollAxes,
    ) -> ScrollPointerResult {
        let scrollbars = self.scrollbars(viewport, content_size, thickness, axes);
        let mut result = ScrollPointerResult::default();

        if let Some(drag) = self.drag {
            let (track, thumb, max_offset) = match drag.axis {
                ScrollAxis::Horizontal => (
                    scrollbars.horizontal_track,
                    scrollbars.horizontal_thumb,
                    (content_size.0 - viewport.width).max(0.0),
                ),
                ScrollAxis::Vertical => (
                    scrollbars.vertical_track,
                    scrollbars.vertical_thumb,
                    (content_size.1 - viewport.height).max(0.0),
                ),
            };
            if let (Some(track), Some(thumb)) = (track, thumb) {
                let track_span = match drag.axis {
                    ScrollAxis::Horizontal => track.width - thumb.width,
                    ScrollAxis::Vertical => track.height - thumb.height,
                }
                .max(0.0);
                let pointer = axis_value(pos, drag.axis);
                let delta = pointer - drag.pointer_origin;
                let next_offset = if track_span > 0.0 && max_offset > 0.0 {
                    (drag.offset_origin + delta / track_span * max_offset).clamp(0.0, max_offset)
                } else {
                    0.0
                };
                let changed = match drag.axis {
                    ScrollAxis::Horizontal => {
                        let changed = (self.offset_x - next_offset).abs() > f32::EPSILON;
                        self.offset_x = next_offset;
                        changed
                    }
                    ScrollAxis::Vertical => {
                        let changed = (self.offset_y - next_offset).abs() > f32::EPSILON;
                        self.offset_y = next_offset;
                        changed
                    }
                };
                result.changed_offset = changed;
                result.changed_visuals = true;
                result.consume = true;
            }
            return result;
        }

        let previous_hover = self.hover_axis;
        self.hover_axis = if scrollbars
            .vertical_thumb
            .is_some_and(|thumb| thumb.contains(pos))
        {
            Some(ScrollAxis::Vertical)
        } else if scrollbars
            .horizontal_thumb
            .is_some_and(|thumb| thumb.contains(pos))
        {
            Some(ScrollAxis::Horizontal)
        } else {
            None
        };
        result.changed_visuals = previous_hover != self.hover_axis;
        result
    }

    pub(crate) fn pointer_down(
        &mut self,
        pos: glam::Vec2,
        viewport: Rect,
        content_size: (f32, f32),
        thickness: f32,
        axes: ScrollAxes,
    ) -> ScrollPointerResult {
        let scrollbars = self.scrollbars(viewport, content_size, thickness, axes);
        if let Some(thumb) = scrollbars.vertical_thumb {
            if thumb.contains(pos) {
                self.drag = Some(DragState {
                    axis: ScrollAxis::Vertical,
                    pointer_origin: pos.y,
                    offset_origin: self.offset_y,
                });
                self.hover_axis = Some(ScrollAxis::Vertical);
                return ScrollPointerResult {
                    changed_visuals: true,
                    capture_pointer: true,
                    consume: true,
                    ..Default::default()
                };
            }
        }
        if let Some(thumb) = scrollbars.horizontal_thumb {
            if thumb.contains(pos) {
                self.drag = Some(DragState {
                    axis: ScrollAxis::Horizontal,
                    pointer_origin: pos.x,
                    offset_origin: self.offset_x,
                });
                self.hover_axis = Some(ScrollAxis::Horizontal);
                return ScrollPointerResult {
                    changed_visuals: true,
                    capture_pointer: true,
                    consume: true,
                    ..Default::default()
                };
            }
        }

        if let Some(track) = scrollbars.vertical_track {
            if track.contains(pos) {
                let page = (viewport.height * 0.8).max(40.0);
                let direction = if pos.y
                    < scrollbars
                        .vertical_thumb
                        .map(|thumb| thumb.y)
                        .unwrap_or(track.y)
                {
                    -1.0
                } else {
                    1.0
                };
                let before = self.offset_y;
                self.offset_y += direction * page;
                self.clamp(viewport, content_size, axes);
                return ScrollPointerResult {
                    changed_offset: (before - self.offset_y).abs() > f32::EPSILON,
                    changed_visuals: true,
                    consume: true,
                    ..Default::default()
                };
            }
        }

        if let Some(track) = scrollbars.horizontal_track {
            if track.contains(pos) {
                let page = (viewport.width * 0.8).max(40.0);
                let direction = if pos.x
                    < scrollbars
                        .horizontal_thumb
                        .map(|thumb| thumb.x)
                        .unwrap_or(track.x)
                {
                    -1.0
                } else {
                    1.0
                };
                let before = self.offset_x;
                self.offset_x += direction * page;
                self.clamp(viewport, content_size, axes);
                return ScrollPointerResult {
                    changed_offset: (before - self.offset_x).abs() > f32::EPSILON,
                    changed_visuals: true,
                    consume: true,
                    ..Default::default()
                };
            }
        }

        ScrollPointerResult::default()
    }

    pub(crate) fn pointer_up(&mut self, pos: glam::Vec2) -> ScrollPointerResult {
        let _ = pos;
        let had_drag = self.drag.take().is_some();
        let previous_hover = self.hover_axis;
        if had_drag {
            self.hover_axis = None;
        }
        ScrollPointerResult {
            changed_visuals: had_drag || previous_hover != self.hover_axis,
            release_pointer: had_drag,
            consume: had_drag,
            ..Default::default()
        }
    }
}

fn axis_value(pos: glam::Vec2, axis: ScrollAxis) -> f32 {
    match axis {
        ScrollAxis::Horizontal => pos.x,
        ScrollAxis::Vertical => pos.y,
    }
}
