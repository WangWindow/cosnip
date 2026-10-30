import Clutter from 'gi://Clutter';
import GObject from 'gi://GObject';
import St from 'gi://St';
import cairo from 'cairo';

import {Tool} from './annotations.js';

const PALETTE = [
    [255, 64, 64, 255],
    [255, 196, 0, 255],
    [80, 220, 100, 255],
    [80, 160, 255, 255],
    [200, 120, 255, 255],
    [255, 255, 255, 255],
    [0, 0, 0, 255],
];
const WIDTHS = [1, 3, 6];

const ICONS = {
    SHAPES: 'shapes',
    ARROW: 'arrow',
    PENCIL: 'pencil',
    MARKER: 'marker',
    MOSAIC: 'mosaic',
    TEXT: 'text',
    ERASER: 'eraser',
    UNDO: 'undo',
    REDO: 'redo',
    CANCEL: 'cancel',
    PIN: 'pin',
    SAVE: 'save',
    COPY: 'copy',
};

export const Toolbar = GObject.registerClass({
    GTypeName: 'CosnipToolbar',
}, class Toolbar extends St.BoxLayout {
    _init(callbacks) {
        super._init({
            style_class: 'cosnip-toolbar',
            orientation: Clutter.Orientation.HORIZONTAL,
            reactive: true,
            track_hover: true,
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

        this._addTool('shapes', ICONS.SHAPES, () => {
            this._shapeTool = this._shapeTool === Tool.RECT
                ? Tool.ELLIPSE
                : this._shapeTool === Tool.ELLIPSE ? Tool.LINE : Tool.RECT;
            this._callbacks.onTool(this._shapeTool);
        });
        this._addTool('arrow', ICONS.ARROW, () => this._callbacks.onTool(Tool.ARROW));
        this._addTool('pencil', ICONS.PENCIL, () => this._callbacks.onTool(Tool.PENCIL));
        this._addTool('marker', ICONS.MARKER, () => this._callbacks.onTool(Tool.MARKER));
        this._addTool('mosaic', ICONS.MOSAIC, () => {
            this._mosaicTool = this._mosaicTool === Tool.MOSAIC
                ? Tool.BLUR : Tool.MOSAIC;
            this._callbacks.onTool(this._mosaicTool);
        });
        this._addTool('text', ICONS.TEXT, () => this._callbacks.onTool(Tool.TEXT));
        this._addTool('eraser', ICONS.ERASER, () => this._callbacks.onTool(Tool.ERASER));
        this._colorButton = new ColorButton(PALETTE[this._colorIndex], () => {
            this._colorIndex = (this._colorIndex + 1) % PALETTE.length;
            this._colorButton.setColor(PALETTE[this._colorIndex]);
            this._callbacks.onColor(PALETTE[this._colorIndex]);
        });
        this.add_child(this._colorButton);
        this._widthButton = new WidthButton(WIDTHS[this._widthIndex], () => {
            this._widthIndex = (this._widthIndex + 1) % WIDTHS.length;
            this._widthButton.setWidth(WIDTHS[this._widthIndex]);
            this._callbacks.onWidth(WIDTHS[this._widthIndex]);
        });
        this.add_child(this._widthButton);

        this._separator();
        this._undo = this._addTool('undo', ICONS.UNDO, () => this._callbacks.onUndo());
        this._redo = this._addTool('redo', ICONS.REDO, () => this._callbacks.onRedo());
        this._separator();
        this._addTool('cancel', ICONS.CANCEL, () => this._callbacks.onCancel());
        this._addTool('pin', ICONS.PIN, () => this._callbacks.onPin());
        this._addTool('save', ICONS.SAVE, () => this._callbacks.onSave());
        this._addTool('copy', ICONS.COPY, () => this._callbacks.onCopy());
    }

    _addTool(name, icon, handler) {
        const button = new ToolButton(icon, handler);
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
        this.setActive('shapes', tool === Tool.RECT ||
            tool === Tool.ELLIPSE || tool === Tool.LINE);
        this.setActive('arrow', tool === Tool.ARROW);
        this.setActive('pencil', tool === Tool.PENCIL);
        this.setActive('marker', tool === Tool.MARKER);
        this.setActive('mosaic', tool === Tool.MOSAIC || tool === Tool.BLUR);
        this.setActive('text', tool === Tool.TEXT);
        this.setActive('eraser', tool === Tool.ERASER);
    }

    syncShape(tool) {
        if (tool === Tool.RECT || tool === Tool.ELLIPSE ||
            tool === Tool.LINE)
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

    clickAt(point) {
        if (!this.visible)
            return false;
        for (const child of this.get_children()) {
            if (!(child instanceof St.Button) || !child.visible)
                continue;
            const [x, y] = child.get_transformed_position();
            const [w, h] = child.get_transformed_size();
            if (point.x >= x && point.x < x + w &&
                point.y >= y && point.y < y + h) {
                child.emit('clicked', child);
                return true;
            }
        }
        return false;
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
        this._active = false;
        this._enabled = true;
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
    const s = size / 24;
    cr.save();
    cr.scale(s, s);
    cr.setLineWidth(1.6);
    cr.setLineCap(cairo.LineCap.ROUND);
    cr.setLineJoin(cairo.LineJoin.ROUND);
    cr.setSourceRGBA(...color);
    const line = (x1, y1, x2, y2) => {
        cr.moveTo(x1, y1);
        cr.lineTo(x2, y2);
    };
    const rect = (x, y, w, h) => cr.rectangle(x, y, w, h);
    const ellipse = (cx, cy, rx, ry) => {
        cr.save();
        cr.translate(cx, cy);
        cr.scale(rx, ry);
        cr.arc(0, 0, 1, 0, Math.PI * 2);
        cr.restore();
    };
    switch (icon) {
    case ICONS.SHAPES:
        rect(2.5, 7, 9, 13);
        ellipse(17, 13, 4.3, 3.5);
        cr.stroke();
        cr.setSourceRGBA(235 / 255, 55 / 255, 55 / 255, 1);
        cr.arc(20.5, 5.5, 1.8, 0, Math.PI * 2);
        cr.fill();
        break;
    case ICONS.ARROW:
        line(5, 19, 19, 5);
        cr.moveTo(14, 5);
        cr.lineTo(19, 5);
        cr.lineTo(19, 10);
        cr.stroke();
        break;
    case ICONS.PENCIL:
        cr.moveTo(5, 19);
        cr.lineTo(7, 12);
        cr.lineTo(16, 3);
        cr.lineTo(21, 8);
        cr.lineTo(12, 17);
        cr.closePath();
        line(7, 12, 12, 17);
        cr.stroke();
        break;
    case ICONS.MARKER:
        line(5, 19, 17, 7);
        cr.setLineWidth(5);
        cr.stroke();
        cr.setLineWidth(1.6);
        cr.moveTo(16, 6);
        cr.lineTo(20, 10);
        cr.stroke();
        break;
    case ICONS.MOSAIC:
        rect(4, 4, 16, 16);
        cr.stroke();
        cr.rectangle(4, 4, 8, 8);
        cr.fill();
        cr.rectangle(12, 12, 8, 8);
        cr.fill();
        break;
    case ICONS.TEXT:
        line(5, 5, 19, 5);
        line(12, 5, 12, 20);
        cr.stroke();
        break;
    case ICONS.ERASER:
        cr.moveTo(5, 17);
        cr.lineTo(13, 6);
        cr.lineTo(20, 12);
        cr.lineTo(12, 22);
        cr.closePath();
        cr.stroke();
        line(9.5, 10.5, 16.5, 16.5);
        cr.stroke();
        break;
    case ICONS.UNDO:
        cr.arc(12, 13, 7, Math.PI * 1.05, Math.PI * 2);
        cr.stroke();
        cr.moveTo(5, 13);
        cr.lineTo(9.5, 8.5);
        cr.lineTo(10, 15);
        cr.closePath();
        cr.fill();
        break;
    case ICONS.REDO:
        cr.arc(12, 13, 7, Math.PI * 0.95, Math.PI * 1.95);
        cr.stroke();
        cr.moveTo(19, 13);
        cr.lineTo(14.5, 8.5);
        cr.lineTo(14, 15);
        cr.closePath();
        cr.fill();
        break;
    case ICONS.CANCEL:
        line(5, 5, 19, 19);
        line(19, 5, 5, 19);
        cr.stroke();
        break;
    case ICONS.PIN:
        cr.arc(15, 8, 3.1, 0, Math.PI * 2);
        cr.fill();
        line(13.5, 10.5, 6.5, 18);
        line(9.5, 11.5, 13, 15.5);
        cr.stroke();
        break;
    case ICONS.SAVE:
        rect(4, 4, 16, 16);
        rect(8, 4, 8, 6);
        rect(7, 13, 10, 7);
        cr.stroke();
        break;
    case ICONS.COPY:
        rect(4, 4, 11, 11);
        rect(9, 9, 11, 11);
        cr.stroke();
        break;
    default:
        break;
    }
    cr.restore();
}
