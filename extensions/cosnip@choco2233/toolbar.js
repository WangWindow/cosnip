import Clutter from 'gi://Clutter';
import GObject from 'gi://GObject';
import St from 'gi://St';
import cairo from 'cairo';

import { Tool } from './annotations.js';

export const PALETTE = [
  [255, 64, 64, 255],
  [255, 196, 0, 255],
  [80, 220, 100, 255],
  [80, 160, 255, 255],
  [200, 120, 255, 255],
  [255, 255, 255, 255],
  [0, 0, 0, 255],
];
export const WIDTHS = [1, 2, 3, 4, 6, 8];

const ICONS = {
  SHAPES: 'shapes',
  LINE: 'line',
  ARROW: 'arrow',
  MARKER: 'marker',
};

const NATIVE_ICONS = {
  pencil: 'document-edit-symbolic',
  mosaic: 'view-grid-symbolic',
  text: 'insert-text-symbolic',
  magnifier: 'edit-find-symbolic',
  eraser: 'edit-clear-symbolic',
  undo: 'edit-undo-symbolic',
  redo: 'edit-redo-symbolic',
  cancel: 'window-close-symbolic',
  pin: 'view-pin-symbolic',
  save: 'document-save-symbolic',
  copy: 'edit-copy-symbolic',
  cursor: 'input-mouse-symbolic',
};

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
    this._mosaicTool = Tool.MOSAIC;
    this._buttons = new Map();
    this._undo = null;
    this._redo = null;
    this._colorIndex = 0;
    this._widthIndex = 1;
    this._hoverButton = null;
    this._cursorVisible = false;
    this._cursorButton = null;

    this._addTool('shapes', ICONS.SHAPES, () => {
      this._shapeTool = Tool.RECT;
      this._callbacks.onTool(this._shapeTool);
    });
    this._addTool('line', ICONS.LINE, () => this._callbacks.onTool(Tool.LINE));
    this._addTool('arrow', ICONS.ARROW, () => this._callbacks.onTool(Tool.ARROW));
    this._addTool('pencil', 'pencil', () => this._callbacks.onTool(Tool.PENCIL));
    this._addTool('marker', ICONS.MARKER, () => this._callbacks.onTool(Tool.MARKER));
    this._addTool('mosaic', 'mosaic', () => {
      this._mosaicTool = this._mosaicTool === Tool.MOSAIC
        ? Tool.BLUR : Tool.MOSAIC;
      this._callbacks.onTool(this._mosaicTool);
    });
    this._addTool('text', 'text', () => this._callbacks.onTool(Tool.TEXT));
    this._addTool('magnifier', 'magnifier',
      () => this._callbacks.onTool(Tool.MAGNIFIER));
    this._addTool('eraser', 'eraser',
      () => this._callbacks.onTool(Tool.ERASER));
    this._colorButton = new ColorButton(PALETTE[this._colorIndex],
      () => this._callbacks.onColorPicker?.(this._colorButton));
    this._colorButton.reactive = false;
    this.add_child(this._colorButton);
    this._widthButton = new WidthButton(WIDTHS[this._widthIndex],
      () => this._callbacks.onWidthPicker?.(this._widthButton));
    this._widthButton.reactive = false;
    this.add_child(this._widthButton);

    this._separator();
    this._undo = this._addTool('undo', 'undo', () => this._callbacks.onUndo());
    this._redo = this._addTool('redo', 'redo', () => this._callbacks.onRedo());
    this._separator();
    this._cursorButton = this._addTool('cursor', 'cursor', () => {
      this._cursorVisible = !this._cursorVisible;
      this.setCursorVisible(this._cursorVisible);
      this._callbacks.onCursor?.(this._cursorVisible);
    });
    this._addTool('cancel', 'cancel', () => this._callbacks.onCancel());
    this._addTool('pin', 'pin', () => this._callbacks.onPin());
    this._addTool('save', 'save', () => this._callbacks.onSave());
    this._addTool('copy', 'copy', () => this._callbacks.onCopy());
  }

  _addTool(name, icon, handler) {
    const button = new ToolButton(icon, handler);
    button.reactive = false;
    button.track_hover = false;
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
    this.setActive('mosaic', tool === Tool.MOSAIC || tool === Tool.BLUR);
    this.setActive('text', tool === Tool.TEXT);
    this.setActive('magnifier', tool === Tool.MAGNIFIER);
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

  setUndo(enabled) {
    this._undo?.setEnabled(enabled);
  }

  setRedo(enabled) {
    this._redo?.setEnabled(enabled);
  }

  setCursorVisible(visible) {
    this._cursorVisible = visible;
    this._cursorButton?.setActive(visible);
  }

  updateHover(point) {
    let target = null;
    if (this.visible) {
      for (const child of this.get_children()) {
        if (!(child instanceof St.Button) || !child.visible)
          continue;
        const [x, y] = child.get_transformed_position();
        const [w, h] = child.get_transformed_size();
        if (point.x >= x && point.x < x + w &&
          point.y >= y && point.y < y + h) {
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
  }

  clearHover() {
    if (!this._hoverButton)
      return;
    this._hoverButton.remove_style_class_name('cosnip-tool-hover');
    this._hoverButton = null;
  }

  buttonAt(point) {
    if (!this.visible)
      return null;
    for (const child of this.get_children()) {
      if (!(child instanceof St.Button) || !child.visible)
        continue;
      const [x, y] = child.get_transformed_position();
      const [w, h] = child.get_transformed_size();
      if (point.x >= x && point.x < x + w &&
        point.y >= y && point.y < y + h)
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

  setStyle(color, width) {
    const colorIndex = PALETTE.findIndex(entry =>
      entry[0] === color[0] && entry[1] === color[1] &&
      entry[2] === color[2] && entry[3] === color[3]);
    if (colorIndex >= 0) {
      this._colorIndex = colorIndex;
      this._colorButton?.setColor(color);
    }
    const widthIndex = WIDTHS.indexOf(width);
    if (widthIndex >= 0) {
      this._widthIndex = widthIndex;
      this._widthButton?.setWidth(width);
    }
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
    this._color = color;
    this._area.queue_repaint();
  }
});

const WidthButton = GObject.registerClass({
  GTypeName: 'CosnipWidthButton',
}, class WidthButton extends St.Button {
  _init(width, handler) {
    super._init({
      style_class: 'cosnip-tool-button',
      reactive: true,
      track_hover: true,
      can_focus: false,
    });
    this._width = width;
    this._area = new St.DrawingArea({
      width: 20,
      height: 20,
      y_align: Clutter.ActorAlign.CENTER,
      x_align: Clutter.ActorAlign.CENTER,
    });
    this._area.connect('repaint', area => {
      const cr = area.get_context();
      cr.setSourceRGBA(0.18, 0.18, 0.18, 1);
      cr.setLineWidth(Math.max(1, this._width * 0.8));
      cr.moveTo(4, 10);
      cr.lineTo(20, 10);
      cr.stroke();
    });
    this.set_child(this._area);
    this.connect('clicked', handler);
    this.accessible_name = 'Line width';
  }

  setWidth(width) {
    this._width = width;
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
    this._nativeIcon = NATIVE_ICONS[icon] ?? null;
    if (this._nativeIcon) {
      this._area = new St.Icon({
        icon_name: this._nativeIcon,
        icon_size: 16,
        style_class: 'cosnip-tool-icon',
        y_align: Clutter.ActorAlign.CENTER,
        x_align: Clutter.ActorAlign.CENTER,
      });
    } else {
      this._area = new St.DrawingArea({
        width: 20,
        height: 20,
        y_align: Clutter.ActorAlign.CENTER,
        x_align: Clutter.ActorAlign.CENTER,
      });
      this._area.connect('repaint', area => {
        const cr = area.get_context();
        paintIcon(cr, this._icon, area.width || 20,
          this._active ? [1, 1, 1, 1] : [0.18, 0.18, 0.18, 1]);
      });
    }
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
    if (!this._nativeIcon)
      this._area.queue_repaint();
  }

  setEnabled(enabled) {
    if (this._enabled === enabled)
      return;
    this._enabled = enabled;
    this.opacity = enabled ? 255 : 120;
  }
});

function paintIcon(cr, icon, size, color) {
  const scale = size / 24;
  cr.save();
  cr.scale(scale, scale);
  cr.setLineWidth(1.7);
  cr.setLineCap(cairo.LineCap.ROUND);
  cr.setLineJoin(cairo.LineJoin.ROUND);
  cr.setSourceRGBA(...color);
  const line = (x1, y1, x2, y2) => {
    cr.moveTo(x1, y1);
    cr.lineTo(x2, y2);
  };
  const rect = (x, y, w, h) => cr.rectangle(x, y, w, h);
  switch (icon) {
    case ICONS.SHAPES:
      rect(3.5, 5.5, 17, 13);
      cr.stroke();
      break;
    case ICONS.LINE:
      line(4, 18, 20, 6);
      cr.stroke();
      rect(2.5, 16.5, 3, 3);
      cr.fill();
      rect(18.5, 4.5, 3, 3);
      cr.fill();
      break;
    case ICONS.ARROW:
      line(5, 19, 19, 5);
      cr.moveTo(13.5, 5);
      cr.lineTo(19, 5);
      cr.lineTo(19, 10.5);
      cr.stroke();
      break;
    case ICONS.MARKER:
      line(5, 19, 17, 7);
      cr.setLineWidth(4.5);
      cr.stroke();
      cr.setLineWidth(1.7);
      cr.moveTo(16, 6);
      cr.lineTo(20, 10);
      cr.stroke();
      break;
    default:
      break;
  }
  cr.restore();
}
