//! Software rasterization of annotations onto a captured frame.
//!
//! The overlay preview uses egui primitives for latency; copied/saved/pinned output uses this
//! renderer so annotations are real pixels. Text uses the system font database through
//! `cosmic-text` (including CJK fallback).

use cosmic_text::{Attrs, Buffer, Color as TextColor, FontSystem, Metrics, Shaping, SwashCache};
use cosnip_core::{Annotation, AnnotationKind, Frame, Point, Rect};

use crate::ImagingError;

/// Crop `selection` and draw `annotations` into the result.
pub fn render(
    frame: &Frame,
    selection: Rect,
    annotations: &[Annotation],
) -> Result<Frame, ImagingError> {
    let base = crate::crop(frame, selection)?;
    let mut canvas = Canvas::from_frame(&base, selection);
    for annotation in annotations {
        draw_annotation(&mut canvas, &base, annotation);
    }
    Frame::new(canvas.width, canvas.height, canvas.rgba)
        .map_err(|e| ImagingError::Encode(e.to_string()))
}

struct Canvas {
    origin: Point,
    width: u32,
    height: u32,
    rgba: Vec<u8>,
}

impl Canvas {
    fn from_frame(frame: &Frame, origin: Rect) -> Self {
        Self {
            origin: Point::new(origin.x, origin.y),
            width: frame.width(),
            height: frame.height(),
            rgba: frame.rgba().to_vec(),
        }
    }

    fn pixel(&self, x: i32, y: i32) -> Option<[u8; 4]> {
        let x = x - self.origin.x;
        let y = y - self.origin.y;
        if x < 0 || y < 0 || x >= self.width as i32 || y >= self.height as i32 {
            return None;
        }
        let index = (y as usize * self.width as usize + x as usize) * 4;
        Some([
            self.rgba[index],
            self.rgba[index + 1],
            self.rgba[index + 2],
            self.rgba[index + 3],
        ])
    }

    fn blend(&mut self, x: i32, y: i32, color: [u8; 4], coverage: u8) {
        let x = x - self.origin.x;
        let y = y - self.origin.y;
        if x < 0 || y < 0 || x >= self.width as i32 || y >= self.height as i32 {
            return;
        }
        let alpha = u16::from(color[3]) * u16::from(coverage) / 255;
        if alpha == 0 {
            return;
        }
        let index = (y as usize * self.width as usize + x as usize) * 4;
        let inverse = 255 - alpha;
        for (dst, src) in self.rgba[index..index + 3]
            .iter_mut()
            .zip(color.iter().take(3))
        {
            let old = u16::from(*dst);
            let src = u16::from(*src);
            *dst = ((src * alpha + old * inverse) / 255) as u8;
        }
        self.rgba[index + 3] = 255;
    }

    fn set(&mut self, x: i32, y: i32, color: [u8; 4]) {
        let x = x - self.origin.x;
        let y = y - self.origin.y;
        if x < 0 || y < 0 || x >= self.width as i32 || y >= self.height as i32 {
            return;
        }
        let index = (y as usize * self.width as usize + x as usize) * 4;
        self.rgba[index..index + 4].copy_from_slice(&color);
    }

    fn global_bounds(&self) -> Rect {
        Rect::new(self.origin.x, self.origin.y, self.width, self.height)
    }
}

fn draw_annotation(canvas: &mut Canvas, base: &Frame, annotation: &Annotation) {
    let color = annotation.color;
    let width = annotation.width.max(1) as i32;
    match &annotation.kind {
        AnnotationKind::Rectangle { rect } => {
            draw_rect_stroke(canvas, *rect, color, width);
        }
        AnnotationKind::Ellipse { rect } => {
            draw_ellipse_stroke(canvas, *rect, color, width);
        }
        AnnotationKind::Arrow { start, end } => {
            draw_line(canvas, *start, *end, color, width, 255);
            draw_arrow_head(canvas, *start, *end, color, width);
        }
        AnnotationKind::Line { start, end } => {
            draw_line(canvas, *start, *end, color, width, 255);
        }
        AnnotationKind::Pencil { points } => {
            draw_polyline(canvas, points, color, width, 255);
        }
        AnnotationKind::Marker { points } => {
            draw_polyline(canvas, points, color, (width * 4).max(4), 96);
        }
        AnnotationKind::Text {
            position,
            text,
            size,
        } => {
            draw_text(canvas, *position, text, *size, color);
        }
        AnnotationKind::Number { position, value } => {
            draw_number(canvas, *position, *value, color, width);
        }
        AnnotationKind::Mosaic { rect } => {
            apply_mosaic(canvas, *rect, (width * 4).max(8) as u32);
        }
        AnnotationKind::Blur { rect } => {
            apply_blur(canvas, *rect, (width * 2).max(2) as u32);
        }
        AnnotationKind::Magnifier {
            center,
            radius,
            zoom,
        } => {
            draw_magnifier(canvas, base, *center, *radius, *zoom, color, width);
        }
    }
}

fn draw_rect_stroke(canvas: &mut Canvas, rect: Rect, color: [u8; 4], width: i32) {
    if rect.is_empty() {
        return;
    }
    let left = rect.x;
    let top = rect.y;
    let right = rect.right().saturating_sub(1);
    let bottom = rect.bottom().saturating_sub(1);
    draw_line(
        canvas,
        Point::new(left, top),
        Point::new(right, top),
        color,
        width,
        255,
    );
    draw_line(
        canvas,
        Point::new(right, top),
        Point::new(right, bottom),
        color,
        width,
        255,
    );
    draw_line(
        canvas,
        Point::new(right, bottom),
        Point::new(left, bottom),
        color,
        width,
        255,
    );
    draw_line(
        canvas,
        Point::new(left, bottom),
        Point::new(left, top),
        color,
        width,
        255,
    );
}

fn draw_ellipse_stroke(canvas: &mut Canvas, rect: Rect, color: [u8; 4], width: i32) {
    if rect.is_empty() {
        return;
    }
    let cx = rect.x as f32 + rect.width as f32 / 2.0;
    let cy = rect.y as f32 + rect.height as f32 / 2.0;
    let rx = (rect.width as f32 / 2.0).max(0.5);
    let ry = (rect.height as f32 / 2.0).max(0.5);
    let steps = ((rx.max(ry) * 2.0) as i32).clamp(32, 720);
    let mut previous = Point::new(rect.right(), (cy.round()) as i32);
    for step in 1..=steps {
        let angle = std::f32::consts::TAU * step as f32 / steps as f32;
        let point = Point::new(
            (cx + rx * angle.cos()).round() as i32,
            (cy + ry * angle.sin()).round() as i32,
        );
        draw_line(canvas, previous, point, color, width, 255);
        previous = point;
    }
}

fn draw_arrow_head(canvas: &mut Canvas, start: Point, end: Point, color: [u8; 4], width: i32) {
    let angle = ((end.y - start.y) as f32).atan2((end.x - start.x) as f32);
    let length = (12.0 + width as f32 * 2.0).min(40.0);
    let spread = 0.5;
    let left = Point::new(
        (end.x as f32 - length * (angle - spread).cos()).round() as i32,
        (end.y as f32 - length * (angle - spread).sin()).round() as i32,
    );
    let right = Point::new(
        (end.x as f32 - length * (angle + spread).cos()).round() as i32,
        (end.y as f32 - length * (angle + spread).sin()).round() as i32,
    );
    draw_line(canvas, end, left, color, width, 255);
    draw_line(canvas, end, right, color, width, 255);
}

fn draw_polyline(canvas: &mut Canvas, points: &[Point], color: [u8; 4], width: i32, alpha: u8) {
    if points.len() == 1 {
        draw_disc(canvas, points[0], width, color, alpha);
    }
    for pair in points.windows(2) {
        draw_line(canvas, pair[0], pair[1], color, width, alpha);
    }
}

fn draw_line(canvas: &mut Canvas, start: Point, end: Point, color: [u8; 4], width: i32, alpha: u8) {
    let dx = end.x - start.x;
    let dy = end.y - start.y;
    let steps = dx.abs().max(dy.abs()).max(1);
    let radius = (width / 2).max(0);
    for step in 0..=steps {
        let x = start.x + dx * step / steps;
        let y = start.y + dy * step / steps;
        draw_disc(canvas, Point::new(x, y), radius, color, alpha);
    }
}

fn draw_disc(canvas: &mut Canvas, center: Point, radius: i32, color: [u8; 4], alpha: u8) {
    if radius <= 0 {
        canvas.blend(center.x, center.y, color, alpha);
        return;
    }
    let radius_squared = radius * radius;
    for dy in -radius..=radius {
        for dx in -radius..=radius {
            if dx * dx + dy * dy <= radius_squared {
                canvas.blend(center.x + dx, center.y + dy, color, alpha);
            }
        }
    }
}

fn draw_number(canvas: &mut Canvas, position: Point, value: u32, color: [u8; 4], width: i32) {
    let radius = (14 + width).clamp(12, 24);
    draw_disc(canvas, position, radius, color, 255);
    let label = value.to_string();
    let size = (radius as u32 * 2).max(14);
    let text_width = label.chars().count() as i32 * (size as i32 * 3 / 5);
    let origin = Point::new(
        position.x - text_width / 2,
        position.y - size as i32 / 2 - 1,
    );
    draw_text_with(canvas, origin, &label, size, [255, 255, 255, 255], None);
}

fn draw_text(canvas: &mut Canvas, position: Point, text: &str, size: u32, color: [u8; 4]) {
    draw_text_with(canvas, position, text, size.max(6), color, None);
}

fn draw_text_with(
    canvas: &mut Canvas,
    position: Point,
    text: &str,
    size: u32,
    color: [u8; 4],
    max_width: Option<f32>,
) {
    if text.is_empty() {
        return;
    }
    let mut font_system = FontSystem::new();
    let mut swash_cache = SwashCache::new();
    let metrics = Metrics::new(size as f32, (size as f32 * 1.25).max(1.0));
    let mut buffer = Buffer::new(&mut font_system, metrics);
    buffer.set_size(max_width, None);
    buffer.set_text(text, &Attrs::new(), Shaping::Advanced, None);
    let text_color = TextColor::rgba(color[0], color[1], color[2], color[3]);
    buffer.draw(
        &mut font_system,
        &mut swash_cache,
        text_color,
        |x, y, width, height, color| {
            for gy in 0..height as i32 {
                for gx in 0..width as i32 {
                    let coverage = color.a();
                    if coverage == 0 {
                        continue;
                    }
                    let source = [color.r(), color.g(), color.b(), 255];
                    canvas.blend(position.x + x + gx, position.y + y + gy, source, coverage);
                }
            }
        },
    );
}

fn apply_mosaic(canvas: &mut Canvas, rect: Rect, block: u32) {
    let rect = match rect.clamp_to(canvas.global_bounds()) {
        Some(rect) => rect,
        None => return,
    };
    let block = block.max(2) as i32;
    let mut y = rect.y;
    while y < rect.bottom() {
        let mut x = rect.x;
        let block_height = block.min(rect.bottom() - y);
        while x < rect.right() {
            let block_width = block.min(rect.right() - x);
            let mut sum = [0u32; 4];
            let mut count = 0u32;
            for py in y..y + block_height {
                for px in x..x + block_width {
                    if let Some(pixel) = canvas.pixel(px, py) {
                        for channel in 0..4 {
                            sum[channel] += u32::from(pixel[channel]);
                        }
                        count += 1;
                    }
                }
            }
            if let Some(count) = std::num::NonZeroU32::new(count) {
                let count = count.get();
                let color = [
                    (sum[0] / count) as u8,
                    (sum[1] / count) as u8,
                    (sum[2] / count) as u8,
                    (sum[3] / count) as u8,
                ];
                for py in y..y + block_height {
                    for px in x..x + block_width {
                        canvas.set(px, py, color);
                    }
                }
            }
            x += block;
        }
        y += block;
    }
}

fn apply_blur(canvas: &mut Canvas, rect: Rect, radius: u32) {
    let rect = match rect.clamp_to(canvas.global_bounds()) {
        Some(rect) => rect,
        None => return,
    };
    let radius = radius.max(1) as i32;
    let source = canvas.rgba.clone();
    let width = canvas.width as i32;
    let origin_x = canvas.origin.x;
    let origin_y = canvas.origin.y;
    for y in rect.y..rect.bottom() {
        for x in rect.x..rect.right() {
            let mut sum = [0u32; 4];
            let mut count = 0u32;
            for dy in -radius..=radius {
                for dx in -radius..=radius {
                    let sx = x + dx;
                    let sy = y + dy;
                    if sx < rect.x || sy < rect.y || sx >= rect.right() || sy >= rect.bottom() {
                        continue;
                    }
                    let index =
                        ((sy - origin_y) as usize * width as usize + (sx - origin_x) as usize) * 4;
                    for channel in 0..4 {
                        sum[channel] += u32::from(source[index + channel]);
                    }
                    count += 1;
                }
            }
            if let Some(count) = std::num::NonZeroU32::new(count) {
                let index =
                    ((y - origin_y) as usize * width as usize + (x - origin_x) as usize) * 4;
                for (channel, total) in sum.iter().enumerate() {
                    canvas.rgba[index + channel] = (total / count.get()) as u8;
                }
            }
        }
    }
}

fn draw_magnifier(
    canvas: &mut Canvas,
    base: &Frame,
    center: Point,
    radius: u32,
    zoom: u32,
    color: [u8; 4],
    width: i32,
) {
    let radius = radius.max(8) as i32;
    let zoom = zoom.max(1) as f32;
    let diameter = radius * 2;
    for dy in -radius..=radius {
        for dx in -radius..=radius {
            if dx * dx + dy * dy > radius * radius {
                continue;
            }
            let source_x = center.x + (dx as f32 / zoom).round() as i32;
            let source_y = center.y + (dy as f32 / zoom).round() as i32;
            let pixel = base
                .pixel(source_x - canvas.origin.x, source_y - canvas.origin.y)
                .or_else(|| canvas.pixel(center.x + dx, center.y + dy))
                .unwrap_or([0, 0, 0, 255]);
            canvas.set(center.x + dx, center.y + dy, pixel);
        }
    }
    let steps = (diameter * 2).clamp(32, 720);
    let mut previous = Point::new(center.x + radius, center.y);
    for step in 1..=steps {
        let angle = std::f32::consts::TAU * step as f32 / steps as f32;
        let point = Point::new(
            center.x + (radius as f32 * angle.cos()).round() as i32,
            center.y + (radius as f32 * angle.sin()).round() as i32,
        );
        draw_line(canvas, previous, point, color, width.max(2), 255);
        previous = point;
    }
}

#[cfg(test)]
mod tests {
    use cosnip_core::{Annotation, AnnotationKind, Frame, Point, Rect};

    use super::render;

    fn frame() -> Frame {
        let mut rgba = Vec::new();
        for y in 0..40u8 {
            for x in 0..40u8 {
                rgba.extend_from_slice(&[x * 5, y * 5, 100, 255]);
            }
        }
        Frame::new(40, 40, rgba).unwrap()
    }

    fn annotation(kind: AnnotationKind) -> Annotation {
        Annotation::new(kind, [255, 0, 0, 255], 2)
    }

    #[test]
    fn renders_rectangle_and_clips_to_selection() {
        let frame = frame();
        let out = render(
            &frame,
            Rect::new(10, 10, 20, 20),
            &[annotation(AnnotationKind::Rectangle {
                rect: Rect::new(12, 12, 10, 10),
            })],
        )
        .unwrap();
        assert_eq!((out.width(), out.height()), (20, 20));
        assert_eq!(out.pixel(0, 0), frame.pixel(10, 10));
        assert_eq!(out.pixel(2, 2), Some([255, 0, 0, 255]));
        assert_eq!(out.pixel(12, 12), frame.pixel(22, 22));
    }

    #[test]
    fn mosaic_replaces_the_region_with_block_colors() {
        let frame = frame();
        let out = render(
            &frame,
            Rect::new(0, 0, 40, 40),
            &[annotation(AnnotationKind::Mosaic {
                rect: Rect::new(8, 8, 10, 10),
            })],
        )
        .unwrap();
        let mut colors = std::collections::HashSet::new();
        for y in 8..18 {
            for x in 8..18 {
                colors.insert(out.pixel(x, y));
            }
        }
        assert!(colors.len() <= 4);
        assert_eq!(out.pixel(0, 0), frame.pixel(0, 0));
    }

    #[test]
    fn blur_changes_pixels_without_changing_size() {
        let frame = frame();
        let out = render(
            &frame,
            Rect::new(0, 0, 40, 40),
            &[annotation(AnnotationKind::Blur {
                rect: Rect::new(8, 8, 10, 10),
            })],
        )
        .unwrap();
        assert_eq!((out.width(), out.height()), (40, 40));
        assert_ne!(out.pixel(8, 8), frame.pixel(8, 8));
    }

    #[test]
    fn text_and_number_are_visible() {
        let frame = frame();
        let out = render(
            &frame,
            Rect::new(0, 0, 40, 40),
            &[
                annotation(AnnotationKind::Text {
                    position: Point::new(2, 2),
                    text: "Hi".into(),
                    size: 12,
                }),
                annotation(AnnotationKind::Number {
                    position: Point::new(30, 30),
                    value: 1,
                }),
            ],
        )
        .unwrap();
        assert_eq!((out.width(), out.height()), (40, 40));
        let changed = (0..40)
            .flat_map(|y| (0..40).map(move |x| (x, y)))
            .filter(|(x, y)| out.pixel(*x, *y) != frame.pixel(*x, *y))
            .count();
        assert!(changed > 10, "expected visible text/number pixels");
    }

    #[test]
    fn cjk_text_uses_font_fallback() {
        let frame = frame();
        let out = render(
            &frame,
            Rect::new(0, 0, 40, 40),
            &[annotation(AnnotationKind::Text {
                position: Point::new(2, 2),
                text: "中文".into(),
                size: 12,
            })],
        )
        .unwrap();
        let changed = (0..40)
            .flat_map(|y| (0..40).map(move |x| (x, y)))
            .filter(|(x, y)| out.pixel(*x, *y) != frame.pixel(*x, *y))
            .count();
        assert!(changed > 0, "expected a CJK fallback font to render pixels");
    }

    #[test]
    fn magnifier_samples_from_the_base_frame() {
        let frame = frame();
        let out = render(
            &frame,
            Rect::new(0, 0, 40, 40),
            &[annotation(AnnotationKind::Magnifier {
                center: Point::new(20, 20),
                radius: 8,
                zoom: 2,
            })],
        )
        .unwrap();
        assert_eq!(out.pixel(20, 20), frame.pixel(20, 20));
        assert_ne!(out.pixel(22, 20), frame.pixel(22, 20));
    }
}
