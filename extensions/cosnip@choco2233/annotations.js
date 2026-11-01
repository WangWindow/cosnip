import Pango from 'gi://Pango';
import PangoCairo from 'gi://PangoCairo';
import cairo from 'cairo';

import { paintPixbuf, pixbufPixel } from './util.js';

export const Tool = {
  SELECT: 'select',
  RECT: 'rect',
  ARROW: 'arrow',
  LINE: 'line',
  PENCIL: 'pencil',
  MARKER: 'marker',
  MOSAIC: 'mosaic',
  BLUR: 'blur',
  TEXT: 'text',
  ERASER: 'eraser',
};

export function createAnnotation(tool, point, style) {
  const base = {
    kind: tool,
    color: style.color,
    width: style.width,
  };
  switch (tool) {
    case Tool.RECT:
      return { ...base, x: point.x, y: point.y, w: 0, h: 0 };
    case Tool.ARROW:
    case Tool.LINE:
      return { ...base, x1: point.x, y1: point.y, x2: point.x, y2: point.y };
    case Tool.PENCIL:
    case Tool.MARKER:
    case Tool.MOSAIC:
    case Tool.BLUR:
    case Tool.ERASER:
      return { ...base, points: [point] };
    case Tool.TEXT:
      return { ...base, x: point.x, y: point.y, text: '', size: style.fontSize };
    default:
      return null;
  }
}

export function updateAnnotation(annotation, point, shift) {
  switch (annotation.kind) {
    case Tool.RECT: {
      let x = Math.min(annotation.x, point.x);
      let y = Math.min(annotation.y, point.y);
      let w = Math.abs(point.x - annotation.x);
      let h = Math.abs(point.y - annotation.y);
      if (shift) {
        const side = Math.max(w, h);
        w = side;
        h = side;
        if (point.x < annotation.x)
          x = annotation.x - side;
        if (point.y < annotation.y)
          y = annotation.y - side;
      }
      annotation.x = x;
      annotation.y = y;
      annotation.w = w;
      annotation.h = h;
      break;
    }
    case Tool.ARROW:
    case Tool.LINE:
      annotation.x2 = point.x;
      annotation.y2 = point.y;
      break;
    case Tool.PENCIL:
    case Tool.MARKER:
    case Tool.MOSAIC:
    case Tool.BLUR:
    case Tool.ERASER: {
      const last = annotation.points[annotation.points.length - 1];
      if (!last || Math.hypot(point.x - last.x, point.y - last.y) >= 1)
        annotation.points.push(point);
      break;
    }
    default:
      break;
  }
}

export function annotationIsValid(annotation) {
  if (!annotation)
    return false;
  switch (annotation.kind) {
    case Tool.RECT:
      return annotation.w >= 1 && annotation.h >= 1;
    case Tool.ARROW:
    case Tool.LINE: {
      const dx = annotation.x2 - annotation.x1;
      const dy = annotation.y2 - annotation.y1;
      return Math.hypot(dx, dy) >= 2;
    }
    case Tool.PENCIL:
    case Tool.MARKER:
    case Tool.MOSAIC:
    case Tool.BLUR:
    case Tool.ERASER:
      return annotation.points.length >= 2;
    case Tool.TEXT:
      return annotation.text.trim().length > 0;
    default:
      return false;
  }
}

export function strokeMaskContains(points, radius, x, y) {
  if (points.length === 0)
    return false;
  if (points.length === 1)
    return Math.hypot(x - points[0].x, y - points[0].y) <= radius;
  for (let i = 1; i < points.length; i++) {
    if (distanceToSegment(x, y, points[i - 1], points[i]) <= radius)
      return true;
  }
  return false;
}

function distanceToSegment(x, y, a, b) {
  const dx = b.x - a.x;
  const dy = b.y - a.y;
  const length = dx * dx + dy * dy;
  if (length <= 0.0001)
    return Math.hypot(x - a.x, y - a.y);
  let t = ((x - a.x) * dx + (y - a.y) * dy) / length;
  t = Math.max(0, Math.min(1, t));
  return Math.hypot(x - (a.x + t * dx), y - (a.y + t * dy));
}

function setColor(cr, color, alpha = 1) {
  cr.setSourceRGBA(
    color[0] / 255,
    color[1] / 255,
    color[2] / 255,
    (color[3] / 255) * alpha);
}

function strokePath(cr, points, width, singlePoint = true) {
  if (points.length === 0)
    return;
  cr.setLineWidth(Math.max(0.5, width));
  cr.setLineCap(cairo.LineCap.ROUND);
  cr.setLineJoin(cairo.LineJoin.ROUND);
  if (points.length === 1 && singlePoint) {
    cr.arc(points[0].x, points[0].y, Math.max(0.5, width / 2), 0, Math.PI * 2);
    cr.fill();
    return;
  }
  cr.moveTo(points[0].x, points[0].y);
  for (let i = 1; i < points.length; i++)
    cr.lineTo(points[i].x, points[i].y);
}

export function drawAnnotations(cr, annotations, options) {
  for (const annotation of annotations) {
    if (annotation.kind === Tool.TEXT)
      drawText(cr, annotation);
    else
      drawVector(cr, annotation, options);
  }
}

function drawVector(cr, annotation, options) {
  const width = Math.max(0.5, annotation.width);
  switch (annotation.kind) {
    case Tool.RECT:
      setColor(cr, annotation.color);
      cr.setLineWidth(width);
      cr.rectangle(annotation.x, annotation.y, annotation.w, annotation.h);
      cr.stroke();
      break;
    case Tool.ARROW:
      drawArrow(cr, annotation);
      break;
    case Tool.LINE:
      setColor(cr, annotation.color);
      cr.setLineWidth(width);
      cr.moveTo(annotation.x1, annotation.y1);
      cr.lineTo(annotation.x2, annotation.y2);
      cr.stroke();
      break;
    case Tool.PENCIL:
      setColor(cr, annotation.color);
      strokePath(cr, annotation.points, width);
      cr.stroke();
      break;
    case Tool.MARKER:
      setColor(cr, annotation.color, 0.45);
      strokePath(cr, annotation.points, width * 4);
      cr.stroke();
      break;
    case Tool.MOSAIC:
      drawMosaic(cr, annotation, options, false);
      break;
    case Tool.BLUR:
      drawMosaic(cr, annotation, options, true);
      break;
    case Tool.ERASER:
      drawEraser(cr, annotation, options);
      break;
    default:
      break;
  }
}

function drawArrow(cr, annotation) {
  const { x1, y1, x2, y2, color, width } = annotation;
  const angle = Math.atan2(y2 - y1, x2 - x1);
  const head = Math.min(40, 12 + width * 2);
  setColor(cr, color);
  cr.setLineWidth(Math.max(1, width));
  cr.setLineCap(cairo.LineCap.ROUND);
  cr.moveTo(x1, y1);
  cr.lineTo(x2, y2);
  cr.stroke();
  for (const spread of [-0.5, 0.5]) {
    cr.moveTo(x2, y2);
    cr.lineTo(
      x2 - head * Math.cos(angle + spread),
      y2 - head * Math.sin(angle + spread));
  }
  cr.stroke();
}

function drawMosaic(cr, annotation, options, soft) {
  const source = options.sourcePixbuf;
  if (!source)
    return;
  const radius = Math.max(18, annotation.width * 6);
  const scale = options.deviceScale || 1;
  const origin = options.sourceOrigin ?? { x: 0, y: 0 };
  const block = Math.max(soft ? 4 : 8, annotation.width * (soft ? 2 : 4));
  let minX = Infinity, minY = Infinity, maxX = -Infinity, maxY = -Infinity;
  for (const point of annotation.points) {
    minX = Math.min(minX, point.x);
    minY = Math.min(minY, point.y);
    maxX = Math.max(maxX, point.x);
    maxY = Math.max(maxY, point.y);
  }
  minX = Math.floor(minX - radius);
  minY = Math.floor(minY - radius);
  maxX = Math.ceil(maxX + radius);
  maxY = Math.ceil(maxY + radius);
  const startX = Math.floor(minX / block) * block;
  const startY = Math.floor(minY / block) * block;
  for (let y = startY; y <= maxY; y += block) {
    for (let x = startX; x <= maxX; x += block) {
      if (!strokeMaskContains(annotation.points, radius, x + block / 2, y + block / 2))
        continue;
      const sampleX = Math.round((x + block / 2 - origin.x) * scale);
      const sampleY = Math.round((y + block / 2 - origin.y) * scale);
      const color = pixbufPixel(source, sampleX, sampleY);
      setColor(cr, color);
      cr.rectangle(x, y, block, block);
      cr.fill();
    }
  }
}

function drawEraser(cr, annotation, options) {
  const source = options.sourcePixbuf;
  if (!source)
    return;
  const radius = Math.max(15, annotation.width * 5);
  const origin = options.sourceOrigin ?? { x: 0, y: 0 };
  cr.save();
  strokePath(cr, annotation.points, radius * 2, false);
  cr.clip();
  paintPixbuf(cr, source, origin.x, origin.y,
    1 / (options.deviceScale || 1));
  cr.restore();
}

function drawText(cr, annotation) {
  const layout = PangoCairo.create_layout(cr);
  const description = Pango.FontDescription.from_string(
    `Sans ${Math.max(6, Math.round(annotation.size))}`);
  layout.set_font_description(description);
  layout.set_text(annotation.text, -1);
  cr.moveTo(annotation.x, annotation.y);
  setColor(cr, annotation.color);
  PangoCairo.show_layout(cr, layout);
}
