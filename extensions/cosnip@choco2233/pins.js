import Clutter from 'gi://Clutter';
import Shell from 'gi://Shell';
import St from 'gi://St';
import cairo from 'cairo';

import * as Main from 'resource:///org/gnome/shell/ui/main.js';

import { pixbufToImageContent, surfaceToPixbuf } from './util.js';

const PIN_BORDER_WIDTH = 1;
const PIN_BORDER_COLOR = [0, 0, 0, 140];
const PIN_ZOOM_STEP = 0.12;
const PIN_MIN_WIDTH = 32;

export class PinManager {
  constructor(settings) {
    this._settings = settings;
    this._pins = new Map();
    this._lastWheelEvent = null;
    this._nextId = 1;
    this._stageScrollId = global.stage.connect(
      'captured-event', (_actor, event) => {
        if (event.type() !== Clutter.EventType.SCROLL)
          return Clutter.EVENT_PROPAGATE;
        return this._onStageScroll(event);
      });
  }

  get count() {
    return this._pins.size;
  }

  debugState() {
    return [...this._pins.values()].map(({ actor }) => ({
      x: Math.round(actor.x),
      y: Math.round(actor.y),
      width: Math.round(actor.width),
      height: Math.round(actor.height),
    }));
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
      track_hover: false,
      cursor_type: Clutter.CursorType.DEFAULT,
    });
    const image = new St.Widget({
      content,
      width,
      height,
      reactive: false,
    });
    image.set_content_scaling_filters(
      Clutter.ScalingFilter.NEAREST, Clutter.ScalingFilter.NEAREST);
    actor.add_child(image);
    const border = new St.DrawingArea({
      width,
      height,
      reactive: false,
    });
    border.connect('repaint', area => drawPinBorder(area));
    actor.add_child(border);
    actor.set_pivot_point(0, 0);
    actor.set_position(Math.round(x), Math.round(y));
    actor.opacity = Math.round(
      Math.max(10, Math.min(100, this._settings.get_int('pin-opacity'))) * 2.55);

    const id = this._nextId++;
    const pin = {
      actor,
      content,
      image,
      border,
      pixelWidth: pixbuf.get_width(),
      pixelHeight: pixbuf.get_height(),
    };
    let drag = null;
    actor.connect('button-press-event', (_actor, event) => {
      if (event.get_button() === 3) {
        this.remove(id);
        return Clutter.EVENT_STOP;
      }
      if (event.get_button() === 1) {
        const [x, y] = event.get_coords();
        drag = { x, y, actorX: actor.x, actorY: actor.y };
        return Clutter.EVENT_STOP;
      }
      return Clutter.EVENT_PROPAGATE;
    });
    actor.connect('motion-event', (_actor, event) => {
      if (!drag)
        return Clutter.EVENT_PROPAGATE;
      const [x, y] = event.get_coords();
      actor.set_position(
        drag.actorX + x - drag.x,
        drag.actorY + y - drag.y);
      return Clutter.EVENT_STOP;
    });
    actor.connect('button-release-event', (_actor, event) => {
      if (event.get_button() === 1 && drag) {
        drag = null;
        return Clutter.EVENT_STOP;
      }
      return Clutter.EVENT_PROPAGATE;
    });
    actor.connect('scroll-event', (_actor, event) => {
      if (!this._zoomFromEvent(pin, event))
        return Clutter.EVENT_PROPAGATE;
      return Clutter.EVENT_STOP;
    });

    this._pins.set(id, pin);
    Main.layoutManager.addTopChrome(actor);
    const uiGroup = Main.layoutManager.uiGroup;
    const overlayGroup = Main.layoutManager.screenshotUIGroup;
    if (overlayGroup && uiGroup.contains(overlayGroup))
      uiGroup.set_child_below_sibling(actor, overlayGroup);
    actor.visible = !Main.overview.visible;
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

  _onStageScroll(event) {
    if (this._pins.size === 0 ||
        Main.actionMode !== Shell.ActionMode.NORMAL)
      return Clutter.EVENT_PROPAGATE;
    const [x, y] = event.get_coords();
    const pin = this._pinAt(x, y);
    if (!pin || !this._zoomFromEvent(pin, event))
      return Clutter.EVENT_PROPAGATE;
    return Clutter.EVENT_STOP;
  }

  _pinAt(x, y) {
    for (const pin of [...this._pins.values()].reverse()) {
      const { actor } = pin;
      if (actor.visible && x >= actor.x && x < actor.x + actor.width &&
        y >= actor.y && y < actor.y + actor.height)
        return pin;
    }
    return null;
  }

  _zoomFromEvent(pin, event) {
    const direction = event.get_scroll_direction();
    let delta = 0;
    if (direction === Clutter.ScrollDirection.SMOOTH) {
      const [dx, dy] = event.get_scroll_delta();
      const raw = dy !== 0 ? dy : -dx;
      const magnitude = Math.max(0.25, Math.min(1.5, Math.abs(raw)));
      delta = raw === 0 ? 0 : -Math.sign(raw) * magnitude;
    } else if (direction === Clutter.ScrollDirection.UP) {
      delta = 1;
    } else if (direction === Clutter.ScrollDirection.DOWN) {
      delta = -1;
    }
    if (delta === 0)
      return false;
    if (event.get_scroll_source() === Clutter.ScrollSource.WHEEL) {
      // Mutter can emit a smooth event and a discrete event for one wheel step.
      const previous = this._lastWheelEvent;
      const current = {
        pin,
        time: event.get_time(),
        smooth: direction === Clutter.ScrollDirection.SMOOTH,
        sign: Math.sign(delta),
      };
      if (previous && previous.pin === pin && previous.time === current.time &&
          previous.smooth !== current.smooth && previous.sign === current.sign) {
        this._lastWheelEvent = null;
        return true;
      }
      this._lastWheelEvent = current;
    }
    const [x, y] = event.get_coords();
    this._zoomPin(pin, Math.max(-2, Math.min(2, delta)), x, y);
    return true;
  }

  _zoomPin(pin, delta, pointerX, pointerY) {
    const { actor, image, border, pixelWidth, pixelHeight } = pin;
    const ratio = pixelWidth / pixelHeight;
    const nextWidth = Math.min(
      pixelWidth * 4,
      Math.max(PIN_MIN_WIDTH,
        actor.width * Math.exp(delta * PIN_ZOOM_STEP)));
    const nextHeight = nextWidth / ratio;
    const x = pointerX - (pointerX - actor.x) * nextWidth / actor.width;
    const y = pointerY - (pointerY - actor.y) * nextHeight / actor.height;
    actor.set_position(x, y);
    actor.set_size(nextWidth, nextHeight);
    image.set_size(nextWidth, nextHeight);
    border.set_size(nextWidth, nextHeight);
    border.queue_repaint();
  }

  remove(id) {
    const pin = this._pins.get(id);
    if (!pin)
      return;
    if (this._lastWheelEvent?.pin === pin)
      this._lastWheelEvent = null;
    this._pins.delete(id);
    Main.layoutManager.removeChrome(pin.actor);
    pin.actor.destroy();
    pin.content.run_dispose?.();
  }

  clear() {
    for (const id of [...this._pins.keys()])
      this.remove(id);
  }

  destroy() {
    this.clear();
    if (this._stageScrollId) {
      global.stage.disconnect(this._stageScrollId);
      this._stageScrollId = 0;
    }
  }

  setVisible(visible) {
    for (const { actor } of this._pins.values())
      actor.visible = visible;
  }
}

function drawPinBorder(area) {
  const cr = area.get_context();
  const width = area.width;
  const height = area.height;
  if (width < 2 || height < 2)
    return;
  cr.setSourceRGBA(
    PIN_BORDER_COLOR[0] / 255, PIN_BORDER_COLOR[1] / 255,
    PIN_BORDER_COLOR[2] / 255, PIN_BORDER_COLOR[3] / 255);
  cr.setLineWidth(PIN_BORDER_WIDTH);
  cr.rectangle(
    PIN_BORDER_WIDTH / 2, PIN_BORDER_WIDTH / 2,
    width - PIN_BORDER_WIDTH, height - PIN_BORDER_WIDTH);
  cr.stroke();
  cr.setSourceRGBA(1, 1, 1, 0.18);
  cr.setLineWidth(1);
  cr.rectangle(1.5, 1.5, width - 3, height - 3);
  cr.stroke();
}
