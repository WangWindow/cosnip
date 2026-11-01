import Clutter from 'gi://Clutter';
import GdkPixbuf from 'gi://GdkPixbuf';
import Gio from 'gi://Gio';
import GLib from 'gi://GLib';
import Meta from 'gi://Meta';
import Shell from 'gi://Shell';
import St from 'gi://St';

import * as Main from 'resource:///org/gnome/shell/ui/main.js';
import { Extension } from 'resource:///org/gnome/shell/extensions/extension.js';

import { NativeOverlay } from './overlay.js';
import { PinManager } from './pins.js';

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
    <method name="DebugState">
      <arg type="s" direction="out" name="state"/>
    </method>
    <method name="WarpClick">
      <arg type="i" direction="in" name="x"/>
      <arg type="i" direction="in" name="y"/>
      <arg type="u" direction="in" name="button"/>
    </method>
    <method name="TestType">
      <arg type="s" direction="in" name="text"/>
    </method>
    <method name="TestScroll">
      <arg type="i" direction="in" name="x"/>
      <arg type="i" direction="in" name="y"/>
      <arg type="i" direction="in" name="step"/>
      <arg type="s" direction="out" name="state"/>
    </method>
    <method name="Pins">
      <arg type="s" direction="out" name="state"/>
    </method>
    <method name="TestDrag">
      <arg type="i" direction="in" name="x1"/>
      <arg type="i" direction="in" name="y1"/>
      <arg type="i" direction="in" name="x2"/>
      <arg type="i" direction="in" name="y2"/>
    </method>
    <method name="Pointer">
      <arg type="i" direction="out" name="x"/>
      <arg type="i" direction="out" name="y"/>
    </method>
    <method name="Selection">
      <arg type="b" direction="out" name="active"/>
      <arg type="i" direction="out" name="x"/>
      <arg type="i" direction="out" name="y"/>
      <arg type="i" direction="out" name="width"/>
      <arg type="i" direction="out" name="height"/>
    </method>
    <method name="Tool">
      <arg type="s" direction="out" name="tool"/>
    </method>
    <method name="TestMotion">
      <arg type="i" direction="in" name="x"/>
      <arg type="i" direction="in" name="y"/>
    </method>
    <method name="HoverWindow">
      <arg type="b" direction="out" name="active"/>
      <arg type="i" direction="out" name="x"/>
      <arg type="i" direction="out" name="y"/>
      <arg type="i" direction="out" name="width"/>
      <arg type="i" direction="out" name="height"/>
    </method>
    <method name="TestRelease"/>
    <method name="TestPress">
      <arg type="i" direction="in" name="x"/>
      <arg type="i" direction="in" name="y"/>
    </method>
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
    return 'cosnip-native/8';
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

  async SaveSelection(path) {
    return await this._extension.saveSelection(path);
  }

  PinSelection() {
    this._extension.pinSelection();
  }

  DebugState() {
    return this._extension.debugState();
  }

  WarpClick(x, y, button) {
    this._extension.warpClick(x, y, button);
  }

  async TestType(text) {
    await this._extension.testType(text);
  }

  async TestScroll(x, y, step) {
    await this._extension.testScroll(x, y, step);
    await new Promise(resolve => {
      GLib.timeout_add(GLib.PRIORITY_DEFAULT, 200, () => {
        resolve();
        return GLib.SOURCE_REMOVE;
      });
    });
    return JSON.stringify(this._extension.pinsState);
  }

  Pins() {
    return JSON.stringify(this._extension.pinsState);
  }

  async TestDrag(x1, y1, x2, y2) {
    await this._extension.testDrag(x1, y1, x2, y2);
  }

  Pointer() {
    return global.get_pointer();
  }

  Selection() {
    const selection = this._extension.overlaySelection;
    if (!selection)
      return [false, 0, 0, 0, 0];
    return [true, Math.round(selection.x), Math.round(selection.y),
      Math.round(selection.w), Math.round(selection.h)];
  }

  Tool() {
    return this._extension.overlayTool;
  }

  TestMotion(x, y) {
    this._extension.testMotion(x, y);
  }

  HoverWindow() {
    const window = this._extension.hoverWindow;
    if (!window)
      return [false, 0, 0, 0, 0];
    return [true, Math.round(window.x), Math.round(window.y),
      Math.round(window.w), Math.round(window.h)];
  }

  TestRelease() {
    this._extension.testRelease();
  }

  TestPress(x, y) {
    this._extension.testPress(x, y);
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

    this._pins = new PinManager(this._settings);
    this._overviewShowingId = Main.overview.connect(
      'showing', () => this._pins.setVisible(false));
    this._overviewHiddenId = Main.overview.connect(
      'hidden', () => this._pins.setVisible(true));

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
    this._pins?.destroy();
    this._pins = null;
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

  get pinCount() {
    return this._pins?.count ?? 0;
  }

  get pinsState() {
    return this._pins?.debugState() ?? [];
  }

  get overlaySelection() {
    return this._overlay?.selection ?? null;
  }

  get overlayTool() {
    return this._overlay?.tool ?? 'closed';
  }

  debugState() {
    return this._overlay?.debugState() ?? '{"overlay":false}';
  }

  _virtualPointerDevice() {
    const seat = global.stage.context.get_backend().get_default_seat();
    if (!this._virtualPointer) {
      this._virtualPointer = seat.create_virtual_device(
        Clutter.InputDeviceType.POINTER_DEVICE);
    }
    return [seat, this._virtualPointer];
  }

  warpClick(x, y, button = 1) {
    try {
      const [seat, device] = this._virtualPointerDevice();
      seat.warp_pointer(x, y);
      const time = GLib.get_monotonic_time();
      device.notify_absolute_motion(time, x, y);
      device.notify_button(time, button, Clutter.ButtonState.PRESSED);
      device.notify_button(time, button, Clutter.ButtonState.RELEASED);
    } catch (error) {
      logError(error, 'cosnip: warp click failed');
    }
  }

  async testType(text) {
    try {
      const seat = global.stage.context.get_backend().get_default_seat();
      if (!this._virtualKeyboard) {
        this._virtualKeyboard = seat.create_virtual_device(
          Clutter.InputDeviceType.KEYBOARD_DEVICE);
      }
      const device = this._virtualKeyboard;
      for (const character of text) {
        const keyval = character === '\n'
          ? Clutter.KEY_Return : character.codePointAt(0);
        let time = Clutter.get_current_event_time() || 0;
        device.notify_keyval(time, keyval, Clutter.KeyState.PRESSED);
        time = Clutter.get_current_event_time() || 0;
        device.notify_keyval(time, keyval, Clutter.KeyState.RELEASED);
        await new Promise(resolve => GLib.timeout_add(
          GLib.PRIORITY_DEFAULT, 40, () => {
            resolve();
            return GLib.SOURCE_REMOVE;
          }));
      }
    } catch (error) {
      logError(error, 'cosnip: test type failed');
    }
  }

  async testDrag(x1, y1, x2, y2) {
    try {
      const [seat, device] = this._virtualPointerDevice();
      seat.warp_pointer(x1, y1);
      let time = Clutter.get_current_event_time() || 0;
      device.notify_absolute_motion(time, x1, y1);
      await new Promise(resolve => GLib.timeout_add(
        GLib.PRIORITY_DEFAULT, 80, () => {
          resolve();
          return GLib.SOURCE_REMOVE;
        }));
      time = Clutter.get_current_event_time() || 0;
      device.notify_button(time, 1, Clutter.ButtonState.PRESSED);
      const steps = 8;
      for (let i = 1; i <= steps; i++) {
        const x = x1 + (x2 - x1) * i / steps;
        const y = y1 + (y2 - y1) * i / steps;
        time = Clutter.get_current_event_time() || 0;
        device.notify_absolute_motion(time, x, y);
        await new Promise(resolve => GLib.timeout_add(
          GLib.PRIORITY_DEFAULT, 40, () => {
            resolve();
            return GLib.SOURCE_REMOVE;
          }));
      }
      time = Clutter.get_current_event_time() || 0;
      device.notify_button(time, 1, Clutter.ButtonState.RELEASED);
    } catch (error) {
      logError(error, 'cosnip: test drag failed');
    }
  }

  async testScroll(x, y, step) {
    try {
      const [seat, device] = this._virtualPointerDevice();
      seat.warp_pointer(x, y);
      device.notify_absolute_motion(GLib.get_monotonic_time(), x, y);
      await new Promise(resolve => GLib.timeout_add(
        GLib.PRIORITY_DEFAULT, 80, () => {
          resolve();
          return GLib.SOURCE_REMOVE;
        }));
      device.notify_scroll_continuous(GLib.get_monotonic_time(), 0, step,
        Clutter.ScrollSource.FINGER, Clutter.ScrollFinishFlags.NONE);
    } catch (error) {
      logError(error, 'cosnip: test scroll failed');
    }
  }

  _captureNativeMonitor(geometry, scale) {
    const screenshot = new Shell.Screenshot();
    const stream = Gio.MemoryOutputStream.new_resizable();
    const bytes = new Promise((resolve) => {
      screenshot.screenshot_area(geometry.x, geometry.y,
        geometry.width, geometry.height, stream, (_shot, result) => {
          try {
            screenshot.screenshot_area_finish(result);
            resolve(stream.steal_as_bytes());
          } catch (_error) {
            resolve(null);
          }
        });
    });
    const pixbuf = bytes.then(value => {
      if (!value)
        return null;
      return new Promise((resolve) => {
        const input = Gio.MemoryInputStream.new_from_bytes(value);
        GdkPixbuf.Pixbuf.new_from_stream_async(input, null,
          (_source, result) => {
            try {
              resolve(GdkPixbuf.Pixbuf.new_from_stream_finish(result));
            } catch (_error) {
              resolve(null);
            }
          });
      });
    });
    return { geometry, scale, pixbuf };
  }

  async startSnip() {
    if (this._overlay) {
      this._overlay.actor?.show();
      return;
    }
    try {
      const stageScale = Math.max(...Array.from(
        { length: global.display.get_n_monitors() },
        (_value, index) => global.display.get_monitor_scale(index) || 1));
      const nativeMonitors = [];
      for (let index = 0; index < global.display.get_n_monitors(); index++) {
        const monitorScale = global.display.get_monitor_scale(index) || 1;
        if (monitorScale + 0.001 >= stageScale)
          continue;
        nativeMonitors.push(this._captureNativeMonitor(
          global.display.get_monitor_geometry(index), monitorScale));
      }
      const screenshot = new Shell.Screenshot();
      const [content, scale, cursorContent, cursorPoint, cursorScale] =
        await screenshot.screenshot_stage_to_content();
      if (!content)
        throw new Error('empty stage content');
      this._overlay = new NativeOverlay({
        content,
        scale,
        cursorContent,
        cursorPoint,
        cursorScale,
        nativeMonitors,
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
      overlay.setSelection({ x, y, w: width, h: height });
      await overlay.saveTo(path);
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
    this._overlay?.setSelection({ x, y, w: width, h: height });
  }

  testMotion(x, y) {
    this._overlay?.testMotion(x, y);
  }

  get hoverWindow() {
    return this._overlay?.hoverWindow ?? null;
  }

  testPress(x, y) {
    this._overlay?.testPress(x, y);
  }

  testRelease() {
    this._overlay?.testRelease();
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
