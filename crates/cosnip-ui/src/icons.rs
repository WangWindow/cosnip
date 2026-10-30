//! Original line icons for the Cosnip toolbar.
//!
//! These are drawn with egui primitives so the UI stays crisp at any scale and ships no copied
//! raster assets. The visual language is intentionally close to a compact desktop snipping tool:
//! 24-unit line icons, rounded joins, monochrome glyphs, and a highlighted active state.

use eframe::egui::{self, Color32, Pos2, Rect, Shape, Stroke, Vec2};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Icon {
    Shapes,
    Arrow,
    Pencil,
    Marker,
    Text,
    Mosaic,
    Eraser,
    Undo,
    Redo,
    Copy,
    Save,
    Pin,
    Cancel,
}

pub fn paint(painter: &egui::Painter, rect: Rect, icon: Icon, color: Color32) {
    let painter = painter.with_clip_rect(rect);
    let width = (rect.width() * 0.09).clamp(1.2, 2.0);
    let stroke = Stroke::new(width, color);
    match icon {
        Icon::Shapes => {
            let shape = map_rect(rect, 2.5, 7.0, 11.5, 20.0);
            painter.rect_stroke(shape, 1.0, stroke, egui::StrokeKind::Inside);
            painter.add(Shape::ellipse_stroke(
                map(rect, 17.0, 13.0),
                Vec2::new(rect.width() * 4.3 / 24.0, rect.height() * 3.5 / 24.0),
                stroke,
            ));
            painter.circle_filled(map(rect, 20.5, 5.5), 1.8, Color32::from_rgb(235, 55, 55));
        }
        Icon::Arrow => {
            line(&painter, rect, (5.0, 19.0), (19.0, 5.0), stroke);
            polyline(
                &painter,
                rect,
                &[(14.0, 5.0), (19.0, 5.0), (19.0, 10.0)],
                stroke,
                false,
            );
        }
        Icon::Pencil => {
            polyline(
                &painter,
                rect,
                &[
                    (5.0, 19.0),
                    (7.0, 12.0),
                    (16.0, 3.0),
                    (21.0, 8.0),
                    (12.0, 17.0),
                    (5.0, 19.0),
                ],
                stroke,
                false,
            );
            line(&painter, rect, (7.0, 12.0), (12.0, 17.0), stroke);
            line(&painter, rect, (5.0, 19.0), (4.0, 22.0), stroke);
        }
        Icon::Marker => {
            let nib = Stroke::new((width * 3.0).max(3.0), color);
            line(&painter, rect, (6.0, 18.0), (16.0, 8.0), nib);
            polyline(
                &painter,
                rect,
                &[(6.0, 18.0), (9.0, 21.0), (20.0, 10.0), (16.0, 6.0)],
                stroke,
                false,
            );
        }
        Icon::Text => {
            line(&painter, rect, (5.0, 5.0), (19.0, 5.0), stroke);
            line(&painter, rect, (12.0, 5.0), (12.0, 20.0), stroke);
        }
        Icon::Mosaic => {
            painter.rect_stroke(
                map_rect(rect, 4.0, 4.0, 20.0, 20.0),
                0.5,
                stroke,
                egui::StrokeKind::Inside,
            );
            for (gx, gy) in [(0, 0), (1, 1)] {
                let x = 4.0 + gx as f32 * 8.0;
                let y = 4.0 + gy as f32 * 8.0;
                let cell_rect = map_rect(rect, x, y, x + 8.0, y + 8.0);
                painter.rect_filled(cell_rect, 0.0, color);
            }
        }
        Icon::Eraser => {
            polyline(
                &painter,
                rect,
                &[(4.0, 16.0), (13.0, 5.0), (21.0, 12.0), (12.0, 22.0)],
                stroke,
                true,
            );
            line(&painter, rect, (8.5, 11.5), (17.0, 18.0), stroke);
        }
        Icon::Undo => arc_arrow(&painter, rect, false, Stroke::new(width * 1.3, color)),
        Icon::Redo => arc_arrow(&painter, rect, true, Stroke::new(width * 1.3, color)),
        Icon::Copy => {
            let back = map_rect(rect, 4.0, 4.0, 15.0, 15.0);
            let front = map_rect(rect, 9.0, 9.0, 20.0, 20.0);
            painter.rect_stroke(back, 1.0, stroke, egui::StrokeKind::Inside);
            painter.rect_filled(front, 1.0, Color32::TRANSPARENT);
            painter.rect_stroke(front, 1.0, stroke, egui::StrokeKind::Inside);
        }
        Icon::Save => {
            painter.rect_stroke(
                map_rect(rect, 4.0, 4.0, 20.0, 20.0),
                1.0,
                stroke,
                egui::StrokeKind::Inside,
            );
            painter.rect_stroke(
                map_rect(rect, 8.0, 4.0, 16.0, 10.0),
                0.5,
                stroke,
                egui::StrokeKind::Inside,
            );
            painter.rect_stroke(
                map_rect(rect, 7.0, 13.0, 17.0, 20.0),
                0.5,
                stroke,
                egui::StrokeKind::Inside,
            );
        }
        Icon::Pin => {
            painter.circle_filled(map(rect, 15.0, 8.0), rect.width() * 3.1 / 24.0, color);
            line(&painter, rect, (13.5, 10.5), (6.5, 18.0), stroke);
            line(&painter, rect, (9.5, 11.5), (13.0, 15.5), stroke);
        }
        Icon::Cancel => {
            line(&painter, rect, (5.0, 5.0), (19.0, 19.0), stroke);
            line(&painter, rect, (19.0, 5.0), (5.0, 19.0), stroke);
        }
    }
}

fn arc_arrow(painter: &egui::Painter, rect: Rect, mirror: bool, stroke: Stroke) {
    let center = map(rect, 12.0, 13.0);
    let radius = rect.width() * 7.0 / 24.0;
    let mut points = Vec::with_capacity(18);
    let direction = if mirror { -1.0 } else { 1.0 };
    for step in 0..=16 {
        let angle = std::f32::consts::PI * (1.05 + 0.95 * step as f32 / 16.0) * direction;
        points.push(center + Vec2::new(radius * angle.cos(), radius * angle.sin()));
    }
    painter.add(Shape::line(points.clone(), stroke));
    if let Some(&tip) = points.first() {
        let tangent = if mirror { -1.0 } else { 1.0 };
        let a = std::f32::consts::PI * 1.05 * direction;
        let dir = Vec2::new(-a.sin() * tangent, a.cos() * tangent);
        let normal = Vec2::new(-dir.y, dir.x);
        let head = radius * 0.85;
        let base = tip + dir * head;
        painter.add(Shape::convex_polygon(
            vec![tip, base + normal * head * 0.8, base - normal * head * 0.8],
            stroke.color,
            Stroke::NONE,
        ));
    }
}

fn map(rect: Rect, x: f32, y: f32) -> Pos2 {
    rect.min + Vec2::new(rect.width() * x / 24.0, rect.height() * y / 24.0)
}

fn map_rect(rect: Rect, x0: f32, y0: f32, x1: f32, y1: f32) -> Rect {
    Rect::from_min_max(map(rect, x0, y0), map(rect, x1, y1))
}

fn line(painter: &egui::Painter, rect: Rect, a: (f32, f32), b: (f32, f32), stroke: Stroke) {
    painter.line_segment([map(rect, a.0, a.1), map(rect, b.0, b.1)], stroke);
}

fn polyline(
    painter: &egui::Painter,
    rect: Rect,
    points: &[(f32, f32)],
    stroke: Stroke,
    closed: bool,
) {
    let mut mapped = points
        .iter()
        .map(|(x, y)| map(rect, *x, *y))
        .collect::<Vec<_>>();
    if closed && !mapped.is_empty() {
        mapped.push(mapped[0]);
    }
    painter.add(Shape::line(mapped, stroke));
}

#[cfg(test)]
mod tests {
    use super::{Icon, paint};
    use eframe::egui::{self, Color32, Pos2, Rect, Vec2};

    #[test]
    fn every_icon_draws() {
        let ctx = egui::Context::default();
        let mut output = ctx.run_ui(egui::RawInput::default(), |ui| {
            let painter = ui.ctx().layer_painter(egui::LayerId::background());
            let icons = [
                Icon::Shapes,
                Icon::Arrow,
                Icon::Pencil,
                Icon::Marker,
                Icon::Text,
                Icon::Mosaic,
                Icon::Eraser,
                Icon::Undo,
                Icon::Redo,
                Icon::Copy,
                Icon::Save,
                Icon::Pin,
                Icon::Cancel,
            ];
            for (index, icon) in icons.into_iter().enumerate() {
                let rect =
                    Rect::from_min_size(Pos2::new(index as f32 * 26.0, 0.0), Vec2::splat(24.0));
                paint(&painter, rect, icon, Color32::WHITE);
            }
        });
        output.textures_delta.clear();
    }
}
