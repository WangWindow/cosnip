//! Annotation model, tools, and undo/redo history.
//!
//! Annotations live in captured-frame coordinates. The UI previews them with egui and the imaging
//! crate rasterizes the same model into copied/saved/pinned output.

use crate::geometry::{Point, Rect};

/// The active overlay tool.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum Tool {
    /// Adjust or create the selection only.
    #[default]
    Select,
    Rectangle,
    Ellipse,
    Arrow,
    Line,
    Pencil,
    Marker,
    Text,
    Number,
    Mosaic,
    Blur,
    Magnifier,
    Eraser,
}

impl Tool {
    /// Toolbar order; the UI and tests share it.
    pub const ALL: [Self; 13] = [
        Self::Select,
        Self::Rectangle,
        Self::Ellipse,
        Self::Arrow,
        Self::Line,
        Self::Pencil,
        Self::Marker,
        Self::Text,
        Self::Number,
        Self::Mosaic,
        Self::Blur,
        Self::Magnifier,
        Self::Eraser,
    ];

    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Select => "Select",
            Self::Rectangle => "Rect",
            Self::Ellipse => "Ellipse",
            Self::Arrow => "Arrow",
            Self::Line => "Line",
            Self::Pencil => "Pencil",
            Self::Marker => "Marker",
            Self::Text => "Text",
            Self::Number => "No.",
            Self::Mosaic => "Mosaic",
            Self::Blur => "Blur",
            Self::Magnifier => "Zoom",
            Self::Eraser => "Eraser",
        }
    }

    #[must_use]
    pub const fn is_draw_tool(self) -> bool {
        matches!(
            self,
            Self::Rectangle
                | Self::Ellipse
                | Self::Arrow
                | Self::Line
                | Self::Pencil
                | Self::Marker
                | Self::Text
                | Self::Number
                | Self::Mosaic
                | Self::Blur
                | Self::Magnifier
                | Self::Eraser
        )
    }

    #[must_use]
    pub const fn is_rect_drag_tool(self) -> bool {
        matches!(self, Self::Rectangle | Self::Ellipse)
    }
}

/// A committed annotation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Annotation {
    pub kind: AnnotationKind,
    pub color: [u8; 4],
    pub width: u32,
}

impl Annotation {
    #[must_use]
    pub fn new(kind: AnnotationKind, color: [u8; 4], width: u32) -> Self {
        Self {
            kind,
            color,
            width: width.max(1),
        }
    }

    /// Bounding box used for hit-testing and erasing.
    #[must_use]
    pub fn bounds(&self) -> Rect {
        let pad = i32::try_from(self.width / 2).unwrap_or(0).saturating_add(2);
        let rect = match &self.kind {
            AnnotationKind::Rectangle { rect } | AnnotationKind::Ellipse { rect } => *rect,
            AnnotationKind::Arrow { start, end } | AnnotationKind::Line { start, end } => {
                Rect::from_points(*start, *end)
            }
            AnnotationKind::Pencil { points } | AnnotationKind::Marker { points } => {
                points_bounds(points)
            }
            AnnotationKind::Mosaic { points }
            | AnnotationKind::Blur { points }
            | AnnotationKind::Eraser { points } => {
                let radius = self.kind.brush_radius(self.width);
                points_bounds(points).expand(radius)
            }
            AnnotationKind::Text {
                position,
                text,
                size,
            } => {
                let width =
                    (text.chars().count() as u32).saturating_mul(size.saturating_mul(3) / 5);
                Rect::new(position.x, position.y, width.max(1), size.saturating_add(4))
            }
            AnnotationKind::Number { position, .. } => Rect::new(position.x, position.y, 28, 28),
            AnnotationKind::Magnifier { center, radius, .. } => Rect::new(
                center.x - *radius as i32,
                center.y - *radius as i32,
                radius.saturating_mul(2),
                radius.saturating_mul(2),
            ),
        };
        rect.expand(pad)
    }

    /// Rough hit test for the eraser: bounding box plus line-distance checks.
    #[must_use]
    pub fn hit_test(&self, point: Point) -> bool {
        if !self.bounds().contains(point) {
            return false;
        }
        let tolerance = i32::try_from(self.width / 2).unwrap_or(0).saturating_add(4);
        match &self.kind {
            AnnotationKind::Line { start, end } | AnnotationKind::Arrow { start, end } => {
                distance_to_segment(point, *start, *end) <= tolerance as f32
            }
            AnnotationKind::Pencil { points } | AnnotationKind::Marker { points } => points
                .windows(2)
                .any(|pair| distance_to_segment(point, pair[0], pair[1]) <= tolerance as f32),
            AnnotationKind::Mosaic { points }
            | AnnotationKind::Blur { points }
            | AnnotationKind::Eraser { points } => {
                let tolerance = self.kind.brush_radius(self.width) as f32;
                points
                    .windows(2)
                    .any(|pair| distance_to_segment(point, pair[0], pair[1]) <= tolerance)
            }
            _ => true,
        }
    }
}

fn points_bounds(points: &[Point]) -> Rect {
    let Some(first) = points.first() else {
        return Rect::default();
    };
    let mut min_x = first.x;
    let mut min_y = first.y;
    let mut max_x = first.x;
    let mut max_y = first.y;
    for point in points.iter().skip(1) {
        min_x = min_x.min(point.x);
        min_y = min_y.min(point.y);
        max_x = max_x.max(point.x);
        max_y = max_y.max(point.y);
    }
    Rect::new(
        min_x,
        min_y,
        u32::try_from(max_x - min_x).unwrap_or(0),
        u32::try_from(max_y - min_y).unwrap_or(0),
    )
}

fn distance_to_segment(point: Point, start: Point, end: Point) -> f32 {
    let px = point.x as f32;
    let py = point.y as f32;
    let ax = start.x as f32;
    let ay = start.y as f32;
    let bx = end.x as f32;
    let by = end.y as f32;
    let dx = bx - ax;
    let dy = by - ay;
    let length_squared = dx * dx + dy * dy;
    if length_squared <= f32::EPSILON {
        return ((px - ax).powi(2) + (py - ay).powi(2)).sqrt();
    }
    let t = (((px - ax) * dx + (py - ay) * dy) / length_squared).clamp(0.0, 1.0);
    let cx = ax + t * dx;
    let cy = ay + t * dy;
    ((px - cx).powi(2) + (py - cy).powi(2)).sqrt()
}

impl AnnotationKind {
    /// Brush radius in image pixels for continuous-effect tools.
    #[must_use]
    pub fn brush_radius(&self, width: u32) -> i32 {
        match self {
            Self::Mosaic { .. } => (width.saturating_mul(12).max(36) / 2) as i32,
            Self::Blur { .. } | Self::Eraser { .. } => {
                (width.saturating_mul(10).max(30) / 2) as i32
            }
            _ => 0,
        }
    }
}

/// The pixels covered by a continuous brush stroke.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StrokeMask {
    rect: Rect,
    bits: Vec<bool>,
}

impl StrokeMask {
    #[must_use]
    pub const fn rect(&self) -> Rect {
        self.rect
    }

    #[must_use]
    pub fn contains(&self, x: i32, y: i32) -> bool {
        if x < self.rect.x || y < self.rect.y || x >= self.rect.right() || y >= self.rect.bottom() {
            return false;
        }
        let index =
            (y - self.rect.y) as usize * self.rect.width as usize + (x - self.rect.x) as usize;
        self.bits[index]
    }

    #[must_use]
    pub fn any_in(&self, rect: Rect) -> bool {
        let Some(rect) = rect.intersect(self.rect) else {
            return false;
        };
        (rect.y..rect.bottom()).any(|y| {
            (rect.x..rect.right()).any(|x| {
                let index = (y - self.rect.y) as usize * self.rect.width as usize
                    + (x - self.rect.x) as usize;
                self.bits[index]
            })
        })
    }
}

/// Rasterize the area swept by a polyline brush.
#[must_use]
pub fn stroke_mask(points: &[Point], radius: i32, bounds: Rect) -> Option<StrokeMask> {
    let first = points.first()?;
    let radius = radius.max(1);
    let mut min_x = first.x;
    let mut min_y = first.y;
    let mut max_x = first.x;
    let mut max_y = first.y;
    for point in points.iter().skip(1) {
        min_x = min_x.min(point.x);
        min_y = min_y.min(point.y);
        max_x = max_x.max(point.x);
        max_y = max_y.max(point.y);
    }
    let rect = Rect::new(
        min_x.saturating_sub(radius),
        min_y.saturating_sub(radius),
        u32::try_from((max_x - min_x).saturating_add(radius * 2)).unwrap_or(0),
        u32::try_from((max_y - min_y).saturating_add(radius * 2)).unwrap_or(0),
    )
    .intersect(bounds)?;
    let mut bits = vec![false; rect.width as usize * rect.height as usize];

    let mut stamp = |a: Point, b: Point| {
        let left = a.x.min(b.x).saturating_sub(radius);
        let top = a.y.min(b.y).saturating_sub(radius);
        let right = a.x.max(b.x).saturating_add(radius).saturating_add(1);
        let bottom = a.y.max(b.y).saturating_add(radius).saturating_add(1);
        let Some(area) = Rect::new(
            left,
            top,
            u32::try_from(right - left).unwrap_or(0),
            u32::try_from(bottom - top).unwrap_or(0),
        )
        .intersect(rect) else {
            return;
        };
        for y in area.y..area.bottom() {
            for x in area.x..area.right() {
                if distance_to_segment(Point::new(x, y), a, b) <= radius as f32 {
                    let index = (y - rect.y) as usize * rect.width as usize + (x - rect.x) as usize;
                    bits[index] = true;
                }
            }
        }
    };

    if points.len() == 1 {
        stamp(points[0], points[0]);
    } else {
        for pair in points.windows(2) {
            stamp(pair[0], pair[1]);
        }
    }
    Some(StrokeMask { rect, bits })
}

/// The shape payload of an annotation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AnnotationKind {
    Rectangle {
        rect: Rect,
    },
    Ellipse {
        rect: Rect,
    },
    Arrow {
        start: Point,
        end: Point,
    },
    Line {
        start: Point,
        end: Point,
    },
    Pencil {
        points: Vec<Point>,
    },
    Marker {
        points: Vec<Point>,
    },
    Text {
        position: Point,
        text: String,
        size: u32,
    },
    Number {
        position: Point,
        value: u32,
    },
    Mosaic {
        points: Vec<Point>,
    },
    Blur {
        points: Vec<Point>,
    },
    Eraser {
        points: Vec<Point>,
    },
    Magnifier {
        center: Point,
        radius: u32,
        zoom: u32,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum HistoryAction {
    Add(Annotation),
}

/// Committed annotations plus undo/redo history.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct AnnotationDocument {
    items: Vec<Annotation>,
    undo: Vec<HistoryAction>,
    redo: Vec<HistoryAction>,
}

impl AnnotationDocument {
    #[must_use]
    pub fn items(&self) -> &[Annotation] {
        &self.items
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    #[must_use]
    pub fn can_undo(&self) -> bool {
        !self.undo.is_empty()
    }

    #[must_use]
    pub fn can_redo(&self) -> bool {
        !self.redo.is_empty()
    }

    pub fn add(&mut self, annotation: Annotation) {
        self.items.push(annotation.clone());
        self.undo.push(HistoryAction::Add(annotation));
        self.redo.clear();
    }

    pub fn undo(&mut self) -> bool {
        let Some(action) = self.undo.pop() else {
            return false;
        };
        match &action {
            HistoryAction::Add(_) => {
                self.items.pop();
            }
        }
        self.redo.push(action);
        true
    }

    pub fn redo(&mut self) -> bool {
        let Some(action) = self.redo.pop() else {
            return false;
        };
        match &action {
            HistoryAction::Add(annotation) => self.items.push(annotation.clone()),
        }
        self.undo.push(action);
        true
    }

    pub fn clear(&mut self) {
        self.items.clear();
        self.undo.clear();
        self.redo.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::{Annotation, AnnotationDocument, AnnotationKind, Tool, points_bounds, stroke_mask};
    use crate::geometry::{Point, Rect};

    fn rect_annotation(x: i32, y: i32) -> Annotation {
        Annotation::new(
            AnnotationKind::Rectangle {
                rect: Rect::new(x, y, 20, 10),
            },
            [255, 0, 0, 255],
            2,
        )
    }

    #[test]
    fn undo_and_redo_additions() {
        let mut doc = AnnotationDocument::default();
        assert!(!doc.can_undo());
        doc.add(rect_annotation(10, 10));
        doc.add(rect_annotation(40, 40));
        assert_eq!(doc.items().len(), 2);
        assert!(doc.undo());
        assert_eq!(doc.items().len(), 1);
        assert!(doc.undo());
        assert!(doc.items().is_empty());
        assert!(!doc.undo());
        assert!(doc.redo());
        assert!(doc.redo());
        assert_eq!(doc.items().len(), 2);
        assert!(!doc.redo());
    }

    #[test]
    fn line_hit_test_uses_distance() {
        let annotation = Annotation::new(
            AnnotationKind::Line {
                start: Point::new(0, 0),
                end: Point::new(100, 100),
            },
            [255, 0, 0, 255],
            2,
        );
        assert!(annotation.hit_test(Point::new(50, 50)));
        assert!(!annotation.hit_test(Point::new(50, 90)));
    }

    #[test]
    fn stroke_mask_covers_the_brush_radius() {
        let mask = stroke_mask(
            &[Point::new(5, 5), Point::new(15, 5)],
            3,
            Rect::new(0, 0, 20, 20),
        )
        .unwrap();
        assert!(mask.contains(10, 5));
        assert!(mask.contains(10, 7));
        assert!(!mask.contains(10, 10));
        assert!(mask.any_in(Rect::new(9, 4, 2, 2)));
    }

    #[test]
    fn tool_metadata_is_consistent() {
        assert_eq!(Tool::ALL.len(), 13);
        assert!(Tool::Rectangle.is_draw_tool());
        assert!(!Tool::Select.is_draw_tool());
        assert!(Tool::Eraser.is_draw_tool());
        assert!(Tool::Mosaic.is_draw_tool());
        assert!(Tool::Rectangle.is_rect_drag_tool());
        assert!(!Tool::Mosaic.is_rect_drag_tool());
        assert!(points_bounds(&[Point::new(5, 7), Point::new(2, 9)]) == Rect::new(2, 7, 3, 2));
    }
}
