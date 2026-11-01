import GdkPixbuf from 'gi://GdkPixbuf';

const [path, widthArg, heightArg, dpiArg] = ARGV;
const image = GdkPixbuf.Pixbuf.new_from_file(path);
const width = Number(widthArg);
const height = Number(heightArg);
const dpi = Number(dpiArg);
if (image.get_width() !== width || image.get_height() !== height)
  throw new Error(`JPEG dimensions ${image.get_width()}x${image.get_height()} != ${width}x${height}`);
if (Number(image.get_option('x-dpi')) !== dpi)
  throw new Error(`JPEG DPI ${image.get_option('x-dpi')} != ${dpi}`);
print(`JPEG: ${width}x${height}, ${dpi} DPI`);
