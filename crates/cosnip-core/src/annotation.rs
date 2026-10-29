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
        )
    }

    #[must_use]
    pub const fn is_rect_drag_tool(self) -> bool {
        matches!(
            self,
            Self::Rectangle | Self::Ellipse | Self::Mosaic | Self::Blur
        )
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
            AnnotationKind::Rectangle { rect }
            | AnnotationKind::Ellipse { rect }
            | AnnotationKind::Mosaic { rect }
            | AnnotationKind::Blur { rect } => *rect,
            AnnotationKind::Arrow { start, end } | AnnotationKind::Line { start, end } => {
                Rect::from_points(*start, *end)
            }
            AnnotationKind::Pencil { points } | AnnotationKind::Marker { points } => {
                points_bounds(points)
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
        rect: Rect,
    },
    Blur {
        rect: Rect,
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
    Remove {
        index: usize,
        annotation: Annotation,
    },
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

    /// Remove the topmost annotation under `point`. Returns true when something was erased.
    pub fn remove_at(&mut self, point: Point) -> bool {
        let Some(index) = self
            .items
            .iter()
            .rposition(|annotation| annotation.hit_test(point))
        else {
            return false;
        };
        let annotation = self.items.remove(index);
        self.undo.push(HistoryAction::Remove { index, annotation });
        self.redo.clear();
        true
    }

    pub fn undo(&mut self) -> bool {
        let Some(action) = self.undo.pop() else {
            return false;
        };
        match &action {
            HistoryAction::Add(_) => {
                self.items.pop();
            }
            HistoryAction::Remove { index, annotation } => {
                let index = (*index).min(self.items.len());
                self.items.insert(index, annotation.clone());
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
            HistoryAction::Remove { index, .. } => {
                if *index < self.items.len() {
                    self.items.remove(*index);
                }
            }
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
    use super::{Annotation, AnnotationDocument, AnnotationKind, Tool, points_bounds};
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
    fn erasing_is_undoable() {
        let mut doc = AnnotationDocument::default();
        doc.add(rect_annotation(10, 10));
        doc.add(rect_annotation(40, 40));
        assert!(doc.remove_at(Point::new(45, 45)));
        assert_eq!(doc.items().len(), 1);
        assert!(doc.undo());
        assert_eq!(doc.items().len(), 2);
        assert!(doc.redo());
        assert_eq!(doc.items().len(), 1);
        assert!(!doc.remove_at(Point::new(500, 500)));
    }

    #[test]
    fn eraser_prefers_the_topmost_item() {
        let mut doc = AnnotationDocument::default();
        doc.add(rect_annotation(10, 10));
        doc.add(rect_annotation(10, 10));
        assert!(doc.remove_at(Point::new(15, 15)));
        assert_eq!(doc.items().len(), 1);
        assert_eq!(doc.items()[0], rect_annotation(10, 10));
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
    fn tool_metadata_is_consistent() {
        assert_eq!(Tool::ALL.len(), 13);
        assert!(Tool::Rectangle.is_draw_tool());
        assert!(!Tool::Select.is_draw_tool());
        assert!(!Tool::Eraser.is_draw_tool());
        assert!(Tool::Mosaic.is_rect_drag_tool());
        assert!(points_bounds(&[Point::new(5, 7), Point::new(2, 9)]) == Rect::new(2, 7, 3, 2));
    }
}
