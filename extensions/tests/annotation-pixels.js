import GdkPixbuf from 'gi://GdkPixbuf';

const [draftPath, committedPath, exportPath, scaleArg, xArg, yArg, wArg, hArg] = ARGV;
const [scale, x, y, w, h] = [scaleArg, xArg, yArg, wArg, hArg].map(Number);
for (const [index, path] of [draftPath, committedPath, exportPath].entries()) {
  const image = GdkPixbuf.Pixbuf.new_from_file(path);
  const bytes = image.get_pixels();
  const channels = image.get_n_channels();
  const stride = image.get_rowstride();
  const left = index === 2 ? 0 : Math.round(x * scale);
  const top = index === 2 ? 0 : Math.round(y * scale);
  let red = 0;
  for (let dy = 10; dy < Math.round(h * scale) - 10; dy++) {
    for (let dx = 10; dx < Math.round(w * scale) - 10; dx++) {
      const offset = (top + dy) * stride + (left + dx) * channels;
      if (bytes[offset] > 200 && bytes[offset + 1] < 150 && bytes[offset + 2] < 150)
        red++;
    }
  }
  if (red < 50)
    throw new Error(`annotation missing from ${path}: ${red} red pixels`);
}
print('annotation visible while drawing, after release, and in export');
