import Clutter from 'gi://Clutter';
import St from 'gi://St';
import cairo from 'cairo';

import * as Main from 'resource:///org/gnome/shell/ui/main.js';

import {pixbufToImageContent, parseColor, surfaceToPixbuf} from './util.js';

export class PinManager {
    constructor(settings, onChange) {
        this._settings = settings;
        this._pins = new Map();
        this._nextId = 1;
        this._onChange = onChange;
    }

    get count() {
        return this._pins.size;
    }

    createFromSurface(surface, x, y, width, height) {
        const pixbuf = surfaceToPixbuf(surface);
        return this.createFromPixbuf(pixbuf, x, y, width, height);
    }

    createFromPixbuf(pixbuf, x, y, width, height) {
        width = Math.max(1, Math.round(width));
        height = Math.max(1, Math.round(height));
        const content = pixbufToImageContent(
            pixbuf, pixbuf.get_width(), pixbuf.get_height());
        x = Math.max(0, Math.min(global.stage.width - width, x));
        y = Math.max(0, Math.min(global.stage.height - height, y));
        const actor = new St.Widget({
            width,
            height,
            reactive: true,
            track_hover: true,
        });
        const image = new St.Widget({
            content,
            width,
            height,
            reactive: false,
        });
        actor.add_child(image);
        const border = new St.DrawingArea({
            width,
            height,
            reactive: false,
        });
        border.connect('repaint', area => drawPinBorder(area, this._settings));
        actor.add_child(border);
        actor.set_pivot_point(0, 0);
        actor.set_position(Math.round(x), Math.round(y));
        actor.opacity = Math.round(
            Math.max(10, Math.min(100, this._settings.get_int('pin-opacity'))) * 2.55);

        const id = this._nextId++;
        const pixelWidth = pixbuf.get_width();
        const pixelHeight = pixbuf.get_height();
        actor.connect('button-press-event', (_actor, event) => {
            if (event.get_button() === 3) {
                this.remove(id);
                return Clutter.EVENT_STOP;
            }
            return Clutter.EVENT_PROPAGATE;
        });
        actor.connect('scroll-event', (_actor, event) => {
            const [dx, dy] = event.get_scroll_delta();
            let delta = dy !== 0 ? dy : -dx;
            if (delta === 0) {
                const direction = event.get_scroll_direction();
                if (direction === Clutter.ScrollDirection.UP)
                    delta = 1;
                else if (direction === Clutter.ScrollDirection.DOWN)
                    delta = -1;
                else
                    return Clutter.EVENT_PROPAGATE;
            }
            const ratio = pixelWidth / pixelHeight;
            const step = Math.max(-1, Math.min(1, delta));
            const nextWidth = Math.min(
                pixelWidth * 8,
                Math.max(32, actor.width * Math.exp(step * 0.08)));
            const nextHeight = nextWidth / ratio;
            if (this._settings.get_boolean('pin-zoom-at-pointer')) {
                const [px, py] = global.get_pointer();
                const relX = (px - actor.x) / Math.max(1, actor.width);
                const relY = (py - actor.y) / Math.max(1, actor.height);
                actor.set_size(nextWidth, nextHeight);
                actor.set_position(px - relX * nextWidth, py - relY * nextHeight);
            } else {
                actor.set_size(nextWidth, nextHeight);
            }
            image.set_size(nextWidth, nextHeight);
            border.set_size(nextWidth, nextHeight);
            border.queue_repaint();
            return Clutter.EVENT_STOP;
        });
        const pan = new Clutter.PanGesture();
        pan.begin_threshold = 3;
        pan.connect('pan-update', gesture => {
            const delta = gesture.get_delta();
            actor.set_position(
                actor.x + delta.get_x(),
                actor.y + delta.get_y());
        });
        actor.add_action(pan);

        this._pins.set(id, {actor, content, image, border});
        Main.layoutManager.addChrome(actor);
        actor.visible = !Main.overview.visible;
        this._onChange?.();
        return id;
    }

    createFromClipboardPixbuf(pixbuf) {
        const [x, y] = global.get_pointer();
        let monitor = 0;
        for (let i = 0; i < global.display.get_n_monitors(); i++) {
            const geometry = global.display.get_monitor_geometry(i);
            if (x >= geometry.x && x < geometry.x + geometry.width &&
                y >= geometry.y && y < geometry.y + geometry.height) {
                monitor = i;
                break;
            }
        }
        const scale = global.display.get_monitor_scale(monitor) || 1;
        const width = pixbuf.get_width() / scale;
        const height = pixbuf.get_height() / scale;
        return this.createFromPixbuf(pixbuf, x, y, width, height);
    }

    remove(id) {
        const pin = this._pins.get(id);
        if (!pin)
            return;
        this._pins.delete(id);
        Main.layoutManager.removeChrome(pin.actor);
        pin.actor.destroy();
        pin.content.run_dispose?.();
        this._onChange?.();
    }

    clear() {
        for (const id of [...this._pins.keys()])
            this.remove(id);
    }

    setVisible(visible) {
        for (const {actor} of this._pins.values())
            actor.visible = visible;
    }
}

function drawPinBorder(area, settings) {
    const cr = area.get_context();
    const width = area.width;
    const height = area.height;
    if (width < 2 || height < 2)
        return;
    const borderWidth = Math.max(0, settings.get_int('pin-border-width'));
    const color = parseColor(settings.get_string('pin-border-color'), [0, 0, 0, 140]);
    if (borderWidth > 0) {
        cr.setSourceRGBA(
            color[0] / 255, color[1] / 255, color[2] / 255, color[3] / 255);
        cr.setLineWidth(borderWidth);
        cr.rectangle(
            borderWidth / 2, borderWidth / 2,
            width - borderWidth, height - borderWidth);
        cr.stroke();
    }
    cr.setSourceRGBA(1, 1, 1, 0.18);
    cr.setLineWidth(1);
    cr.rectangle(1.5, 1.5, width - 3, height - 3);
    cr.stroke();
}
