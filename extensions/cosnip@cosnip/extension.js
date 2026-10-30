import GdkPixbuf from 'gi://GdkPixbuf';
import Gio from 'gi://Gio';
import GLib from 'gi://GLib';
import Meta from 'gi://Meta';
import Shell from 'gi://Shell';
import St from 'gi://St';

import * as Main from 'resource:///org/gnome/shell/ui/main.js';
import * as PanelMenu from 'resource:///org/gnome/shell/ui/panelMenu.js';
import {Extension} from 'resource:///org/gnome/shell/extensions/extension.js';

import {NativeOverlay} from './overlay.js';
import {PinManager} from './pins.js';

Gio._promisify(Shell.Screenshot.prototype, 'screenshot_stage_to_content');

const SERVICE_NAME = 'org.cosnip.Native';
const OBJECT_PATH = '/org/cosnip/Native';

const INTERFACE_XML = `
<node>
  <interface name="org.cosnip.Native">
    <method name="Snip"/>
    <method name="Paste"/>
    <method name="Ping">
      <arg type="s" direction="out" name="version"/>
    </method>
    <method name="Count">
      <arg type="u" direction="out" name="count"/>
    </method>
    <method name="CaptureArea">
      <arg type="i" direction="in" name="x"/>
      <arg type="i" direction="in" name="y"/>
      <arg type="i" direction="in" name="width"/>
      <arg type="i" direction="in" name="height"/>
      <arg type="s" direction="in" name="path"/>
      <arg type="b" direction="out" name="ok"/>
      <arg type="s" direction="out" name="message"/>
    </method>
    <method name="SnipArea">
      <arg type="i" direction="in" name="x"/>
      <arg type="i" direction="in" name="y"/>
      <arg type="i" direction="in" name="width"/>
      <arg type="i" direction="in" name="height"/>
      <arg type="s" direction="in" name="path"/>
      <arg type="b" direction="out" name="ok"/>
      <arg type="s" direction="out" name="message"/>
    </method>
    <method name="AddRect">
      <arg type="i" direction="in" name="x"/>
      <arg type="i" direction="in" name="y"/>
      <arg type="i" direction="in" name="width"/>
      <arg type="i" direction="in" name="height"/>
    </method>
    <method name="AddMosaic">
      <arg type="i" direction="in" name="x1"/>
      <arg type="i" direction="in" name="y1"/>
      <arg type="i" direction="in" name="x2"/>
      <arg type="i" direction="in" name="y2"/>
    </method>
    <method name="SetSelection">
      <arg type="i" direction="in" name="x"/>
      <arg type="i" direction="in" name="y"/>
      <arg type="i" direction="in" name="width"/>
      <arg type="i" direction="in" name="height"/>
    </method>
    <method name="AddEraser">
      <arg type="i" direction="in" name="x1"/>
      <arg type="i" direction="in" name="y1"/>
      <arg type="i" direction="in" name="x2"/>
      <arg type="i" direction="in" name="y2"/>
    </method>
    <method name="AddText">
      <arg type="s" direction="in" name="text"/>
      <arg type="i" direction="in" name="x"/>
      <arg type="i" direction="in" name="y"/>
    </method>
    <method name="SaveSelection">
      <arg type="s" direction="in" name="path"/>
      <arg type="b" direction="out" name="ok"/>
    </method>
    <method name="PinSelection"/>
    <method name="CaptureStage">
      <arg type="s" direction="in" name="path"/>
      <arg type="b" direction="out" name="ok"/>
      <arg type="s" direction="out" name="message"/>
    </method>
    <method name="CloseOverlay"/>
  </interface>
</node>`;

class NativeService {
    constructor(extension) {
        this._extension = extension;
    }

    Snip() {
        this._extension.startSnip();
    }

    Paste() {
        this._extension.pasteClipboard();
    }

    Ping() {
        return 'cosnip-native/1';
    }

    Count() {
        return this._extension.pinCount;
    }

    CaptureArea(x, y, width, height, path) {
        return this._extension.captureArea(x, y, width, height, path);
    }

    SnipArea(x, y, width, height, path) {
        return this._extension.snipArea(x, y, width, height, path);
    }

    AddRect(x, y, width, height) {
        this._extension.addRect(x, y, width, height);
    }

    AddMosaic(x1, y1, x2, y2) {
        this._extension.addMosaic(x1, y1, x2, y2);
    }

    SetSelection(x, y, width, height) {
        this._extension.setSelection(x, y, width, height);
    }

    AddEraser(x1, y1, x2, y2) {
        this._extension.addEraser(x1, y1, x2, y2);
    }

    AddText(text, x, y) {
        this._extension.addText(text, x, y);
    }

    SaveSelection(path) {
        return this._extension.saveSelection(path);
    }

    PinSelection() {
        this._extension.pinSelection();
    }

    CaptureStage(path) {
        return this._extension.captureStage(path);
    }

    CloseOverlay() {
        this._extension.closeOverlay();
    }
}

export default class CosnipExtension extends Extension {
    enable() {
        this._settings = this.getSettings();
        this._overlay = null;

        this._pins = new PinManager(() => this._syncPanel());
        this._overviewShowingId = Main.overview.connect(
            'showing', () => this._pins.setVisible(false));
        this._overviewHiddenId = Main.overview.connect(
            'hidden', () => this._pins.setVisible(true));

        this._panelButton = new PanelMenu.Button(0.0, 'Cosnip', false);
        this._panelButton.add_child(new St.Icon({
            icon_name: 'applets-screenshooter-symbolic',
            style_class: 'system-status-icon',
        }));
        const menu = this._panelButton.menu;
        menu.addAction('Snip', () => this.startSnip());
        menu.addAction('Paste image as pin', () => this.pasteClipboard());
        menu.addAction('Close all pins', () => this._pins.clear());
        menu.addAction('Preferences', () => this.openPreferences());
        Main.panel.addToStatusArea('cosnip', this._panelButton);

        this._addKeybinding('snip-shortcut', () => this.startSnip());
        this._addKeybinding('paste-shortcut', () => this.pasteClipboard());

        this._service = new NativeService(this);
        this._dbus = Gio.DBusExportedObject.wrapJSObject(INTERFACE_XML, this._service);
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
            logError(error, 'cosnip: cannot export D-Bus service');
        }
        log('cosnip: native extension enabled');
    }

    disable() {
        Main.wm.removeKeybinding('snip-shortcut');
        Main.wm.removeKeybinding('paste-shortcut');
        if (this._overviewShowingId) {
            Main.overview.disconnect(this._overviewShowingId);
            this._overviewShowingId = null;
        }
        if (this._overviewHiddenId) {
            Main.overview.disconnect(this._overviewHiddenId);
            this._overviewHiddenId = null;
        }
        this.closeOverlay();
        this._pins?.clear();
        this._pins = null;
        this._panelButton?.destroy();
        this._panelButton = null;
        if (this._dbus) {
            this._dbus.unexport();
            this._dbus = null;
        }
        if (this._nameOwnerId) {
            Gio.bus_unown_name(this._nameOwnerId);
            this._nameOwnerId = null;
        }
        this._service = null;
        this._settings = null;
    }

    _addKeybinding(name, handler) {
        try {
            Main.wm.addKeybinding(
                name,
                this._settings,
                Meta.KeyBindingFlags.IGNORE_AUTOREPEAT,
                Shell.ActionMode.NORMAL | Shell.ActionMode.OVERVIEW,
                handler);
        } catch (error) {
            logError(error, `cosnip: cannot add ${name}`);
        }
    }

    _syncPanel() {
        if (!this._panelButton)
            return;
        this._panelButton.visible = true;
    }

    get pinCount() {
        return this._pins?.count ?? 0;
    }

    async startSnip() {
        if (this._overlay) {
            this._overlay.actor?.show();
            return;
        }
        try {
            const screenshot = new Shell.Screenshot();
            const [content, scale] = await screenshot.screenshot_stage_to_content();
            if (!content)
                throw new Error('empty stage content');
            this._overlay = new NativeOverlay({
                content,
                scale,
                settings: this._settings,
                onClose: () => {
                    this._overlay = null;
                },
                onPin: (surface, x, y, width, height) =>
                    this._pins.createFromSurface(surface, x, y, width, height),
            });
            await this._overlay.open();
            log('cosnip: overlay ready');
            return this._overlay;
        } catch (error) {
            this._overlay = null;
            logError(error, 'cosnip: capture failed');
            return null;
        }
    }

    closeOverlay() {
        if (!this._overlay)
            return;
        this._overlay.close();
        this._overlay = null;
    }

    async snipArea(x, y, width, height, path) {
        try {
            const overlay = await this.startSnip();
            if (!overlay)
                return [false, 'overlay did not open'];
            overlay.setSelection({x, y, w: width, h: height});
            overlay.saveTo(path);
            this.closeOverlay();
            return [true, path];
        } catch (error) {
            this.closeOverlay();
            return [false, error.message];
        }
    }

    addRect(x, y, width, height) {
        this._overlay?.addRect(x, y, width, height);
    }

    addMosaic(x1, y1, x2, y2) {
        this._overlay?.addMosaic(x1, y1, x2, y2);
    }

    addEraser(x1, y1, x2, y2) {
        this._overlay?.addEraser(x1, y1, x2, y2);
    }

    addText(text, x, y) {
        this._overlay?.addText(text, x, y);
    }

    saveSelection(path) {
        return this._overlay?.saveTo(path) ?? false;
    }

    setSelection(x, y, width, height) {
        this._overlay?.setSelection({x, y, w: width, h: height});
    }

    pinSelection() {
        this._overlay?.pinSelection();
    }

    async captureStage(path) {
        try {
            const screenshot = new Shell.Screenshot();
            const [content, scale] = await screenshot.screenshot_stage_to_content();
            const texture = content.get_texture();
            const stream = Gio.MemoryOutputStream.new_resizable();
            const pixbuf = await Shell.Screenshot.composite_to_stream(
                texture,
                0, 0,
                Math.round(global.stage.width * scale),
                Math.round(global.stage.height * scale),
                scale,
                null, 0, 0, 1,
                stream);
            stream.close(null);
            pixbuf.savev(path, 'png', [], []);
            return [true, path];
        } catch (error) {
            return [false, error.message];
        }
    }

    async captureArea(x, y, width, height, path) {
        try {
            const screenshot = new Shell.Screenshot();
            const [content, scale] = await screenshot.screenshot_stage_to_content();
            const texture = content.get_texture();
            log(`cosnip: CaptureArea stage=${global.stage.width}x${global.stage.height} ` +
                `scale=${scale} texture=${texture.get_width()}x${texture.get_height()} ` +
                `area=${x},${y} ${width}x${height}`);
            const stream = Gio.MemoryOutputStream.new_resizable();
            const pixbuf = await Shell.Screenshot.composite_to_stream(
                texture,
                Math.round(x * scale),
                Math.round(y * scale),
                Math.max(1, Math.round(width * scale)),
                Math.max(1, Math.round(height * scale)),
                scale,
                null, 0, 0, 1,
                stream);
            stream.close(null);
            pixbuf.savev(path, 'png', [], []);
            return [true, path];
        } catch (error) {
            return [false, error.message];
        }
    }

    pasteClipboard() {
        const clipboard = St.Clipboard.get_default();
        clipboard.get_content(
            St.ClipboardType.CLIPBOARD,
            'image/png',
            (_clipboard, bytes) => {
                if (!bytes) {
                    log('cosnip: clipboard does not contain a PNG image');
                    return;
                }
                try {
                    const stream = Gio.MemoryInputStream.new_from_bytes(bytes);
                    const pixbuf = GdkPixbuf.Pixbuf.new_from_stream(stream, null);
                    this._pins.createFromClipboardPixbuf(pixbuf);
                } catch (error) {
                    logError(error, 'cosnip: cannot paste clipboard image');
                }
            });
    }

    openPreferences() {
        try {
            Main.extensionManager.openExtensionPrefs(this.uuid, '', {});
        } catch (error) {
            logError(error, 'cosnip: cannot open preferences');
        }
    }
}
