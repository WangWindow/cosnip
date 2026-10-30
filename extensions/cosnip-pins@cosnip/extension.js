import Cogl from 'gi://Cogl';
import Gio from 'gi://Gio';
import GLib from 'gi://GLib';
import Clutter from 'gi://Clutter';
import St from 'gi://St';

import * as Main from 'resource:///org/gnome/shell/ui/main.js';
import {Extension} from 'resource:///org/gnome/shell/extensions/extension.js';

const SERVICE_NAME = 'org.cosnip.Pins';
const OBJECT_PATH = '/org/cosnip/Pins';

const INTERFACE_XML = `
<node>
  <interface name="org.cosnip.Pins">
    <method name="Pin">
      <arg type="ay" direction="in" name="rgba"/>
      <arg type="i" direction="in" name="x"/>
      <arg type="i" direction="in" name="y"/>
      <arg type="i" direction="in" name="width"/>
      <arg type="i" direction="in" name="height"/>
      <arg type="i" direction="in" name="pixel_width"/>
      <arg type="i" direction="in" name="pixel_height"/>
      <arg type="u" direction="out" name="id"/>
    </method>
    <method name="Clear"/>
    <method name="Count">
      <arg type="u" direction="out" name="count"/>
    </method>
    <method name="Ping">
      <arg type="s" direction="out" name="version"/>
    </method>
  </interface>
</node>`;

class PinsService {
    constructor(extension) {
        this._extension = extension;
    }

    Pin(rgba, x, y, width, height, pixelWidth, pixelHeight) {
        return this._extension.addPin(
            rgba, x, y, width, height, pixelWidth, pixelHeight);
    }

    Clear() {
        this._extension.clearPins();
    }

    Count() {
        return this._extension.count;
    }

    Ping() {
        return 'cosnip-pins/1';
    }
}

export default class CosnipPinsExtension extends Extension {
    enable() {
        this._pins = new Map();
        this._nextId = 1;
        this._service = new PinsService(this);
        this._dbus = Gio.DBusExportedObject.wrapJSObject(
            INTERFACE_XML, this._service);

        try {
            this._dbus.export(Gio.DBus.session, OBJECT_PATH);
            this._nameOwnerId = Gio.bus_own_name(
                Gio.BusType.SESSION,
                SERVICE_NAME,
                Gio.BusNameOwnerFlags.NONE,
                null,
                null,
                null);
        } catch (error) {
            this._dbus.unexport();
            this._dbus = null;
            throw error;
        }

        this._overviewShowingId = Main.overview.connect(
            'showing', () => this._setPinsVisible(false));
        this._overviewHiddenId = Main.overview.connect(
            'hidden', () => this._setPinsVisible(true));
    }

    disable() {
        if (this._overviewShowingId) {
            Main.overview.disconnect(this._overviewShowingId);
            this._overviewShowingId = null;
        }
        if (this._overviewHiddenId) {
            Main.overview.disconnect(this._overviewHiddenId);
            this._overviewHiddenId = null;
        }

        this.clearPins();

        if (this._dbus) {
            this._dbus.unexport();
            this._dbus = null;
        }
        if (this._nameOwnerId) {
            Gio.bus_unown_name(this._nameOwnerId);
            this._nameOwnerId = null;
        }
        this._service = null;
    }

    get count() {
        return this._pins?.size ?? 0;
    }

    addPin(rgba, x, y, width, height, pixelWidth, pixelHeight) {
        if (pixelWidth <= 0 || pixelHeight <= 0) {
            throw new Error('pin image has an invalid size');
        }
        const expected = pixelWidth * pixelHeight * 4;
        if (rgba.length !== expected) {
            throw new Error(
                `pin image has ${rgba.length} bytes, expected ${expected}`);
        }

        if (width <= 0 || height <= 0) {
            let scale = 1;
            const monitors = global.display.get_n_monitors();
            for (let i = 0; i < monitors; i++) {
                const geometry = global.display.get_monitor_geometry(i);
                if (x >= geometry.x && x < geometry.x + geometry.width &&
                    y >= geometry.y && y < geometry.y + geometry.height) {
                    scale = global.display.get_monitor_scale(i) || 1;
                    break;
                }
            }
            width = pixelWidth / scale;
            height = pixelHeight / scale;
        }
        width = Math.max(1, Math.round(width));
        height = Math.max(1, Math.round(height));

        const data = rgba instanceof Uint8Array
            ? rgba
            : Uint8Array.from(rgba);
        const content = St.ImageContent.new_with_preferred_size(width, height);
        const args = [
            GLib.Bytes.new(data),
            Cogl.PixelFormat.RGBA_8888,
            pixelWidth,
            pixelHeight,
            pixelWidth * 4,
        ];
        const cogl = global.stage.context
            ?.get_backend?.()
            ?.get_cogl_context?.();
        if (cogl)
            args.unshift(cogl);

        if (!content.set_bytes(...args))
            throw new Error('cannot upload the pin image');

        const actor = new St.Widget({
            content,
            width,
            height,
            reactive: true,
            track_hover: true,
        });
        actor.set_pivot_point(0, 0);
        actor.set_position(Math.round(x), Math.round(y));

        const id = this._nextId++;
        const originalWidth = pixelWidth;
        const originalHeight = pixelHeight;

        actor.connect('button-press-event', (_actor, event) => {
            if (event.get_button() === 3) {
                this.removePin(id);
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

            const ratio = originalWidth / originalHeight;
            const minWidth = 32;
            const maxWidth = originalWidth * 8;
            const step = Math.max(-1, Math.min(1, delta));
            const nextWidth = Math.min(
                maxWidth,
                Math.max(minWidth, actor.width * Math.exp(step * 0.08)));
            actor.set_size(nextWidth, nextWidth / ratio);
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

        this._pins.set(id, {actor, content});
        Main.layoutManager.addChrome(actor);
        actor.visible = !Main.overview.visible;
        return id;
    }

    removePin(id) {
        const pin = this._pins.get(id);
        if (!pin)
            return;
        this._pins.delete(id);
        Main.layoutManager.removeChrome(pin.actor);
        pin.actor.destroy();
        pin.content.run_dispose?.();
    }

    clearPins() {
        if (!this._pins)
            return;
        for (const id of [...this._pins.keys()])
            this.removePin(id);
    }

    _setPinsVisible(visible) {
        for (const {actor} of this._pins.values())
            actor.visible = visible;
    }
}
