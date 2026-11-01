import Gio from 'gi://Gio';
import GLib from 'gi://GLib';
import Gdk from 'gi://Gdk?version=4.0';
import GdkPixbuf from 'gi://GdkPixbuf';
import Cogl from 'gi://Cogl';
import St from 'gi://St';
import cairo from 'cairo';

export function tempPath(suffix = '.png') {
  const dir = GLib.build_filenamev([GLib.get_user_runtime_dir(), 'cosnip']);
  GLib.mkdir_with_parents(dir, 0o700);
  return GLib.build_filenamev([
    dir,
    `cosnip-${GLib.get_monotonic_time()}-${Math.floor(Math.random() * 100000)}${suffix}`,
  ]);
}

function opaquePixbuf(pixbuf) {
  if (!pixbuf.get_has_alpha())
    return pixbuf;
  const width = pixbuf.get_width();
  const height = pixbuf.get_height();
  const source = pixbuf.get_pixels();
  const stride = pixbuf.get_rowstride();
  const channels = pixbuf.get_n_channels();
  const outputStride = width * 3;
  const data = new Uint8Array(outputStride * height);
  for (let y = 0; y < height; y++) {
    for (let x = 0; x < width; x++) {
      const sourceOffset = y * stride + x * channels;
      const outputOffset = y * outputStride + x * 3;
      data[outputOffset] = source[sourceOffset];
      data[outputOffset + 1] = source[sourceOffset + 1];
      data[outputOffset + 2] = source[sourceOffset + 2];
    }
  }
  return GdkPixbuf.Pixbuf.new_from_bytes(GLib.Bytes.new(data),
    GdkPixbuf.Colorspace.RGB, false, 8, width, height, outputStride);
}

function saveOptions(dpi, format) {
  const jpeg = format === 'jpeg' || format === 'jpg';
  const keys = [jpeg ? 'quality' : 'compression'];
  const values = [jpeg ? '90' : '3'];
  if (dpi > 0) {
    keys.push('x-dpi', 'y-dpi');
    values.push(`${Math.round(dpi)}`, `${Math.round(dpi)}`);
  }
  return { type: jpeg ? 'jpeg' : 'png', keys, values };
}

export async function saveSurface(surface, path, dpi = 0, format = 'png') {
  const file = Gio.File.new_for_path(path);
  const parent = file.get_parent();
  if (parent && !parent.query_exists(null))
    parent.make_directory_with_parents(null);
  const pixbuf = opaquePixbuf(surfaceToPixbuf(surface));
  const { type, keys, values } = saveOptions(dpi, format);
  const stream = file.replace(null, false, Gio.FileCreateFlags.NONE, null);
  try {
    await new Promise((resolve, reject) => {
      pixbuf.save_to_streamv_async(stream, type, keys, values, null,
        (_source, result) => {
          try {
            GdkPixbuf.Pixbuf.save_to_stream_finish(result);
            resolve();
          } catch (error) {
            reject(error);
          }
        });
    });
    stream.close(null);
  } catch (error) {
    stream.close(null);
    GLib.unlink(path);
    throw error;
  }
}

function saveSurfaceSync(surface, path, dpi = 0, format = 'png') {
  const file = Gio.File.new_for_path(path);
  const parent = file.get_parent();
  if (parent && !parent.query_exists(null))
    parent.make_directory_with_parents(null);
  const pixbuf = opaquePixbuf(surfaceToPixbuf(surface));
  const { type, keys, values } = saveOptions(dpi, format);
  pixbuf.savev(path, type, keys, values);
}

export function surfaceToPngBytes(surface, dpi = 0) {
  const path = tempPath();
  try {
    saveSurfaceSync(surface, path, dpi);
    const [bytes] = Gio.File.new_for_path(path).load_bytes(null);
    return bytes;
  } finally {
    GLib.unlink(path);
  }
}

export function surfaceToPixbuf(surface) {
  return Gdk.pixbuf_get_from_surface(
    surface, 0, 0, surface.getWidth(), surface.getHeight());
}

export function copySurfaceToClipboard(surface, dpi = 0) {
  const bytes = surfaceToPngBytes(surface, dpi);
  St.Clipboard.get_default().set_content(
    St.ClipboardType.CLIPBOARD, 'image/png', bytes);
}

export function pixbufToImageContent(pixbuf, width, height) {
  const content = St.ImageContent.new_with_preferred_size(width, height);
  const source = pixbuf.get_has_alpha()
    ? pixbuf
    : pixbuf.add_alpha(false, 0, 0, 0);
  const args = [
    GLib.Bytes.new(source.get_pixels()),
    Cogl.PixelFormat.RGBA_8888,
    source.get_width(),
    source.get_height(),
    source.get_rowstride(),
  ];
  const cogl = global.stage.context
    ?.get_backend?.()
    ?.get_cogl_context?.();
  if (cogl)
    args.unshift(cogl);
  if (!content.set_bytes(...args))
    throw new Error('cannot upload pin image');
  return content;
}

export function surfaceToImageContent(surface, width, height) {
  return pixbufToImageContent(surfaceToPixbuf(surface), width, height);
}

export function paintPixbuf(cr, pixbuf, x, y, scale = 1, filter = null) {
  cr.save();
  cr.translate(x, y);
  cr.scale(scale, scale);
  Gdk.cairo_set_source_pixbuf(cr, pixbuf, 0, 0);
  if (filter !== null)
    cr.getSource().setFilter(filter);
  cr.paint();
  cr.restore();
}

export function expandFilename(pattern, now) {
  const pad = value => String(value).padStart(2, '0');
  return (pattern || 'Cosnip_%Y-%m-%d_%H-%M-%S.png').replace(
    /%[YmdHMS]/g, token => {
      switch (token) {
        case '%Y': return String(now.get_year());
        case '%m': return pad(now.get_month());
        case '%d': return pad(now.get_day_of_month());
        case '%H': return pad(now.get_hour());
        case '%M': return pad(now.get_minute());
        case '%S': return pad(now.get_second());
        default: return token;
      }
    });
}

export function uniquePath(dir, name) {
  let path = GLib.build_filenamev([dir, name]);
  if (!GLib.file_test(path, GLib.FileTest.EXISTS))
    return path;
  const dot = name.lastIndexOf('.');
  const stem = dot > 0 ? name.slice(0, dot) : name;
  const suffix = dot > 0 ? name.slice(dot) : '';
  for (let index = 1; ; index++) {
    path = GLib.build_filenamev([dir, `${stem}-${index}${suffix}`]);
    if (!GLib.file_test(path, GLib.FileTest.EXISTS))
      return path;
  }
}

export function defaultSaveDir() {
  const pictures = GLib.get_user_special_dir(
    GLib.UserDirectory.DIRECTORY_PICTURES);
  return pictures || GLib.build_filenamev([GLib.get_home_dir(), 'Pictures']);
}

const PIXBUF_INFO = new WeakMap();

function pixbufInfo(pixbuf) {
  let info = PIXBUF_INFO.get(pixbuf);
  if (!info) {
    info = {
      pixels: pixbuf.get_pixels(),
      width: pixbuf.get_width(),
      height: pixbuf.get_height(),
      channels: pixbuf.get_n_channels(),
      stride: pixbuf.get_rowstride(),
    };
    PIXBUF_INFO.set(pixbuf, info);
  }
  return info;
}

export function pixbufPixel(pixbuf, x, y) {
  const { pixels, width, height, channels, stride } = pixbufInfo(pixbuf);
  if (x < 0 || y < 0 || x >= width || y >= height)
    return [0, 0, 0, 0];
  const index = y * stride + x * channels;
  return [
    pixels[index],
    channels > 1 ? pixels[index + 1] : pixels[index],
    channels > 2 ? pixels[index + 2] : pixels[index],
    channels > 3 ? pixels[index + 3] : 255,
  ];
}
