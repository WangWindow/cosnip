import Clutter from 'gi://Clutter';
import Gio from 'gi://Gio';
import GObject from 'gi://GObject';
import St from 'gi://St';
import cairo from 'cairo';

import { Tool } from './annotations.js';

const ICON_BASE = import.meta.url.replace(/\/[^/]+$/, '');

export const PALETTE = [
  [255, 64, 64, 255],
  [255, 196, 0, 255],
  [80, 220, 100, 255],
  [80, 160, 255, 255],
  [200, 120, 255, 255],
  [255, 255, 255, 255],
  [0, 0, 0, 255],
];

const TOOL_ICONS = {
  shapes: 'square.svg',
  line: 'slash.svg',
  arrow: 'arrow-up-right.svg',
  pencil: 'pencil.svg',
  marker: 'highlighter.svg',
  mosaic: 'grid-3x3.svg',
  text: 'type.svg',
  eraser: 'eraser.svg',
  cursor: 'mouse-pointer-2.svg',
  cancel: 'x.svg',
  pin: 'pin.svg',
  save: 'save.svg',
  copy: 'copy.svg',
};

function containsPoint(actor, point) {
  const [x, y] = actor.get_transformed_position();
  const [w, h] = actor.get_transformed_size();
  return point.x >= x && point.x < x + w &&
    point.y >= y && point.y < y + h;
}

export const Toolbar = GObject.registerClass({
  GTypeName: 'CosnipToolbar',
}, class Toolbar extends St.BoxLayout {
  _init(callbacks) {
    super._init({
      style_class: 'cosnip-toolbar',
      orientation: Clutter.Orientation.HORIZONTAL,
      reactive: true,
      track_hover: false,
      cursor_type: Clutter.CursorType.POINTER,
    });
    this._callbacks = callbacks;
    this._tool = Tool.SELECT;
    this._shapeTool = Tool.RECT;
    this._buttons = new Map();
    this._colorIndex = 0;
    this._hoverButton = null;
    this._cursorVisible = false;
    this._cursorButton = null;

    this._addTool('shapes', 'shapes',
      () => this._toggleTool(Tool.RECT), 'Rectangle');
    this._addTool('line', 'line', () => this._toggleTool(Tool.LINE), 'Line');
    this._addTool('arrow', 'arrow', () => this._toggleTool(Tool.ARROW),
      'Arrow');
    this._addTool('pencil', 'pencil', () => this._toggleTool(Tool.PENCIL),
      'Pencil');
    this._addTool('marker', 'marker', () => this._toggleTool(Tool.MARKER),
      'Highlighter');
    this._addTool('mosaic', 'mosaic', () => this._toggleTool(Tool.MOSAIC),
      'Mosaic');
    this._addTool('text', 'text', () => this._toggleTool(Tool.TEXT), 'Text');
    this._addTool('eraser', 'eraser', () => this._toggleTool(Tool.ERASER),
      'Eraser');
    this._colorButton = new ColorButton(PALETTE[0], () => {
      this._colorIndex = (this._colorIndex + 1) % PALETTE.length;
      const color = PALETTE[this._colorIndex];
      this._colorButton.setColor(color);
      this._callbacks.onColor?.(color);
    });
    this._colorButton.reactive = false;
    this._colorButton._tooltip = 'Color';
    this.add_child(this._colorButton);

    this._separator();
    this._cursorButton = this._addTool('cursor', 'cursor', () => {
      this._cursorVisible = !this._cursorVisible;
      this.setCursorVisible(this._cursorVisible);
      this._callbacks.onCursor?.(this._cursorVisible);
    }, 'Capture cursor');
    this._addTool('cancel', 'cancel', () => this._callbacks.onCancel(),
      'Cancel');
    this._addTool('pin', 'pin', () => this._callbacks.onPin(), 'Pin');
    this._addTool('save', 'save', () => this._callbacks.onSave(), 'Save');
    this._addTool('copy', 'copy', () => this._callbacks.onCopy(), 'Copy');
  }

  _toggleTool(tool) {
    const next = this._tool === tool ? Tool.SELECT : tool;
    this._callbacks.onTool(next);
  }

  _addTool(name, icon, handler, tooltip = name) {
    const button = new ToolButton(icon, handler);
    button.reactive = false;
    button.track_hover = false;
    button._tooltip = tooltip;
    this._buttons.set(name, button);
    this.add_child(button);
    return button;
  }

  _separator() {
    this.add_child(new St.Widget({
      style_class: 'cosnip-separator',
      width: 1,
      height: 18,
      y_align: Clutter.ActorAlign.CENTER,
    }));
  }

  setTool(tool) {
    this._tool = tool;
    this.setActive('shapes', tool === Tool.RECT);
    this.setActive('line', tool === Tool.LINE);
    this.setActive('arrow', tool === Tool.ARROW);
    this.setActive('pencil', tool === Tool.PENCIL);
    this.setActive('marker', tool === Tool.MARKER);
    this.setActive('mosaic', tool === Tool.MOSAIC);
    this.setActive('text', tool === Tool.TEXT);
    this.setActive('eraser', tool === Tool.ERASER);
  }

  syncShape(tool) {
    if (tool === Tool.RECT)
      this._shapeTool = tool;
    this.setTool(tool);
  }

  setActive(name, active) {
    const button = this._buttons.get(name);
    if (button)
      button.setActive(active);
  }

  setCursorVisible(visible) {
    this._cursorVisible = visible;
    this._cursorButton?.setActive(visible);
  }

  setSaving(saving) {
    const button = this._buttons.get('save');
    if (!button)
      return;
    button.setEnabled(!saving);
    button._tooltip = saving ? 'Saving…' : 'Save';
  }

  updateHover(point) {
    let target = null;
    if (this.visible && containsPoint(this, point)) {
      for (const child of this.get_children()) {
        if (!(child instanceof St.Button) || !child.visible)
          continue;
        if (containsPoint(child, point)) {
          target = child;
          break;
        }
      }
    }
    if (target === this._hoverButton)
      return;
    this._hoverButton?.remove_style_class_name('cosnip-tool-hover');
    this._hoverButton = target;
    this._hoverButton?.add_style_class_name('cosnip-tool-hover');
    this._callbacks.onHover?.(target?._tooltip ?? null, target);
  }

  clearHover() {
    if (!this._hoverButton)
      return;
    this._hoverButton.remove_style_class_name('cosnip-tool-hover');
    this._hoverButton = null;
    this._callbacks.onHover?.(null, null);
  }

  buttonAt(point) {
    if (!this.visible || !containsPoint(this, point))
      return null;
    for (const child of this.get_children()) {
      if (!(child instanceof St.Button) || !child.visible)
        continue;
      if (containsPoint(child, point))
        return child;
    }
    return null;
  }

  clickAt(point) {
    const button = this.buttonAt(point);
    if (!button)
      return false;
    button.emit('clicked', button);
    return true;
  }

  setStyle(color) {
    this._colorButton?.setColor(color);
  }

  get styleDisplay() {
    return { color: [...this._colorButton._color] };
  }
});

const ColorButton = GObject.registerClass({
  GTypeName: 'CosnipColorButton',
}, class ColorButton extends St.Button {
  _init(color, handler) {
    super._init({
      style_class: 'cosnip-tool-button',
      reactive: true,
      track_hover: true,
      can_focus: false,
    });
    this._color = color;
    this._area = new St.DrawingArea({
      width: 20,
      height: 20,
      y_align: Clutter.ActorAlign.CENTER,
      x_align: Clutter.ActorAlign.CENTER,
    });
    this._area.connect('repaint', area => {
      const cr = area.get_context();
      cr.setSourceRGBA(
        this._color[0] / 255, this._color[1] / 255,
        this._color[2] / 255, this._color[3] / 255);
      cr.rectangle(4, 4, 12, 12);
      cr.fillPreserve();
      cr.setSourceRGBA(0, 0, 0, 0.55);
      cr.setLineWidth(1);
      cr.stroke();
    });
    this.set_child(this._area);
    this.connect('clicked', handler);
    this.accessible_name = 'Color';
  }

  setColor(color) {
    if (this._color.every((channel, index) => channel === color[index]))
      return;
    this._color = [...color];
    this._area.queue_repaint();
  }
});

const ToolButton = GObject.registerClass({
  GTypeName: 'CosnipToolButton',
}, class ToolButton extends St.Button {
  _init(icon, handler) {
    super._init({
      style_class: 'cosnip-tool-button',
      reactive: true,
      track_hover: true,
      can_focus: false,
    });
    this._icon = icon;
    this.accessible_name = icon;
    this._active = false;
    this._enabled = true;
    const fileIcon = TOOL_ICONS[icon] ?? null;
    const iconProps = {
      icon_size: 16,
      style_class: 'cosnip-tool-icon',
      y_align: Clutter.ActorAlign.CENTER,
      x_align: Clutter.ActorAlign.CENTER,
    };
    if (fileIcon)
      iconProps.gicon = Gio.icon_new_for_string(`${ICON_BASE}/icons/${fileIcon}`);
    else
      iconProps.icon_name = 'image-missing-symbolic';
    this._area = new St.Icon(iconProps);
    this.set_child(this._area);
    this.connect('clicked', () => {
      if (this._enabled)
        handler();
    });
  }

  setActive(active) {
    if (this._active === active)
      return;
    this._active = active;
    if (active)
      this.add_style_class_name('cosnip-tool-active');
    else
      this.remove_style_class_name('cosnip-tool-active');

  }

  setEnabled(enabled) {
    if (this._enabled === enabled)
      return;
    this._enabled = enabled;
    this.opacity = enabled ? 255 : 120;
  }
});
