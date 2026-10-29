//! Integer geometry used by capture and the selection overlay.

/// A point in captured-frame coordinates.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct Point {
    pub x: i32,
    pub y: i32,
}

impl Point {
    #[must_use]
    pub const fn new(x: i32, y: i32) -> Self {
        Self { x, y }
    }
}

/// A size in pixels.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct Size {
    pub width: u32,
    pub height: u32,
}

impl Size {
    #[must_use]
    pub const fn new(width: u32, height: u32) -> Self {
        Self { width, height }
    }
}

/// A normalized rectangle. `width`/`height` are zero for an empty rectangle.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct Rect {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}

impl Rect {
    #[must_use]
    pub const fn new(x: i32, y: i32, width: u32, height: u32) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }

    /// Build a normalized rectangle from two arbitrary corners.
    #[must_use]
    pub fn from_points(a: Point, b: Point) -> Self {
        let x = a.x.min(b.x);
        let y = a.y.min(b.y);
        let width = a.x.abs_diff(b.x);
        let height = a.y.abs_diff(b.y);
        Self::new(x, y, width, height)
    }

    /// Build a rectangle constrained to a square, anchored at `anchor`.
    #[must_use]
    pub fn square_from_points(anchor: Point, current: Point) -> Self {
        let dx = current.x - anchor.x;
        let dy = current.y - anchor.y;
        let side = dx.abs().max(dy.abs());
        let x = if dx < 0 { anchor.x - side } else { anchor.x };
        let y = if dy < 0 { anchor.y - side } else { anchor.y };
        Self::new(x, y, side.unsigned_abs(), side.unsigned_abs())
    }

    #[must_use]
    pub const fn is_empty(self) -> bool {
        self.width == 0 || self.height == 0
    }

    #[must_use]
    pub const fn right(self) -> i32 {
        self.x.saturating_add(self.width as i32)
    }

    #[must_use]
    pub const fn bottom(self) -> i32 {
        self.y.saturating_add(self.height as i32)
    }

    #[must_use]
    pub fn contains(self, point: Point) -> bool {
        point.x >= self.x && point.y >= self.y && point.x < self.right() && point.y < self.bottom()
    }

    /// Clamp the rectangle into `bounds`, returning `None` when nothing remains.
    #[must_use]
    pub fn clamp_to(self, bounds: Rect) -> Option<Self> {
        if bounds.is_empty() {
            return None;
        }
        let left = i64::from(self.x).max(i64::from(bounds.x));
        let top = i64::from(self.y).max(i64::from(bounds.y));
        let right = i64::from(self.right()).min(i64::from(bounds.right()));
        let bottom = i64::from(self.bottom()).min(i64::from(bounds.bottom()));
        if right <= left || bottom <= top {
            return None;
        }
        Some(Self::new(
            i32::try_from(left).ok()?,
            i32::try_from(top).ok()?,
            u32::try_from(right - left).ok()?,
            u32::try_from(bottom - top).ok()?,
        ))
    }

    #[must_use]
    pub fn intersect(self, other: Rect) -> Option<Self> {
        let left = i64::from(self.x).max(i64::from(other.x));
        let top = i64::from(self.y).max(i64::from(other.y));
        let right = i64::from(self.right()).min(i64::from(other.right()));
        let bottom = i64::from(self.bottom()).min(i64::from(other.bottom()));
        if right <= left || bottom <= top {
            return None;
        }
        Some(Self::new(
            i32::try_from(left).ok()?,
            i32::try_from(top).ok()?,
            u32::try_from(right - left).ok()?,
            u32::try_from(bottom - top).ok()?,
        ))
    }

    #[must_use]
    pub fn nudge(self, dx: i32, dy: i32, bounds: Rect) -> Self {
        let moved = Self::new(
            self.x.saturating_add(dx),
            self.y.saturating_add(dy),
            self.width,
            self.height,
        );
        moved.clamp_to(bounds).unwrap_or(self)
    }

    /// Grow the rectangle by `amount` on every side, saturating at the coordinate limits.
    #[must_use]
    pub fn expand(self, amount: i32) -> Self {
        if amount <= 0 {
            return self;
        }
        let amount = i64::from(amount);
        let left = i64::from(self.x) - amount;
        let top = i64::from(self.y) - amount;
        let right = i64::from(self.right()) + amount;
        let bottom = i64::from(self.bottom()) + amount;
        Self::new(
            i32::try_from(left).unwrap_or(i32::MIN),
            i32::try_from(top).unwrap_or(i32::MIN),
            u32::try_from(right - left).unwrap_or(u32::MAX),
            u32::try_from(bottom - top).unwrap_or(u32::MAX),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::{Point, Rect};

    #[test]
    fn normalizes_reverse_drag() {
        let r = Rect::from_points(Point::new(500, 400), Point::new(100, 100));
        assert_eq!(r, Rect::new(100, 100, 400, 300));
    }

    #[test]
    fn square_uses_dominant_axis() {
        let r = Rect::square_from_points(Point::new(100, 100), Point::new(180, 130));
        assert_eq!(r, Rect::new(100, 100, 80, 80));
    }

    #[test]
    fn clamps_to_bounds() {
        let bounds = Rect::new(0, 0, 1920, 1080);
        let r = Rect::new(-10, 1000, 500, 300).clamp_to(bounds);
        assert_eq!(r, Some(Rect::new(0, 1000, 490, 80)));
    }

    #[test]
    fn rejects_empty_after_clamp() {
        let bounds = Rect::new(0, 0, 100, 100);
        assert_eq!(Rect::new(-50, -50, 10, 10).clamp_to(bounds), None);
    }
}
