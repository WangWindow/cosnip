import Gio from 'gi://Gio';
import GLib from 'gi://GLib';
import Gdk from 'gi://Gdk?version=4.0';
import GdkPixbuf from 'gi://GdkPixbuf';
import Cogl from 'gi://Cogl';
import St from 'gi://St';
import cairo from 'cairo';

export function parseColor(value) {
    const text = (value || '#FF4040').replace('#', '');
    const rgba = [255, 64, 64, 255];
    if (text.length === 6 || text.length === 8) {
        rgba[0] = parseInt(text.slice(0, 2), 16) || 0;
        rgba[1] = parseInt(text.slice(2, 4), 16) || 0;
        rgba[2] = parseInt(text.slice(4, 6), 16) || 0;
        rgba[3] = text.length === 8 ? parseInt(text.slice(6, 8), 16) || 0 : 255;
    }
    return rgba;
}

export function tempPath(suffix = '.png') {
    const dir = GLib.build_filenamev([GLib.get_user_runtime_dir(), 'cosnip']);
    GLib.mkdir_with_parents(dir, 0o700);
    return GLib.build_filenamev([
        dir,
        `cosnip-${GLib.get_monotonic_time()}-${Math.floor(Math.random() * 100000)}${suffix}`,
    ]);
}

export function saveSurface(surface, path) {
    const file = Gio.File.new_for_path(path);
    const parent = file.get_parent();
    if (parent && !parent.query_exists(null))
        parent.make_directory_with_parents(null);
    surface.writeToPNG(path);
}

export function surfaceToPngBytes(surface) {
    const path = tempPath();
    try {
        surface.writeToPNG(path);
        const [bytes] = Gio.File.new_for_path(path).load_bytes(null);
        return bytes;
    } finally {
        GLib.unlink(path);
    }
}

export function surfaceToPixbuf(surface) {
    const path = tempPath();
    try {
        surface.writeToPNG(path);
        return GdkPixbuf.Pixbuf.new_from_file(path);
    } finally {
        GLib.unlink(path);
    }
}

export function copySurfaceToClipboard(surface) {
    const bytes = surfaceToPngBytes(surface);
    St.Clipboard.get_default().set_content(
        St.ClipboardType.CLIPBOARD, 'image/png', bytes);
}

export function pixbufToImageContent(pixbuf, width, height) {
    const content = St.ImageContent.new_with_preferred_size(width, height);
    const args = [
        GLib.Bytes.new(pixbuf.get_pixels()),
        Cogl.PixelFormat.RGBA_8888,
        pixbuf.get_width(),
        pixbuf.get_height(),
        pixbuf.get_rowstride(),
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

export function paintPixbuf(cr, pixbuf, x, y, scale = 1) {
    cr.save();
    cr.translate(x, y);
    cr.scale(scale, scale);
    Gdk.cairo_set_source_pixbuf(cr, pixbuf, 0, 0);
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

export function pixbufPixel(pixbuf, x, y) {
    const width = pixbuf.get_width();
    const height = pixbuf.get_height();
    if (x < 0 || y < 0 || x >= width || y >= height)
        return [0, 0, 0, 0];
    const pixels = pixbuf.get_pixels();
    const channels = pixbuf.get_n_channels();
    const stride = pixbuf.get_rowstride();
    const index = y * stride + x * channels;
    return [
        pixels[index],
        channels > 1 ? pixels[index + 1] : pixels[index],
        channels > 2 ? pixels[index + 2] : pixels[index],
        channels > 3 ? pixels[index + 3] : 255,
    ];
}
