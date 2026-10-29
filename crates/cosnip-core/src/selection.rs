//! Selection state used by the overlay.

use crate::geometry::{Point, Rect};

/// One of the eight resize handles around a selection.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Handle {
    NorthWest,
    North,
    NorthEast,
    East,
    SouthEast,
    South,
    SouthWest,
    West,
}

impl Handle {
    pub const ALL: [Self; 8] = [
        Self::NorthWest,
        Self::North,
        Self::NorthEast,
        Self::East,
        Self::SouthEast,
        Self::South,
        Self::SouthWest,
        Self::West,
    ];

    #[must_use]
    pub fn position(self, rect: Rect) -> Point {
        match self {
            Self::NorthWest => Point::new(rect.x, rect.y),
            Self::North => Point::new(rect.x + rect.width as i32 / 2, rect.y),
            Self::NorthEast => Point::new(rect.right(), rect.y),
            Self::East => Point::new(rect.right(), rect.y + rect.height as i32 / 2),
            Self::SouthEast => Point::new(rect.right(), rect.bottom()),
            Self::South => Point::new(rect.x + rect.width as i32 / 2, rect.bottom()),
            Self::SouthWest => Point::new(rect.x, rect.bottom()),
            Self::West => Point::new(rect.x, rect.y + rect.height as i32 / 2),
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Selection {
    rect: Option<Rect>,
}

impl Selection {
    #[must_use]
    pub fn new(rect: Option<Rect>) -> Self {
        Self { rect }
    }

    #[must_use]
    pub fn rect(&self) -> Option<Rect> {
        self.rect
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.rect.is_none_or(Rect::is_empty)
    }

    pub fn set(&mut self, rect: Option<Rect>) {
        self.rect = rect.filter(|r| !r.is_empty());
    }

    pub fn clear(&mut self) {
        self.rect = None;
    }

    /// Begin a new drag selection at `anchor`.
    pub fn begin(&mut self, anchor: Point) {
        self.rect = Some(Rect::new(anchor.x, anchor.y, 0, 0));
    }

    /// Update the current drag. `square` keeps the selection square.
    pub fn drag_to(&mut self, anchor: Point, current: Point, bounds: Rect, square: bool) {
        let raw = if square {
            Rect::square_from_points(anchor, current)
        } else {
            Rect::from_points(anchor, current)
        };
        self.rect = raw.clamp_to(bounds);
    }

    /// Move the selection by a keyboard nudge, clamped to the frame.
    pub fn nudge(&mut self, dx: i32, dy: i32, bounds: Rect) {
        if let Some(rect) = self.rect {
            self.rect = Some(rect.nudge(dx, dy, bounds));
        }
    }

    pub fn full(&mut self, bounds: Rect) {
        self.rect = Some(bounds);
    }

    /// Find the closest resize handle within `tolerance` pixels.
    #[must_use]
    pub fn handle_at(&self, point: Point, tolerance: i32) -> Option<Handle> {
        let rect = self.rect.filter(|rect| !rect.is_empty())?;
        let mut best: Option<(i32, Handle)> = None;
        for handle in Handle::ALL {
            let position = handle.position(rect);
            let distance = (position.x - point.x)
                .abs()
                .max((position.y - point.y).abs());
            if distance <= tolerance && best.is_none_or(|(best, _)| distance < best) {
                best = Some((distance, handle));
            }
        }
        best.map(|(_, handle)| handle)
    }

    /// Move one edge/corner to `point`, keeping the opposite edge fixed and the rectangle valid.
    pub fn resize(&mut self, handle: Handle, point: Point, bounds: Rect, min_size: u32) {
        let Some(rect) = self.rect else {
            return;
        };
        let min_size = min_size.max(1) as i32;
        let mut left = rect.x;
        let mut top = rect.y;
        let mut right = rect.right();
        let mut bottom = rect.bottom();
        match handle {
            Handle::NorthWest => {
                left = point.x;
                top = point.y;
            }
            Handle::North => top = point.y,
            Handle::NorthEast => {
                right = point.x;
                top = point.y;
            }
            Handle::East => right = point.x,
            Handle::SouthEast => {
                right = point.x;
                bottom = point.y;
            }
            Handle::South => bottom = point.y,
            Handle::SouthWest => {
                left = point.x;
                bottom = point.y;
            }
            Handle::West => left = point.x,
        }
        left = left.clamp(bounds.x, right - min_size);
        top = top.clamp(bounds.y, bottom - min_size);
        right = right.clamp(left + min_size, bounds.right());
        bottom = bottom.clamp(top + min_size, bounds.bottom());
        self.rect = Rect::new(
            left,
            top,
            u32::try_from(right - left).unwrap_or(1),
            u32::try_from(bottom - top).unwrap_or(1),
        )
        .clamp_to(bounds);
    }

    /// Move a previously captured selection by the pointer delta, clamped to `bounds`.
    pub fn move_from(&mut self, start: Rect, dx: i32, dy: i32, bounds: Rect) {
        let dx = dx.clamp(bounds.x - start.x, bounds.right() - start.right());
        let dy = dy.clamp(bounds.y - start.y, bounds.bottom() - start.bottom());
        self.rect = Some(Rect::new(
            start.x.saturating_add(dx),
            start.y.saturating_add(dy),
            start.width,
            start.height,
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::{Handle, Selection};
    use crate::geometry::{Point, Rect};

    #[test]
    fn drag_is_clamped() {
        let mut s = Selection::default();
        let bounds = Rect::new(0, 0, 100, 100);
        s.begin(Point::new(80, 80));
        s.drag_to(Point::new(80, 80), Point::new(120, 120), bounds, false);
        assert_eq!(s.rect(), Some(Rect::new(80, 80, 20, 20)));
    }

    #[test]
    fn full_screen_selects_bounds() {
        let mut s = Selection::default();
        s.full(Rect::new(0, 0, 1920, 1080));
        assert_eq!(s.rect(), Some(Rect::new(0, 0, 1920, 1080)));
    }

    #[test]
    fn finds_the_closest_handle() {
        let mut s = Selection::default();
        s.full(Rect::new(100, 100, 200, 100));
        assert_eq!(
            s.handle_at(Point::new(100, 100), 6),
            Some(Handle::NorthWest)
        );
        assert_eq!(s.handle_at(Point::new(300, 150), 6), Some(Handle::East));
        assert_eq!(s.handle_at(Point::new(200, 200), 6), Some(Handle::South));
        assert_eq!(s.handle_at(Point::new(150, 150), 6), None);
    }

    #[test]
    fn resizing_a_corner_keeps_the_opposite_corner() {
        let mut s = Selection::default();
        s.full(Rect::new(100, 100, 200, 100));
        s.resize(
            Handle::SouthEast,
            Point::new(260, 180),
            Rect::new(0, 0, 500, 500),
            4,
        );
        assert_eq!(s.rect(), Some(Rect::new(100, 100, 160, 80)));
    }

    #[test]
    fn resizing_honors_min_size_and_bounds() {
        let mut s = Selection::default();
        s.full(Rect::new(100, 100, 200, 100));
        s.resize(
            Handle::NorthWest,
            Point::new(400, 400),
            Rect::new(0, 0, 500, 500),
            10,
        );
        assert_eq!(s.rect(), Some(Rect::new(290, 190, 10, 10)));
    }

    #[test]
    fn moving_is_clamped_to_the_frame() {
        let mut s = Selection::default();
        let start = Rect::new(100, 100, 200, 100);
        s.full(start);
        s.move_from(start, -500, -500, Rect::new(0, 0, 500, 500));
        assert_eq!(s.rect(), Some(Rect::new(0, 0, 200, 100)));
        s.move_from(start, 1000, 1000, Rect::new(0, 0, 500, 500));
        assert_eq!(s.rect(), Some(Rect::new(300, 400, 200, 100)));
    }
}
