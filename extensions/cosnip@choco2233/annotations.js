import Gdk from 'gi://Gdk?version=4.0';
import Pango from 'gi://Pango';
import PangoCairo from 'gi://PangoCairo';
import cairo from 'cairo';

import { pixbufPixel } from './util.js';

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
      return { ...base, x: point.x, y: point.y, w: 0, h: 0,
        anchorX: point.x, anchorY: point.y };
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
      const anchorX = annotation.anchorX;
      const anchorY = annotation.anchorY;
      let x = Math.min(anchorX, point.x);
      let y = Math.min(anchorY, point.y);
      let w = Math.abs(point.x - anchorX);
      let h = Math.abs(point.y - anchorY);
      if (shift) {
        const side = Math.max(w, h);
        w = side;
        h = side;
        if (point.x < anchorX)
          x = anchorX - side;
        if (point.y < anchorY)
          y = anchorY - side;
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
      return annotation.points.length >= 2;
    case Tool.ERASER:
      return annotation.points.length >= 1;
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

export function draftBounds(annotation, selection, pathBounds = null) {
  const width = Math.max(0.5, annotation.width ?? 1);
  let minX, minY, maxX, maxY;
  if (annotation.points?.length) {
    const points = annotation.points;
    const bounds = pathBounds ?? {
      minX: Math.min(...points.map(point => point.x)),
      minY: Math.min(...points.map(point => point.y)),
      maxX: Math.max(...points.map(point => point.x)),
      maxY: Math.max(...points.map(point => point.y)),
    };
    ({ minX, minY, maxX, maxY } = bounds);
  } else if (annotation.kind === Tool.RECT) {
    minX = annotation.x; minY = annotation.y;
    maxX = annotation.x + annotation.w; maxY = annotation.y + annotation.h;
  } else {
    minX = Math.min(annotation.x1, annotation.x2);
    minY = Math.min(annotation.y1, annotation.y2);
    maxX = Math.max(annotation.x1, annotation.x2);
    maxY = Math.max(annotation.y1, annotation.y2);
  }
  let margin = width / 2 + 2;
  if (annotation.kind === Tool.ARROW)
    margin += Math.min(40, 12 + width * 2);
  else if (annotation.kind === Tool.MARKER)
    margin = width * 2 + 2;
  else if (annotation.kind === Tool.ERASER)
    margin = eraserRadius(annotation) + 2;
  else if (annotation.kind === Tool.MOSAIC || annotation.kind === Tool.BLUR) {
    const block = Math.max(annotation.kind === Tool.MOSAIC ? 8 : 4,
      width * (annotation.kind === Tool.MOSAIC ? 4 : 2));
    margin = Math.max(18, width * 6) + block / 2 + 2;
  }
  const tile = 64;
  const sx = Math.round(selection.x), sy = Math.round(selection.y);
  const sw = Math.max(1, Math.round(selection.w));
  const sh = Math.max(1, Math.round(selection.h));
  const x = Math.max(sx, sx + Math.floor((minX - margin - sx) / tile) * tile);
  const y = Math.max(sy, sy + Math.floor((minY - margin - sy) / tile) * tile);
  const right = Math.min(sx + sw, sx + Math.ceil((maxX + margin - sx) / tile) * tile);
  const bottom = Math.min(sy + sh, sy + Math.ceil((maxY + margin - sy) / tile) * tile);
  return { x, y, w: Math.max(1, right - x), h: Math.max(1, bottom - y) };
}

const MOSAIC_CACHE = new WeakMap();

// A draft only appends points. Reuse its existing cells and examine only new
// segments on subsequent frames; the WeakMap releases data with the stroke.
export function mosaicCells(points, radius, block) {
  if (points.length === 0)
    return [];
  let cache = MOSAIC_CACHE.get(points);
  if (!cache || cache.radius !== radius || cache.block !== block ||
      points.length < cache.count || points[cache.count - 1] !== cache.lastPoint) {
    cache = { radius, block, count: 0, lastPoint: null, cells: [], visited: new Set() };
    MOSAIC_CACHE.set(points, cache);
  }
  const { cells, visited } = cache;
  if (cache.count === points.length)
    return cells;
  const segments = Math.max(1, points.length - 1);
  const first = cache.count <= 1 ? 0 : cache.count - 1;
  for (let i = first; i < segments; i++) {
    const a = points[i];
    const b = points[Math.min(i + 1, points.length - 1)];
    const minX = Math.floor((Math.min(a.x, b.x) - radius) / block);
    const minY = Math.floor((Math.min(a.y, b.y) - radius) / block);
    const maxX = Math.ceil((Math.max(a.x, b.x) + radius) / block);
    const maxY = Math.ceil((Math.max(a.y, b.y) + radius) / block);
    for (let row = minY; row <= maxY; row++) {
      for (let col = minX; col <= maxX; col++) {
        const key = `${col},${row}`;
        if (visited.has(key))
          continue;
        const x = col * block;
        const y = row * block;
        if (distanceToSegment(x + block / 2, y + block / 2, a, b) > radius)
          continue;
        visited.add(key);
        cells.push([x, y]);
      }
    }
  }
  cache.count = points.length;
  cache.lastPoint = points[points.length - 1];
  return cells;
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
  for (const [x, y] of mosaicCells(annotation.points, radius, block)) {
    const sampleX = Math.round((x + block / 2 - origin.x) * scale);
    const sampleY = Math.round((y + block / 2 - origin.y) * scale);
    const color = pixbufPixel(source, sampleX, sampleY);
    setColor(cr, color);
    cr.rectangle(x, y, block, block);
    cr.fill();
  }
}

function eraserRadius(annotation) {
  return Math.max(0.5, annotation.width / 2);
}

const ERASER_SOURCES = new WeakMap();

function drawEraser(cr, annotation, options) {
  const source = options.sourcePixbuf;
  if (!source || !annotation.points.length)
    return;
  const radius = eraserRadius(annotation);
  const origin = options.sourceOrigin ?? { x: 0, y: 0 };
  const scale = options.deviceScale || 1;
  cr.save();
  // Set the screenshot as the source while its transform is active, then
  // restore the drawing coordinates before stroking the actual brush path.
  cr.translate(origin.x, origin.y);
  cr.scale(1 / scale, 1 / scale);
  let pattern = ERASER_SOURCES.get(source);
  if (pattern) {
    cr.setSource(pattern);
  } else {
    // Converting the full capture for every pointer motion is expensive.
    Gdk.cairo_set_source_pixbuf(cr, source, 0, 0);
    pattern = cr.getSource();
    pattern.setFilter(cairo.Filter.NEAREST);
    ERASER_SOURCES.set(source, pattern);
  }
  cr.scale(scale, scale);
  cr.translate(-origin.x, -origin.y);
  if (annotation.points.length === 1) {
    cr.arc(annotation.points[0].x, annotation.points[0].y, radius, 0, 2 * Math.PI);
    cr.fill();
  } else {
    strokePath(cr, annotation.points, radius * 2, false);
    cr.stroke();
  }
  cr.restore();
}

function drawText(cr, annotation) {
  const layout = PangoCairo.create_layout(cr);
  const description = Pango.FontDescription.from_string('Sans');
  description.set_absolute_size(
    Math.max(6, Math.round(annotation.size)) * Pango.SCALE);
  layout.set_font_description(description);
  layout.set_text(annotation.text, -1);
  cr.moveTo(annotation.x, annotation.y);
  setColor(cr, annotation.color);
  PangoCairo.show_layout(cr, layout);
}
