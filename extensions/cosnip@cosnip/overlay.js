import Clutter from 'gi://Clutter';
import Gio from 'gi://Gio';
import GLib from 'gi://GLib';
import Shell from 'gi://Shell';
import St from 'gi://St';
import cairo from 'cairo';

import * as Main from 'resource:///org/gnome/shell/ui/main.js';

import {
    Tool,
    annotationIsValid,
    createAnnotation,
    drawAnnotations,
    updateAnnotation,
} from './annotations.js';
import {Toolbar} from './toolbar.js';
import {
    copySurfaceToClipboard,
    defaultSaveDir,
    expandFilename,
    paintPixbuf,
    parseColor,
    saveSurface,
    uniquePath,
} from './util.js';

const HANDLE_RADIUS = 6;
const SELECTION_COLOR = [0.23, 0.51, 0.96, 1.0];

export class NativeOverlay {
    constructor(options) {
        this._content = options.content;
        this._scale = options.scale || 1;
        this._settings = options.settings;
        this._onClose = options.onClose;
        this._onPin = options.onPin;
        this._sourcePixbuf = null;
        this._annotations = [];
        this._undo = [];
        this._redo = [];
        this._draft = null;
        this._selection = null;
        this._tool = Tool.SELECT;
        this._style = {
            color: parseColor(this._settings.get_string('annotation-color')),
            width: this._settings.get_int('annotation-width'),
            fontSize: this._settings.get_int('font-size'),
        };
        this._drag = null;
        this._textEntry = null;
        this._closed = false;
        this._root = null;
        this._toolbar = null;
        this._sizeBadge = null;
        this._tips = null;
    }

    async open() {
        try {
            this._sourcePixbuf = await this._capturePixbuf();
        } catch (error) {
            logError(error, 'cosnip: cannot get screenshot pixels');
        }
        try {
            this._buildUI();
        } catch (error) {
            this._close();
            throw error;
        }
    }

    get actor() {
        return this._root;
    }

    setSelection(selection) {
        this._selection = clampSelection(selection);
        this._layoutChrome();
        this._repaint();
    }

    addRect(x, y, w, h) {
        this._addAnnotation({
            kind: Tool.RECT,
            x, y, w, h,
            color: this._style.color,
            width: this._style.width,
        });
        this._repaint();
    }

    addMosaic(x1, y1, x2, y2) {
        this._addAnnotation({
            kind: Tool.MOSAIC,
            points: [{x: x1, y: y1}, {x: x2, y: y2}],
            color: this._style.color,
            width: this._style.width,
        });
        this._repaint();
    }

    addText(text, x, y) {
        this._addAnnotation({
            kind: Tool.TEXT,
            x, y, text,
            size: this._style.fontSize,
            color: this._style.color,
            width: this._style.width,
        });
        this._repaint();
    }

    addEraser(x1, y1, x2, y2) {
        this._addAnnotation({
            kind: Tool.ERASER,
            points: [{x: x1, y: y1}, {x: x2, y: y2}],
            color: this._style.color,
            width: this._style.width,
        });
        this._repaint();
    }

    pinSelection() {
        this._pin();
    }

    saveTo(path) {
        const surface = this._renderSelection();
        if (!surface)
            return false;
        saveSurface(surface, path);
        return true;
    }

    async _capturePixbuf() {
        const texture = this._content.get_texture();
        const width = Math.round(global.stage.width * this._scale);
        const height = Math.round(global.stage.height * this._scale);
        log(`cosnip: stage=${global.stage.width}x${global.stage.height} ` +
            `scale=${this._scale} texture=${texture.get_width()}x${texture.get_height()} ` +
            `geometry=${width}x${height}`);
        const stream = Gio.MemoryOutputStream.new_resizable();
        const pixbuf = await Shell.Screenshot.composite_to_stream(
            texture, 0, 0, width, height, this._scale,
            null, 0, 0, 1, stream);
        stream.close(null);
        log(`cosnip: pixbuf=${pixbuf.get_width()}x${pixbuf.get_height()}`);
        return pixbuf;
    }

    _buildUI() {
        const stageWidth = global.stage.width;
        const stageHeight = global.stage.height;
        this._root = new St.Widget({
            name: 'cosnip-overlay',
            reactive: true,
            can_focus: true,
            layout_manager: new Clutter.BinLayout(),
            width: stageWidth,
            height: stageHeight,
        });

        this._background = new St.Widget({
            content: this._content,
            width: stageWidth,
            height: stageHeight,
        });
        this._root.add_child(this._background);

        this._dim = new St.DrawingArea({
            width: stageWidth,
            height: stageHeight,
            reactive: false,
        });
        this._dim.connect('repaint', area => this._paintDim(area));
        this._root.add_child(this._dim);

        this._annotationsArea = new St.DrawingArea({
            width: stageWidth,
            height: stageHeight,
            reactive: false,
        });
        this._annotationsArea.connect('repaint', area => this._paintAnnotations(area));
        this._root.add_child(this._annotationsArea);

        this._toolbar = new Toolbar({
            onTool: tool => this._setTool(tool),
            onUndo: () => this._undoAction(),
            onRedo: () => this._redoAction(),
            onCancel: () => this._close(),
            onPin: () => this._pin(),
            onSave: () => this._save(),
            onCopy: () => this._copy(),
            onColor: color => {
                this._style.color = color;
            },
            onWidth: width => {
                this._style.width = width;
            },
        });
        this._toolbar.setStyle(this._style.color, this._style.width);
        this._toolbar.visible = false;
        this._root.add_child(this._toolbar);

        this._sizeBadge = new St.Label({
            style_class: 'cosnip-size-badge',
            visible: false,
        });
        this._root.add_child(this._sizeBadge);

        this._tips = new St.Label({
            style_class: 'cosnip-tips',
            visible: this._settings.get_boolean('show-tips'),
        });
        this._tips.clutter_text.set_markup(
            '<b>Drag</b> select   <b>Shift</b> square   <b>Enter</b> copy\n' +
            '<b>Ctrl+S</b> save   <b>P</b> pin   <b>Esc</b> cancel');
        this._root.add_child(this._tips);

        this._root.connect('button-press-event', (_actor, event) => this._onPress(event));
        this._root.connect('button-release-event', (_actor, event) => this._onRelease(event));
        this._root.connect('motion-event', (_actor, event) => this._onMotion(event));
        this._root.connect('key-press-event', (_actor, event) => this._onKey(event));
        this._root.connect('scroll-event', (_actor, event) => this._onScroll(event));
        this._root.connect('destroy', () => {
            if (this._panelBox)
                this._panelBox.visible = this._panelVisible ?? true;
        });

        Main.layoutManager.screenshotUIGroup.add_child(this._root);
        this._panelBox = Main.layoutManager.panelBox;
        this._panelVisible = this._panelBox.visible;
        this._panelBox.hide();
        this._root.show();
        this._grab = Main.pushModal(this._root, {
            actionMode: Shell.ActionMode.POPUP,
        });
        this._layoutChrome();
        this._repaint();
    }

    _paintDim(area) {
        const cr = area.get_context();
        const width = area.width;
        const height = area.height;
        cr.setSourceRGBA(0, 0, 0, 0.45);
        if (!this._selection) {
            cr.rectangle(0, 0, width, height);
            cr.fill();
            return;
        }
        const {x, y, w, h} = this._selection;
        cr.rectangle(0, 0, width, Math.max(0, y));
        cr.rectangle(0, y + h, width, Math.max(0, height - y - h));
        cr.rectangle(0, y, Math.max(0, x), h);
        cr.rectangle(x + w, y, Math.max(0, width - x - w), h);
        cr.fill();

        cr.setSourceRGBA(...SELECTION_COLOR);
        cr.setLineWidth(1.5);
        cr.rectangle(x, y, w, h);
        cr.stroke();
        for (const [hx, hy] of handlePoints(this._selection)) {
            cr.arc(hx, hy, 4.5, 0, Math.PI * 2);
            cr.setSourceRGBA(1, 1, 1, 1);
            cr.fillPreserve();
            cr.setSourceRGBA(...SELECTION_COLOR);
            cr.setLineWidth(1.2);
            cr.stroke();
        }
    }

    _paintAnnotations(area) {
        if (!this._selection)
            return;
        const cr = area.get_context();
        const {x, y, w, h} = this._selection;
        cr.save();
        cr.rectangle(x, y, w, h);
        cr.clip();
        drawAnnotations(cr, this._annotations, {
            sourcePixbuf: this._sourcePixbuf,
            deviceScale: this._scale,
        });
        if (this._draft) {
            drawAnnotations(cr, [this._draft], {
                sourcePixbuf: this._sourcePixbuf,
                deviceScale: this._scale,
            });
        }
        cr.restore();
    }

    _repaint() {
        this._dim.queue_repaint();
        this._annotationsArea.queue_repaint();
    }

    _layoutChrome() {
        if (!this._selection) {
            this._toolbar.visible = false;
            this._sizeBadge.visible = false;
            this._tips.visible = this._settings.get_boolean('show-tips');
            this._tips.set_position(12, global.stage.height - 58);
            return;
        }
        const {x, y, w, h} = this._selection;
        const [minWidth, naturalWidth] = this._toolbar.get_preferred_width(-1);
        const [minHeight, naturalHeight] = this._toolbar.get_preferred_height(naturalWidth);
        const toolbarWidth = Math.max(minWidth, naturalWidth);
        const toolbarHeight = Math.max(minHeight, naturalHeight);
        let tx = Math.round(x + w / 2 - toolbarWidth / 2);
        tx = Math.max(8, Math.min(global.stage.width - toolbarWidth - 8, tx));
        let ty = Math.round(y + h + 8);
        if (ty + toolbarHeight > global.stage.height - 8)
            ty = Math.round(y - toolbarHeight - 8);
        ty = Math.max(8, ty);
        this._toolbar.set_position(tx, ty);
        this._toolbar.visible = true;

        this._sizeBadge.set_text(`${Math.round(w)} x ${Math.round(h)} px`);
        this._sizeBadge.set_position(
            Math.round(x), Math.round(Math.max(4, y - 24)));
        this._sizeBadge.visible = true;

        const showTips = this._settings.get_boolean('show-tips');
        const tipsHeight = this._tips.get_preferred_height(-1)[1] || 48;
        const tipsTop = global.stage.height - tipsHeight - 12;
        const tipsBottom = tipsTop + tipsHeight;
        const tipsWidth = this._tips.get_preferred_width(-1)[1] || 250;
        const overlap = tx < 12 + tipsWidth && tx + toolbarWidth > 12 &&
            ty < tipsBottom && ty + toolbarHeight > tipsTop;
        this._tips.visible = showTips && !overlap;
        this._tips.set_position(12, Math.round(tipsTop));
    }

    _setTool(tool) {
        if (this._textEntry)
            this._commitText();
        this._tool = tool;
        this._toolbar.syncShape(tool);
        this._toolbar.setTool(tool);
    }

    _pointer(event) {
        const [x, y] = event.get_coords();
        return {x, y};
    }

    _sourceIsChrome(source) {
        let actor = source;
        while (actor) {
            if (actor === this._toolbar || actor === this._sizeBadge ||
                actor === this._tips || actor === this._textEntry?.entry)
                return true;
            actor = actor.get_parent();
        }
        return false;
    }

    _onPress(event) {
        if (this._sourceIsChrome(event.get_source()))
            return Clutter.EVENT_PROPAGATE;
        if (this._textEntry) {
            this._commitText();
            return Clutter.EVENT_STOP;
        }
        const button = event.get_button();
        if (button !== 1) {
            if (this._draft) {
                this._draft = null;
                this._repaint();
            } else if (this._tool !== Tool.SELECT) {
                this._setTool(Tool.SELECT);
            }
            return Clutter.EVENT_STOP;
        }
        const point = this._pointer(event);
        if (this._tool === Tool.SELECT) {
            if (this._selection) {
                const handle = hitHandle(this._selection, point);
                if (handle) {
                    this._drag = {
                        mode: 'resize',
                        handle,
                        start: {...this._selection},
                    };
                    return Clutter.EVENT_STOP;
                }
                if (contains(this._selection, point)) {
                    this._drag = {
                        mode: 'move',
                        start: {...this._selection},
                        origin: point,
                    };
                    return Clutter.EVENT_STOP;
                }
            }
            this._drag = {mode: 'new', anchor: point};
            this._selection = {x: point.x, y: point.y, w: 0, h: 0};
            this._layoutChrome();
            this._repaint();
            return Clutter.EVENT_STOP;
        }

        if (!this._selection || !contains(this._selection, point)) {
            this._drag = {mode: 'new', anchor: point};
            this._selection = {x: point.x, y: point.y, w: 0, h: 0};
            this._layoutChrome();
            this._repaint();
            return Clutter.EVENT_STOP;
        }

        if (this._tool === Tool.TEXT) {
            this._createTextEntry(point);
            return Clutter.EVENT_STOP;
        }

        this._draft = createAnnotation(this._tool, point, this._style);
        this._drag = {mode: 'annotation'};
        this._repaint();
        return Clutter.EVENT_STOP;
    }

    _onMotion(event) {
        if (!this._drag)
            return Clutter.EVENT_PROPAGATE;
        const point = this._pointer(event);
        if (this._drag.mode === 'new') {
            const shift = Boolean(event.get_state() & Clutter.ModifierType.SHIFT_MASK);
            this._selection = rectFromPoints(this._drag.anchor, point, shift);
            this._layoutChrome();
        } else if (this._drag.mode === 'move') {
            const dx = point.x - this._drag.origin.x;
            const dy = point.y - this._drag.origin.y;
            this._selection = clampSelection({
                x: this._drag.start.x + dx,
                y: this._drag.start.y + dy,
                w: this._drag.start.w,
                h: this._drag.start.h,
            });
            this._layoutChrome();
        } else if (this._drag.mode === 'resize') {
            this._selection = resizeSelection(
                this._drag.start, this._drag.handle, point);
            this._layoutChrome();
        } else if (this._drag.mode === 'annotation' && this._draft) {
            const shift = Boolean(event.get_state() & Clutter.ModifierType.SHIFT_MASK);
            updateAnnotation(this._draft, point, shift);
        }
        this._repaint();
        return Clutter.EVENT_STOP;
    }

    _onRelease(event) {
        if (!this._drag)
            return Clutter.EVENT_PROPAGATE;
        const mode = this._drag.mode;
        this._drag = null;
        if (mode === 'annotation' && this._draft) {
            if (annotationIsValid(this._draft))
                this._addAnnotation(this._draft);
            this._draft = null;
            this._repaint();
        } else if (mode === 'new') {
            if (this._selection && (this._selection.w < 3 || this._selection.h < 3)) {
                this._selection = null;
                this._layoutChrome();
            }
            this._repaint();
        }
        return Clutter.EVENT_STOP;
    }

    _onScroll(event) {
        if (!this._selection)
            return Clutter.EVENT_PROPAGATE;
        const [dx, dy] = event.get_scroll_delta();
        const delta = dy !== 0 ? dy : -dx;
        if (delta === 0)
            return Clutter.EVENT_PROPAGATE;
        this._style.width = Math.max(1, Math.min(32,
            this._style.width + Math.sign(delta)));
        this._toolbar.setStyle(this._style.color, this._style.width);
        return Clutter.EVENT_STOP;
    }

    _onKey(event) {
        const symbol = event.get_key_symbol();
        const state = event.get_state();
        const ctrl = Boolean(state & Clutter.ModifierType.CONTROL_MASK);
        const shift = Boolean(state & Clutter.ModifierType.SHIFT_MASK);
        if (this._textEntry)
            return Clutter.EVENT_PROPAGATE;
        if (symbol === Clutter.KEY_Escape) {
            this._escape();
            return Clutter.EVENT_STOP;
        }
        if (symbol === Clutter.KEY_Return || symbol === Clutter.KEY_KP_Enter) {
            this._defaultOutput();
            return Clutter.EVENT_STOP;
        }
        if (ctrl && symbol === Clutter.KEY_c) {
            this._copy();
            return Clutter.EVENT_STOP;
        }
        if (ctrl && symbol === Clutter.KEY_s) {
            this._save();
            return Clutter.EVENT_STOP;
        }
        if (ctrl && symbol === Clutter.KEY_z && !shift) {
            this._undoAction();
            return Clutter.EVENT_STOP;
        }
        if (ctrl && (symbol === Clutter.KEY_y || (shift && symbol === Clutter.KEY_z))) {
            this._redoAction();
            return Clutter.EVENT_STOP;
        }
        if (ctrl && symbol === Clutter.KEY_a) {
            this._selection = {x: 0, y: 0, w: global.stage.width, h: global.stage.height};
            this._layoutChrome();
            this._repaint();
            return Clutter.EVENT_STOP;
        }
        if (!ctrl) {
            const tool = toolForKey(symbol);
            if (tool) {
                this._setTool(tool);
                return Clutter.EVENT_STOP;
            }
        }
        if (symbol === Clutter.KEY_p && !ctrl) {
            this._pin();
            return Clutter.EVENT_STOP;
        }
        if (this._selection &&
            (symbol === Clutter.KEY_Left || symbol === Clutter.KEY_Right ||
             symbol === Clutter.KEY_Up || symbol === Clutter.KEY_Down)) {
            const step = shift ? 10 : 1;
            const dx = symbol === Clutter.KEY_Left ? -step :
                symbol === Clutter.KEY_Right ? step : 0;
            const dy = symbol === Clutter.KEY_Up ? -step :
                symbol === Clutter.KEY_Down ? step : 0;
            this._selection = clampSelection({
                ...this._selection,
                x: this._selection.x + dx,
                y: this._selection.y + dy,
            });
            this._layoutChrome();
            this._repaint();
            return Clutter.EVENT_STOP;
        }
        return Clutter.EVENT_PROPAGATE;
    }

    _escape() {
        if (this._draft) {
            this._draft = null;
            this._repaint();
            return;
        }
        if (this._tool !== Tool.SELECT) {
            this._setTool(Tool.SELECT);
            return;
        }
        this._close();
    }

    _addAnnotation(annotation) {
        this._annotations.push(annotation);
        this._undo.push(annotation);
        this._redo = [];
        this._syncUndo();
    }

    _undoAction() {
        if (this._draft) {
            this._draft = null;
            this._repaint();
            return;
        }
        const annotation = this._undo.pop();
        if (!annotation)
            return;
        const index = this._annotations.lastIndexOf(annotation);
        if (index >= 0)
            this._annotations.splice(index, 1);
        this._redo.push(annotation);
        this._syncUndo();
        this._repaint();
    }

    _redoAction() {
        const annotation = this._redo.pop();
        if (!annotation)
            return;
        this._annotations.push(annotation);
        this._undo.push(annotation);
        this._syncUndo();
        this._repaint();
    }

    _syncUndo() {
        this._toolbar.setUndo(this._undo.length > 0);
        this._toolbar.setRedo(this._redo.length > 0);
    }

    _createTextEntry(point) {
        const entry = new St.Entry({
            style_class: 'cosnip-text-entry',
            can_focus: true,
            reactive: true,
            width: Math.min(360, Math.max(120, this._selection.w - (point.x - this._selection.x))),
        });
        entry.set_style(`font-size: ${this._style.fontSize}px;`);
        entry.set_position(Math.round(point.x), Math.round(point.y));
        this._root.add_child(entry);
        this._textEntry = {entry, point};
        entry.grab_key_focus();
        entry.connect('key-press-event', (_text, event) => {
            const symbol = event.get_key_symbol();
            if (symbol === Clutter.KEY_Return || symbol === Clutter.KEY_KP_Enter) {
                this._commitText();
                return Clutter.EVENT_STOP;
            }
            if (symbol === Clutter.KEY_Escape) {
                this._cancelText();
                return Clutter.EVENT_STOP;
            }
            return Clutter.EVENT_PROPAGATE;
        });
    }

    _commitText() {
        if (!this._textEntry)
            return;
        const {entry, point} = this._textEntry;
        const text = entry.get_text().trim();
        this._textEntry = null;
        entry.destroy();
        if (text) {
            this._addAnnotation({
                kind: Tool.TEXT,
                x: point.x,
                y: point.y,
                text,
                size: this._style.fontSize,
                color: this._style.color,
                width: this._style.width,
            });
            this._repaint();
        }
        this._root.grab_key_focus();
    }

    _cancelText() {
        if (!this._textEntry)
            return;
        this._textEntry.entry.destroy();
        this._textEntry = null;
        this._root.grab_key_focus();
    }

    _renderSelection() {
        if (!this._selection)
            return null;
        const sel = {
            x: Math.round(this._selection.x),
            y: Math.round(this._selection.y),
            w: Math.max(1, Math.round(this._selection.w)),
            h: Math.max(1, Math.round(this._selection.h)),
        };
        const deviceW = Math.max(1, Math.round(sel.w * this._scale));
        const deviceH = Math.max(1, Math.round(sel.h * this._scale));
        const surface = new cairo.ImageSurface(cairo.Format.ARGB32, deviceW, deviceH);
        const cr = new cairo.Context(surface);
        if (this._sourcePixbuf) {
            paintPixbuf(cr, this._sourcePixbuf,
                -sel.x * this._scale, -sel.y * this._scale, 1);
        } else {
            cr.setSourceRGBA(0, 0, 0, 1);
            cr.paint();
        }
        cr.save();
        cr.scale(this._scale, this._scale);
        cr.translate(-sel.x, -sel.y);
        drawAnnotations(cr, this._annotations, {
            sourcePixbuf: this._sourcePixbuf,
            deviceScale: this._scale,
        });
        cr.restore();
        return surface;
    }

    _defaultOutput() {
        switch (this._settings.get_enum('default-output')) {
        case 1:
            this._save();
            break;
        case 2:
            this._pin();
            break;
        default:
            this._copy();
            break;
        }
    }

    _copy() {
        const surface = this._renderSelection();
        if (!surface)
            return;
        copySurfaceToClipboard(surface);
        this._notify('Copied selection');
        this._close();
    }

    _save() {
        const surface = this._renderSelection();
        if (!surface)
            return;
        const dirSetting = this._settings.get_string('save-dir').trim();
        const dir = dirSetting || defaultSaveDir();
        const now = GLib.DateTime.new_now_local();
        const name = expandFilename(this._settings.get_string('filename'), now);
        const path = uniquePath(dir, name);
        try {
            saveSurface(surface, path);
            this._notify(`Saved ${path}`);
        } catch (error) {
            logError(error, 'cosnip: save failed');
            this._notify('Save failed; see log');
            return;
        }
        this._close();
    }

    _pin() {
        const surface = this._renderSelection();
        if (!surface || !this._selection)
            return;
        this._onPin?.(
            surface,
            Math.round(this._selection.x),
            Math.round(this._selection.y),
            Math.max(1, Math.round(this._selection.w)),
            Math.max(1, Math.round(this._selection.h)));
        this._close();
    }

    _notify(message) {
        try {
            Main.notify('Cosnip', message);
        } catch (_error) {
            log(`cosnip: ${message}`);
        }
    }

    close() {
        this._close();
    }

    _close() {
        if (this._closed)
            return;
        this._closed = true;
        this._cancelText();
        if (this._grab) {
            Main.popModal(this._grab);
            this._grab = null;
        }
        if (this._panelBox)
            this._panelBox.visible = this._panelVisible ?? true;
        this._root?.destroy();
        this._root = null;
        this._onClose?.();
    }
}

function contains(rect, point) {
    return point.x >= rect.x && point.y >= rect.y &&
        point.x < rect.x + rect.w && point.y < rect.y + rect.h;
}

function rectFromPoints(a, b, square) {
    let x = Math.min(a.x, b.x);
    let y = Math.min(a.y, b.y);
    let w = Math.abs(b.x - a.x);
    let h = Math.abs(b.y - a.y);
    if (square) {
        const side = Math.max(w, h);
        if (b.x < a.x)
            x = a.x - side;
        if (b.y < a.y)
            y = a.y - side;
        w = side;
        h = side;
    }
    return clampSelection({x, y, w, h});
}

function clampSelection(rect) {
    const x = Math.max(0, Math.min(global.stage.width, rect.x));
    const y = Math.max(0, Math.min(global.stage.height, rect.y));
    const w = Math.max(0, Math.min(global.stage.width - x, rect.w));
    const h = Math.max(0, Math.min(global.stage.height - y, rect.h));
    return {x, y, w, h};
}

function handlePoints(rect) {
    const x1 = rect.x;
    const y1 = rect.y;
    const x2 = rect.x + rect.w;
    const y2 = rect.y + rect.h;
    return [
        [x1, y1], [x2, y1], [x1, y2], [x2, y2],
        [(x1 + x2) / 2, y1], [(x1 + x2) / 2, y2],
        [x1, (y1 + y2) / 2], [x2, (y1 + y2) / 2],
    ];
}

function hitHandle(rect, point) {
    const positions = [
        'nw', 'ne', 'sw', 'se', 'n', 's', 'w', 'e',
    ];
    const points = handlePoints(rect);
    for (let i = 0; i < points.length; i++) {
        const [x, y] = points[i];
        if (Math.hypot(point.x - x, point.y - y) <= HANDLE_RADIUS)
            return positions[i];
    }
    return null;
}

function resizeSelection(start, handle, point) {
    let left = start.x;
    let top = start.y;
    let right = start.x + start.w;
    let bottom = start.y + start.h;
    if (handle.includes('w'))
        left = point.x;
    if (handle.includes('e'))
        right = point.x;
    if (handle.includes('n'))
        top = point.y;
    if (handle.includes('s'))
        bottom = point.y;
    if (left > right)
        [left, right] = [right, left];
    if (top > bottom)
        [top, bottom] = [bottom, top];
    return clampSelection({
        x: left,
        y: top,
        w: Math.max(1, right - left),
        h: Math.max(1, bottom - top),
    });
}

function toolForKey(symbol) {
    switch (symbol) {
    case Clutter.KEY_r: return Tool.RECT;
    case Clutter.KEY_c: return Tool.ELLIPSE;
    case Clutter.KEY_a: return Tool.ARROW;
    case Clutter.KEY_b: return Tool.PENCIL;
    case Clutter.KEY_h: return Tool.MARKER;
    case Clutter.KEY_m: return Tool.MOSAIC;
    case Clutter.KEY_t: return Tool.TEXT;
    case Clutter.KEY_e: return Tool.ERASER;
    case Clutter.KEY_v: return Tool.SELECT;
    default: return null;
    }
}
