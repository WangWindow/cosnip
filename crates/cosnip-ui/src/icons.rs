//! Original line icons for the Cosnip toolbar.
//!
//! These are drawn with egui primitives so the UI stays crisp at any scale and ships no copied
//! raster assets. The visual language is intentionally close to a compact desktop snipping tool:
//! 24-unit line icons, rounded joins, monochrome glyphs, and a highlighted active state.

use eframe::egui::{self, Color32, Pos2, Rect, Shape, Stroke, Vec2};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Icon {
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
    Undo,
    Redo,
    Copy,
    Save,
    Pin,
    Cancel,
    Stroke(u32),
}

pub fn paint(painter: &egui::Painter, rect: Rect, icon: Icon, color: Color32) {
    let painter = painter.with_clip_rect(rect);
    let width = (rect.width() * 0.09).clamp(1.2, 2.0);
    let stroke = Stroke::new(width, color);
    match icon {
        Icon::Select => {
            let points = [
                (5.0, 3.0),
                (5.0, 19.0),
                (9.0, 15.0),
                (12.0, 22.0),
                (15.0, 20.5),
                (12.0, 14.0),
                (18.0, 14.0),
            ];
            polyline(&painter, rect, &points, stroke, true);
        }
        Icon::Rectangle => {
            let rect = map_rect(rect, 4.0, 5.0, 20.0, 19.0);
            painter.rect_stroke(rect, 0.0, stroke, egui::StrokeKind::Inside);
        }
        Icon::Ellipse => {
            let center = map(rect, 12.0, 12.0);
            painter.add(Shape::ellipse_stroke(
                center,
                Vec2::new(rect.width() * 8.0 / 24.0, rect.height() * 6.0 / 24.0),
                stroke,
            ));
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
        Icon::Line => {
            line(&painter, rect, (5.0, 19.0), (19.0, 5.0), stroke);
            painter.circle_filled(map(rect, 5.0, 19.0), width, color);
            painter.circle_filled(map(rect, 19.0, 5.0), width, color);
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
            line(&painter, rect, (8.0, 20.0), (16.0, 20.0), stroke);
        }
        Icon::Number => {
            painter.circle_stroke(map(rect, 12.0, 12.0), rect.width() * 6.5 / 24.0, stroke);
            polyline(
                &painter,
                rect,
                &[(10.0, 9.0), (12.0, 7.0), (12.0, 17.0)],
                stroke,
                false,
            );
            line(&painter, rect, (9.0, 17.0), (15.0, 17.0), stroke);
        }
        Icon::Mosaic => {
            for gy in 0..3 {
                for gx in 0..3 {
                    if (gx + gy) % 2 == 0 {
                        let x = 4.0 + gx as f32 * 6.0;
                        let y = 5.0 + gy as f32 * 5.0;
                        let cell_rect = map_rect(rect, x, y, x + 5.0, y + 4.0);
                        painter.rect_filled(cell_rect, 0.0, color);
                    }
                }
            }
        }
        Icon::Blur => {
            let lines = [
                (7.0, 5.0, 14.0),
                (5.0, 9.0, 17.0),
                (7.0, 13.0, 15.0),
                (5.0, 17.0, 12.0),
            ];
            for (start, y, end) in lines {
                line(&painter, rect, (start, y), (end, y), stroke);
                painter.circle_filled(map(rect, end + 1.0, y), width, color);
            }
        }
        Icon::Magnifier => {
            painter.circle_stroke(map(rect, 10.0, 10.0), rect.width() * 5.0 / 24.0, stroke);
            line(&painter, rect, (14.0, 14.0), (20.0, 20.0), stroke);
        }
        Icon::Eraser => {
            polyline(
                &painter,
                rect,
                &[
                    (5.0, 17.0),
                    (13.0, 7.0),
                    (20.0, 13.0),
                    (12.0, 22.0),
                    (5.0, 17.0),
                ],
                stroke,
                false,
            );
            line(&painter, rect, (9.0, 13.0), (16.0, 19.0), stroke);
        }
        Icon::Undo => arc_arrow(&painter, rect, false, stroke),
        Icon::Redo => arc_arrow(&painter, rect, true, stroke),
        Icon::Copy => {
            let back = map_rect(rect, 4.0, 4.0, 15.0, 15.0);
            let front = map_rect(rect, 9.0, 9.0, 20.0, 20.0);
            painter.rect_stroke(back, 1.0, stroke, egui::StrokeKind::Inside);
            painter.rect_filled(front, 1.0, Color32::TRANSPARENT);
            painter.rect_stroke(front, 1.0, stroke, egui::StrokeKind::Inside);
        }
        Icon::Save => {
            line(&painter, rect, (12.0, 3.0), (12.0, 15.0), stroke);
            polyline(
                &painter,
                rect,
                &[(8.0, 11.0), (12.0, 15.0), (16.0, 11.0)],
                stroke,
                false,
            );
            polyline(
                &painter,
                rect,
                &[(4.0, 16.0), (4.0, 21.0), (20.0, 21.0), (20.0, 16.0)],
                stroke,
                false,
            );
        }
        Icon::Pin => {
            painter.circle_filled(map(rect, 9.0, 8.0), rect.width() * 2.8 / 24.0, color);
            line(&painter, rect, (10.0, 10.0), (17.0, 17.0), stroke);
            line(&painter, rect, (14.0, 20.0), (20.0, 14.0), stroke);
        }
        Icon::Cancel => {
            line(&painter, rect, (6.0, 6.0), (18.0, 18.0), stroke);
            line(&painter, rect, (18.0, 6.0), (6.0, 18.0), stroke);
        }
        Icon::Stroke(thickness) => {
            let width = (thickness as f32 * 0.65).clamp(1.0, 4.0);
            let stroke = Stroke::new(width, color);
            line(&painter, rect, (4.0, 12.0), (20.0, 12.0), stroke);
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
        let head = radius * 0.65;
        painter.line_segment([tip, tip + dir * head + normal * head * 0.7], stroke);
        painter.line_segment([tip, tip + dir * head - normal * head * 0.7], stroke);
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
                Icon::Select,
                Icon::Rectangle,
                Icon::Ellipse,
                Icon::Arrow,
                Icon::Line,
                Icon::Pencil,
                Icon::Marker,
                Icon::Text,
                Icon::Number,
                Icon::Mosaic,
                Icon::Blur,
                Icon::Magnifier,
                Icon::Eraser,
                Icon::Undo,
                Icon::Redo,
                Icon::Copy,
                Icon::Save,
                Icon::Pin,
                Icon::Cancel,
                Icon::Stroke(1),
                Icon::Stroke(3),
                Icon::Stroke(6),
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
