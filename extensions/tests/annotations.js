import {
  Tool, createAnnotation, draftBounds, mosaicCells, strokeMaskContains,
  updateAnnotation
} from '../cosnip@choco2233/annotations.js';

function assert(condition, message) {
  if (!condition)
    throw new Error(message);
}

const rectangle = createAnnotation(Tool.RECT, { x: 50, y: 50 }, {
  color: [255, 0, 0, 255], width: 3,
});
updateAnnotation(rectangle, { x: 20, y: 10 }, false);
updateAnnotation(rectangle, { x: 70, y: 80 }, false);
assert(rectangle.x === 50 && rectangle.y === 50 &&
  rectangle.w === 20 && rectangle.h === 30,
  'rectangle must retain its starting point when the pointer changes direction');
updateAnnotation(rectangle, { x: 30, y: 40 }, true);
assert(rectangle.x === 30 && rectangle.y === 30 &&
  rectangle.w === 20 && rectangle.h === 20,
  'shift-drag must produce a square anchored at the original point');

const cases = [
  [],
  [{ x: 0, y: 0 }],
  [{ x: -12, y: 4 }, { x: 41, y: 28 }],
  [{ x: 1, y: 1 }, { x: 2, y: 30 }, { x: 47, y: -12 }],
  Array.from({ length: 200 }, (_, i) => ({ x: i * 2, y: i * 1.5 })),
];
for (const points of cases) {
  for (const [radius, block] of [[18, 8], [24, 12], [18, 4]]) {
    const cells = mosaicCells(points, radius, block);
    const actual = new Set(cells.map(([x, y]) => `${x},${y}`));
    assert(actual.size === cells.length, 'mosaic blocks must be unique');
    if (!points.length) {
      assert(actual.size === 0, 'empty strokes must not create blocks');
      continue;
    }
    const minX = Math.floor((Math.min(...points.map(p => p.x)) - radius) / block);
    const maxX = Math.ceil((Math.max(...points.map(p => p.x)) + radius) / block);
    const minY = Math.floor((Math.min(...points.map(p => p.y)) - radius) / block);
    const maxY = Math.ceil((Math.max(...points.map(p => p.y)) + radius) / block);
    for (let row = minY; row <= maxY; row++) {
      for (let col = minX; col <= maxX; col++) {
        const x = col * block;
        const y = row * block;
        const expected = strokeMaskContains(points, radius, x + block / 2, y + block / 2);
        assert(actual.has(`${x},${y}`) === expected,
          `mosaic mismatch at ${x},${y} for radius ${radius}, block ${block}`);
      }
    }
  }
}

for (const [radius, block] of [[18, 8], [24, 12]]) {
  const growing = [];
  for (let index = 0; index < 60; index++) {
    growing.push({ x: index * 3, y: 40 + 25 * Math.sin(index / 7) });
    const incremental = new Set(mosaicCells(growing, radius, block)
      .map(([x, y]) => `${x},${y}`));
    const full = new Set(mosaicCells([...growing], radius, block)
      .map(([x, y]) => `${x},${y}`));
    assert(incremental.size === full.size && [...full].every(cell => incremental.has(cell)),
      `incremental mosaic differs from a full redraw after ${index + 1} points`);
  }
}

const draft = { kind: Tool.PENCIL, width: 3,
  points: [{ x: 100, y: 100 }, { x: 120, y: 115 }] };
const bounds = draftBounds(draft, { x: 0, y: 0, w: 1920, h: 1080 });
assert(bounds.x <= 100 && bounds.y <= 100 &&
  bounds.x + bounds.w >= 120 && bounds.y + bounds.h >= 115,
  'draft bounds must contain the active stroke');
assert(bounds.w * bounds.h < 1920 * 1080 / 20,
  'a short draft must not allocate a full-selection texture');

print('annotation geometry: ok');
