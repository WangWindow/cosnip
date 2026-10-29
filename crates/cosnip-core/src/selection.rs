//! Selection state used by the overlay.

use crate::geometry::{Point, Rect};

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
}

#[cfg(test)]
mod tests {
    use super::Selection;
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
}
