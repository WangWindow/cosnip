import GdkPixbuf from 'gi://GdkPixbuf';

const [beforePath, previewPath, exportPath, scaleArg, xArg, yArg, wArg, hArg] = ARGV;
const [scale, x, y, w, h] = [scaleArg, xArg, yArg, wArg, hArg].map(Number);
const images = [beforePath, previewPath, exportPath].map(path =>
  GdkPixbuf.Pixbuf.new_from_file(path));
const pixels = images.map(image => ({
  bytes: image.get_pixels(), stride: image.get_rowstride(),
  channels: image.get_n_channels(),
}));

function rgb(index, xPos, yPos) {
  const { bytes, stride, channels } = pixels[index];
  const offset = yPos * stride + xPos * channels;
  return [bytes[offset], bytes[offset + 1], bytes[offset + 2]];
}

const left = Math.round(x * scale);
const top = Math.round(y * scale);
const width = Math.round(w * scale);
const height = Math.round(h * scale);
if (images[2].get_width() !== width || images[2].get_height() !== height)
  throw new Error(`incorrect export dimensions at scale ${scale}`);

const expectedDpi = Math.round(96 * scale);
const exportDpi = Number(images[2].get_option('x-dpi'));
if (exportDpi !== expectedDpi)
  throw new Error(`incorrect export DPI at scale ${scale}: ${exportDpi} != ${expectedDpi}`);

// Keep clear of selection borders and compare the actual desktop pixels, not
// a hand-constructed image. The checkerboard exposes filtering and offsets.
let checked = 0;
for (let dy = 10; dy < height - 10; dy++) {
  for (let dx = 10; dx < width - 10; dx++) {
    const source = rgb(0, left + dx, top + dy);
    const preview = rgb(1, left + dx, top + dy);
    const exported = rgb(2, dx, dy);
    for (let channel = 0; channel < 3; channel++) {
      if (source[channel] !== preview[channel] || source[channel] !== exported[channel])
        throw new Error(`pixel mismatch at ${dx},${dy}, scale ${scale}`);
      checked++;
    }
  }
}
let dimmedPanelPixels = 0;
for (let yPos = 0; yPos < Math.round(24 * scale); yPos++) {
  for (let xPos = 0; xPos < images[0].get_width(); xPos++) {
    const source = rgb(0, xPos, yPos);
    const preview = rgb(1, xPos, yPos);
    if (source[0] > 100 && preview[0] < source[0] * 0.8)
      dimmedPanelPixels++;
  }
}
if (dimmedPanelPixels < 10)
  throw new Error(`the screenshot mask did not cover the panel at scale ${scale}`);
print(`scale ${scale}: ${checked} preview/export components match, ${exportDpi} DPI`);
